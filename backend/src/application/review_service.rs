//! 集計・振り返りのユースケース（F-08〜F-10、F-13〜F-17、CSV 出力）。

use std::collections::HashMap;

use chrono::Duration;
use sqlx::SqlitePool;
use tracing::info;

use super::common::ensure_exam;
use crate::{
    domain::{
        error::{AppError, AppResult},
        models::{AreaStat, Attempt, DailyCount, Dashboard, QuestionItem},
        time::{today_jst, utc_string_to_jst_string},
        validation::{validate_correct_answer, validate_note, validate_question_number, validate_title},
    },
    infrastructure::{
        database::begin_write,
        repositories::{master_repository, practice_repository as repo},
    },
};

/// 日別の推移で表示する日数（今日を含む）。
const DAILY_DAYS: i64 = 14;

/// 問題一覧の絞り込み条件。
#[derive(Debug, Clone)]
pub enum QuestionFilter {
    /// すべての問題（F-13）。
    All,
    /// 最新の結果が不正解の問題だけ（F-14 復習一覧）。
    Review,
    /// 最新の回答のカテゴリが一致する問題だけ（F-15）。
    Category(i64),
}

/// 集計・振り返りのユースケース。
#[derive(Clone)]
pub struct ReviewService {
    pool: SqlitePool,
}

impl ReviewService {
    /// サービスを作る。
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// ダッシュボードの集計を返す（F-08〜F-10）。集計対象は採点済みの回答だけ。
    pub async fn dashboard(&self, user_id: i64, exam_id: i64) -> AppResult<Dashboard> {
        let mut conn = self.pool.acquire().await?;
        ensure_exam(&mut conn, user_id, exam_id).await?;
        let (total_count, correct_count, average_seconds) = repo::totals(&mut conn, exam_id).await?;

        // 回答がない日も 0 件として出力する。
        let today = today_jst();
        let since = today - Duration::days(DAILY_DAYS - 1);
        let counts: HashMap<String, (i64, i64)> =
            repo::daily_counts_since(&mut conn, exam_id, &since.format("%Y-%m-%d").to_string())
                .await?
                .into_iter()
                .map(|(day, count, correct)| (day, (count, correct)))
                .collect();
        let daily_counts = (0..DAILY_DAYS)
            .map(|offset| {
                let date = (since + Duration::days(offset)).format("%Y-%m-%d").to_string();
                let (count, correct_count) = counts.get(&date).copied().unwrap_or((0, 0));
                DailyCount {
                    date,
                    count,
                    correct_count,
                }
            })
            .collect();

        // カテゴリマスタの並び順で並べ、回答が無いカテゴリは 0/0 とする。
        // マスタに無いカテゴリ名の回答（通常は起動時にマスタへ登録される）は末尾に付ける。
        let mut area_counts: HashMap<String, (i64, i64)> = repo::area_counts(&mut conn, exam_id)
            .await?
            .into_iter()
            .map(|(area, total, correct)| (area, (total, correct)))
            .collect();
        let mut area_stats: Vec<AreaStat> = master_repository::list_categories(&mut conn, exam_id)
            .await?
            .into_iter()
            .map(|category| {
                let (total_count, correct_count) =
                    area_counts.remove(&category.name).unwrap_or((0, 0));
                AreaStat {
                    area: category.name,
                    total_count,
                    correct_count,
                    in_master: true,
                }
            })
            .collect();
        let mut orphans: Vec<AreaStat> = area_counts
            .into_iter()
            .map(|(area, (total_count, correct_count))| AreaStat {
                area,
                total_count,
                correct_count,
                in_master: false,
            })
            .collect();
        orphans.sort_by(|a, b| a.area.cmp(&b.area));
        area_stats.extend(orphans);

        Ok(Dashboard {
            exam_id,
            total_count,
            correct_count,
            average_seconds,
            daily_counts,
            area_stats,
        })
    }

    /// 問題一覧を返す（F-13〜F-15）。
    pub async fn questions(
        &self,
        user_id: i64,
        exam_id: i64,
        filter: QuestionFilter,
    ) -> AppResult<Vec<QuestionItem>> {
        let mut conn = self.pool.acquire().await?;
        ensure_exam(&mut conn, user_id, exam_id).await?;
        let items = repo::list_questions(&mut conn, exam_id).await?;
        let items = match filter {
            QuestionFilter::All => items,
            QuestionFilter::Review => items.into_iter().filter(|q| !q.correct).collect(),
            QuestionFilter::Category(category_id) => {
                let name = master_repository::find_category_name(&mut conn, exam_id, category_id)
                    .await?
                    .ok_or_else(|| AppError::NotFound("カテゴリが見つかりません。".into()))?;
                items.into_iter().filter(|q| q.area == name).collect()
            }
        };
        Ok(items)
    }

    /// 問題ごとの正誤履歴を返す（F-16）。
    pub async fn history(
        &self,
        user_id: i64,
        exam_id: i64,
        title: &str,
        question_number: i64,
    ) -> AppResult<Vec<Attempt>> {
        let title = validate_title(title)?;
        let question_number = validate_question_number(question_number)?;
        let mut conn = self.pool.acquire().await?;
        ensure_exam(&mut conn, user_id, exam_id).await?;
        Ok(repo::list_attempts(&mut conn, exam_id, &title, question_number).await?)
    }

    /// 採点済みの回答のカテゴリを変更する（F-17）。
    pub async fn change_answer_category(
        &self,
        user_id: i64,
        answer_id: i64,
        category_id: i64,
    ) -> AppResult<()> {
        let mut tx = begin_write(&self.pool).await?;
        let owner = find_graded_answer(&mut tx, user_id, answer_id).await?;
        let name = master_repository::find_category_name(&mut tx, owner.exam_id, category_id)
            .await?
            .ok_or_else(|| AppError::Validation("カテゴリを選択してください。".into()))?;
        repo::update_area(&mut tx, answer_id, &name).await?;
        tx.commit().await?;
        info!(user_id, answer_id, category_id, "回答のカテゴリを変更しました");
        Ok(())
    }

    /// 採点済みの回答の「正解」「メモ」を更新する。
    pub async fn update_answer_memo(
        &self,
        user_id: i64,
        answer_id: i64,
        correct_answer: Option<&str>,
        note: Option<&str>,
    ) -> AppResult<()> {
        let correct_answer = validate_correct_answer(correct_answer)?;
        let note = validate_note(note)?;
        let mut tx = begin_write(&self.pool).await?;
        find_graded_answer(&mut tx, user_id, answer_id).await?;
        repo::update_memo(&mut tx, answer_id, correct_answer.as_deref(), note.as_deref()).await?;
        tx.commit().await?;
        info!(user_id, answer_id, "回答のメモを更新しました");
        Ok(())
    }

    /// 試験の採点済み回答を CSV（UTF-8 BOM 付き、Excel で開ける形式）で返す。
    ///
    /// 戻り値は（ファイル名に使う試験名, CSV 本文）。
    pub async fn export_csv(&self, user_id: i64, exam_id: i64) -> AppResult<(String, String)> {
        let mut conn = self.pool.acquire().await?;
        let exam_name = ensure_exam(&mut conn, user_id, exam_id).await?;
        let rows = repo::export_rows(&mut conn, exam_id).await?;
        let mut csv = String::from("\u{feff}");
        csv.push_str("学習ID,学習開始日時,問題名称,問題数,カテゴリ,回答,回答時間(秒),回答日時,正誤,正解,メモ\r\n");
        for row in &rows {
            let fields = [
                row.practice_id.to_string(),
                row.practice_created_at.clone(),
                row.title.clone(),
                row.question_number.to_string(),
                row.area.clone(),
                row.response.clone(),
                row.elapsed_seconds.to_string(),
                utc_string_to_jst_string(&row.answered_at),
                if row.correct { "正解" } else { "不正解" }.to_string(),
                row.correct_answer.clone().unwrap_or_default(),
                row.note.clone().unwrap_or_default(),
            ];
            let line: Vec<String> = fields.iter().map(|f| csv_field(f)).collect();
            csv.push_str(&line.join(","));
            csv.push_str("\r\n");
        }
        info!(user_id, exam_id, rows = rows.len(), "CSV を出力しました");
        Ok((exam_name, csv))
    }
}

/// 採点済みの回答であることを確かめる（未採点・存在しない回答はエラー）。
async fn find_graded_answer(
    conn: &mut sqlx::SqliteConnection,
    user_id: i64,
    answer_id: i64,
) -> AppResult<repo::AnswerOwner> {
    let owner = repo::find_answer_owner(conn, user_id, answer_id)
        .await?
        .ok_or_else(|| AppError::NotFound("回答が見つかりません。".into()))?;
    if owner.correct.is_none() {
        return Err(AppError::Conflict("採点済みの回答だけ変更できます。".into()));
    }
    Ok(owner)
}

/// CSV の 1 項目を必要に応じて二重引用符で囲む（RFC 4180）。
///
/// 表計算ソフトで数式として解釈されないよう、`=` `+` `-` `@` で始まる値は先頭に `'` を付ける。
fn csv_field(value: &str) -> String {
    let value = if value.starts_with(['=', '+', '-', '@']) {
        format!("'{value}")
    } else {
        value.to_string()
    };
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::csv_field;

    #[test]
    fn csv_field_escapes_special_characters() {
        assert_eq!(csv_field("abc"), "abc");
        assert_eq!(csv_field("a,b"), "\"a,b\"");
        assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_field("1行\n2行"), "\"1行\n2行\"");
        assert_eq!(csv_field("=SUM(A1)"), "'=SUM(A1)");
    }
}
