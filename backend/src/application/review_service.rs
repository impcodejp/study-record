//! 集計・振り返りのユースケース（F-08〜F-10、F-13〜F-17、CSV 出力、今日の復習・学習目標）。

use std::collections::{BTreeSet, HashMap};

use chrono::{Duration, NaiveDate};
use sqlx::SqlitePool;
use tracing::info;

use super::common::ensure_exam;
use crate::{
    domain::{
        error::{AppError, AppResult},
        models::{
            AreaReadiness, AreaStat, Attempt, DailyCount, Dashboard, NoteItem, QuestionItem, Readiness,
            ReadinessPoint, READINESS_WEEKS,
            TodayPlan, HEATMAP_DAYS,
        },
        study_plan::{
            days_until, mastery_level, next_review_date, readiness_percent, remaining_correct_answers,
            required_daily_pace, study_streak, MAX_MASTERY_LEVEL,
        },
        time::{today_jst, utc_string_to_jst_date, utc_string_to_jst_string},
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
    /// 忘却曲線にもとづく復習日が今日までに来ている問題だけ（今日の復習）。
    Due,
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

    /// ダッシュボードの集計を返す（F-08〜F-10、今日やること、学習カレンダー）。
    /// 集計対象は採点済みの回答だけ。
    pub async fn dashboard(&self, user_id: i64, exam_id: i64) -> AppResult<Dashboard> {
        let mut conn = self.pool.acquire().await?;
        ensure_exam(&mut conn, user_id, exam_id).await?;
        let (total_count, correct_count, average_seconds) = repo::totals(&mut conn, exam_id).await?;

        // 学習カレンダーの期間の日別件数。回答がない日も 0 件として出力する。
        let today = today_jst();
        let since = today - Duration::days(HEATMAP_DAYS - 1);
        let counts: HashMap<String, (i64, i64)> =
            repo::daily_counts_since(&mut conn, exam_id, &format_date(since))
                .await?
                .into_iter()
                .map(|(day, count, correct)| (day, (count, correct)))
                .collect();
        let heatmap: Vec<DailyCount> = (0..HEATMAP_DAYS)
            .map(|offset| {
                let date = format_date(since + Duration::days(offset));
                let (count, correct_count) = counts.get(&date).copied().unwrap_or((0, 0));
                DailyCount {
                    date,
                    count,
                    correct_count,
                }
            })
            .collect();
        // 日別の推移のグラフは、そのうち直近の日だけを使う。
        let daily_counts = heatmap[heatmap.len() - DAILY_DAYS as usize..].to_vec();
        let today_counts = heatmap.last().cloned().unwrap_or(DailyCount {
            date: format_date(today),
            count: 0,
            correct_count: 0,
        });

        // 連続学習日数
        let study_days: BTreeSet<NaiveDate> = repo::study_days(&mut conn, exam_id)
            .await?
            .iter()
            .filter_map(|day| NaiveDate::parse_from_str(day, "%Y-%m-%d").ok())
            .collect();
        let streak = study_streak(&study_days, today);

        // 復習の時期が来ている問題・習得済みの問題
        let questions = self.scheduled_questions(&mut conn, exam_id, today).await?;
        let due_count = questions.iter().filter(|q| is_due(q, today)).count() as i64;
        let mastered_count = questions
            .iter()
            .filter(|q| q.mastery_level >= MAX_MASTERY_LEVEL)
            .count() as i64;

        // 学習目標
        let (exam_date, daily_goal) = master_repository::find_exam_goal(&mut conn, exam_id)
            .await?
            .unwrap_or((None, None));
        let days_until_exam = exam_date
            .as_deref()
            .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
            .map(|d| days_until(d, today));
        let today_plan = TodayPlan {
            date: format_date(today),
            exam_date,
            days_until_exam,
            daily_goal,
            answered_count: today_counts.count,
            correct_count: today_counts.correct_count,
            current_streak: streak.current,
            longest_streak: streak.longest,
            due_count,
            question_count: questions.len() as i64,
            mastered_count,
        };

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

        // 準備度とペース（直近の平均は日別の推移と同じ期間で求める）
        let recent_total: i64 = daily_counts.iter().map(|d| d.count).sum();
        let area_order: Vec<String> = area_stats.iter().map(|s| s.area.clone()).collect();
        let results = repo::list_results(&mut conn, exam_id).await?;
        let readiness = build_readiness(
            &questions,
            &area_order,
            recent_total as f64 / DAILY_DAYS as f64,
            days_until_exam,
            readiness_history(&results, today),
        );

        Ok(Dashboard {
            exam_id,
            total_count,
            correct_count,
            average_seconds,
            daily_counts,
            area_stats,
            today: today_plan,
            heatmap,
            readiness,
        })
    }

    /// 見直しノート（正解・メモを残した採点済みの回答。新しい順）を返す。
    pub async fn notes(&self, user_id: i64, exam_id: i64) -> AppResult<Vec<NoteItem>> {
        let mut conn = self.pool.acquire().await?;
        ensure_exam(&mut conn, user_id, exam_id).await?;
        Ok(repo::list_notes(&mut conn, exam_id).await?)
    }

    /// 問題一覧を返す（F-13〜F-15、今日の復習）。
    pub async fn questions(
        &self,
        user_id: i64,
        exam_id: i64,
        filter: QuestionFilter,
    ) -> AppResult<Vec<QuestionItem>> {
        let mut conn = self.pool.acquire().await?;
        ensure_exam(&mut conn, user_id, exam_id).await?;
        let today = today_jst();
        let items = self.scheduled_questions(&mut conn, exam_id, today).await?;
        let items = match filter {
            QuestionFilter::All => items,
            QuestionFilter::Review => items.into_iter().filter(|q| !q.correct).collect(),
            QuestionFilter::Due => {
                // 復習日を過ぎている日数が長い問題（忘れかけている問題）から順に並べる。
                let mut due: Vec<QuestionItem> = items.into_iter().filter(|q| is_due(q, today)).collect();
                due.sort_by(|a, b| {
                    (a.next_review_on.as_str(), a.mastery_level, a.answered_at.as_str())
                        .cmp(&(b.next_review_on.as_str(), b.mastery_level, b.answered_at.as_str()))
                });
                due
            }
            QuestionFilter::Category(category_id) => {
                let name = master_repository::find_category_name(&mut conn, exam_id, category_id)
                    .await?
                    .ok_or_else(|| AppError::NotFound("カテゴリが見つかりません。".into()))?;
                items.into_iter().filter(|q| q.area == name).collect()
            }
        };
        Ok(items)
    }

    /// 試験の問題一覧に、正誤の履歴から求めた習熟度と次の復習日を付けて返す（最終回答が新しい順）。
    async fn scheduled_questions(
        &self,
        conn: &mut sqlx::SqliteConnection,
        exam_id: i64,
        today: NaiveDate,
    ) -> AppResult<Vec<QuestionItem>> {
        let records = repo::list_questions(conn, exam_id).await?;
        let results = repo::list_results(conn, exam_id).await?;
        Ok(build_question_items(records, &results, today))
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

/// 問題一覧の行に、習熟度と次の復習日を付ける。
///
/// `results` は問題ごとに記録順（古い順）に並んだ正誤。最終回答日を解析できない行は今日に解いたものとみなす。
fn build_question_items(
    records: Vec<repo::QuestionRecord>,
    results: &[repo::ResultRecord],
    today: NaiveDate,
) -> Vec<QuestionItem> {
    let mut histories: HashMap<(&str, i64), Vec<bool>> = HashMap::new();
    for result in results {
        histories
            .entry((result.title.as_str(), result.question_number))
            .or_default()
            .push(result.correct);
    }
    records
        .into_iter()
        .map(|record| {
            let level = histories
                .get(&(record.title.as_str(), record.question_number))
                .map_or(0, |history| mastery_level(history));
            let last_day = utc_string_to_jst_date(&record.answered_at).unwrap_or(today);
            QuestionItem {
                next_review_on: format_date(next_review_date(last_day, level)),
                mastery_level: level,
                answered_at: utc_string_to_jst_string(&record.answered_at),
                title: record.title,
                question_number: record.question_number,
                area: record.area,
                response: record.response,
                correct: record.correct,
                attempt_count: record.attempt_count,
                correct_count: record.correct_count,
                average_seconds: record.average_seconds,
            }
        })
        .collect()
}

/// 準備度の推移（[`READINESS_WEEKS`] 週分、1 週おき。最後が今日）を求める。
///
/// 各時点について、その日（日本時間）までの回答だけで習熟度を求め直す。
/// `results` は問題ごとに記録順に並んでいること。
fn readiness_history(results: &[repo::ResultRecord], today: NaiveDate) -> Vec<ReadinessPoint> {
    // 回答日（日本時間）を先に求めておく。解析できない日時は今日とみなす。
    let dated: Vec<(&repo::ResultRecord, NaiveDate)> = results
        .iter()
        .map(|r| (r, utc_string_to_jst_date(&r.answered_at).unwrap_or(today)))
        .collect();
    (0..READINESS_WEEKS)
        .rev()
        .map(|weeks_ago| {
            let cutoff = today - Duration::weeks(weeks_ago);
            let mut histories: HashMap<(&str, i64), Vec<bool>> = HashMap::new();
            for (result, day) in &dated {
                if *day <= cutoff {
                    histories
                        .entry((result.title.as_str(), result.question_number))
                        .or_default()
                        .push(result.correct);
                }
            }
            let levels: Vec<u8> = histories.values().map(|h| mastery_level(h)).collect();
            ReadinessPoint {
                date: format_date(cutoff),
                percent: readiness_percent(&levels),
                question_count: levels.len() as i64,
            }
        })
        .collect()
}

/// 準備度とペースの目安を求める。
///
/// - `area_order`：カテゴリマスタの並び順（カテゴリ別の準備度をこの順に並べる。マスタに無いものは末尾）
/// - `recent_daily_average`：直近の 1 日あたりの平均解答数
fn build_readiness(
    questions: &[QuestionItem],
    area_order: &[String],
    recent_daily_average: f64,
    days_until_exam: Option<i64>,
    history: Vec<ReadinessPoint>,
) -> Readiness {
    let levels: Vec<u8> = questions.iter().map(|q| q.mastery_level).collect();
    let remaining = remaining_correct_answers(&levels);

    // カテゴリごとに習熟度を集める（並びはマスタ順、マスタに無いカテゴリは名前順で末尾）。
    let mut by_area: HashMap<&str, Vec<u8>> = HashMap::new();
    for q in questions {
        by_area.entry(q.area.as_str()).or_default().push(q.mastery_level);
    }
    let mut names: Vec<&str> = area_order
        .iter()
        .map(String::as_str)
        .filter(|name| by_area.contains_key(name))
        .collect();
    let mut others: Vec<&str> = by_area
        .keys()
        .copied()
        .filter(|name| !area_order.iter().any(|o| o == name))
        .collect();
    others.sort_unstable();
    names.extend(others);
    let areas = names
        .into_iter()
        .map(|name| {
            let area_levels = &by_area[name];
            AreaReadiness {
                area: name.to_string(),
                percent: readiness_percent(area_levels),
                question_count: area_levels.len() as i64,
                mastered_count: area_levels.iter().filter(|l| **l >= MAX_MASTERY_LEVEL).count() as i64,
            }
        })
        .collect();

    Readiness {
        percent: readiness_percent(&levels),
        areas,
        recent_daily_average,
        remaining_correct_answers: remaining,
        required_daily: days_until_exam.and_then(|days| required_daily_pace(remaining, days)),
        history,
    }
}

/// 今日までに復習の時期が来ているか。
fn is_due(item: &QuestionItem, today: NaiveDate) -> bool {
    item.next_review_on.as_str() <= format_date(today).as_str()
}

/// 日付を `YYYY-MM-DD` にする。
fn format_date(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
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
    use super::*;

    fn record(title: &str, number: i64, answered_at: &str, correct: bool) -> repo::QuestionRecord {
        repo::QuestionRecord {
            title: title.into(),
            question_number: number,
            area: "未分類".into(),
            response: "ア".into(),
            answered_at: answered_at.into(),
            correct,
            attempt_count: 1,
            correct_count: i64::from(correct),
            average_seconds: 60.0,
        }
    }

    fn result(title: &str, number: i64, correct: bool) -> repo::ResultRecord {
        repo::ResultRecord {
            title: title.into(),
            question_number: number,
            correct,
            answered_at: "2026-10-01 00:00:00".into(),
        }
    }

    #[test]
    fn readiness_history_replays_answers_up_to_each_week() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 10).unwrap();
        let at = |title: &str, n: i64, correct: bool, utc: &str| repo::ResultRecord {
            title: title.into(),
            question_number: n,
            correct,
            answered_at: utc.into(),
        };
        let results = vec![
            // 問1：3 週間前に不正解 → 今日正解
            at("過去問", 1, false, "2026-09-19 00:00:00"),
            at("過去問", 1, true, "2026-10-10 00:00:00"),
            // 問2：1 週間前に正解
            at("過去問", 2, true, "2026-10-03 00:00:00"),
        ];
        let history = readiness_history(&results, today);
        assert_eq!(history.len(), READINESS_WEEKS as usize);
        assert_eq!(history.last().unwrap().date, "2026-10-10");
        // 4 週間前：まだ何も解いていない
        assert_eq!(history[READINESS_WEEKS as usize - 5].question_count, 0);
        // 3 週間前：問1 が習熟度 0 → 0%
        let three_weeks = &history[READINESS_WEEKS as usize - 4];
        assert_eq!((three_weeks.question_count, three_weeks.percent), (1, 0.0));
        // 1 週間前：問1=0, 問2=1 → 10%
        assert!((history[READINESS_WEEKS as usize - 2].percent - 10.0).abs() < 1e-9);
        // 今日：問1=1, 問2=1 → 20%
        assert!((history.last().unwrap().percent - 20.0).abs() < 1e-9);
    }

    #[test]
    fn readiness_is_grouped_in_master_order() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 10).unwrap();
        let mut a = record("過去問", 1, "2026-10-01 00:00:00", true);
        a.area = "B".into();
        let mut b = record("過去問", 2, "2026-10-01 00:00:00", true);
        b.area = "A".into();
        let mut c = record("過去問", 3, "2026-10-01 00:00:00", false);
        c.area = "マスタ外".into();
        let results = vec![
            result("過去問", 1, true),
            result("過去問", 2, true),
            result("過去問", 2, true),
            result("過去問", 2, true),
            result("過去問", 2, true),
            result("過去問", 2, true),
            result("過去問", 3, false),
        ];
        let items = build_question_items(vec![a, b, c], &results, today);
        let readiness = build_readiness(&items, &["A".into(), "B".into()], 3.0, Some(4), Vec::new());
        // 習熟度 1・5・0 → 6/15 = 40%。不足は 4+0+5=9 回、4 日で 1 日 3 問
        assert!((readiness.percent - 40.0).abs() < 1e-9);
        assert_eq!(readiness.remaining_correct_answers, 9);
        assert_eq!(readiness.required_daily, Some(3));
        let names: Vec<&str> = readiness.areas.iter().map(|a| a.area.as_str()).collect();
        assert_eq!(names, ["A", "B", "マスタ外"]);
        assert_eq!(readiness.areas[0].mastered_count, 1);
        assert_eq!(build_readiness(&items, &[], 0.0, None, Vec::new()).required_daily, None);
    }

    #[test]
    fn question_items_get_mastery_and_next_review() {
        let today = NaiveDate::from_ymd_opt(2026, 10, 10).unwrap();
        // UTC 2026-10-01 15:00 は日本時間 10/2 0:00
        let records = vec![
            record("過去問", 1, "2026-10-01 15:00:00", true),
            record("過去問", 2, "2026-10-09 00:00:00", false),
        ];
        let results = vec![
            result("過去問", 1, false),
            result("過去問", 1, true),
            result("過去問", 1, true),
            result("過去問", 2, false),
        ];
        let items = build_question_items(records, &results, today);
        // 問1：2 回連続で正解 → 習熟度 2、10/2 の 7 日後
        assert_eq!(items[0].mastery_level, 2);
        assert_eq!(items[0].next_review_on, "2026-10-09");
        assert_eq!(items[0].answered_at, "2026-10-02 00:00:00");
        assert!(is_due(&items[0], today));
        // 問2：不正解 → 習熟度 0、翌日
        assert_eq!(items[1].mastery_level, 0);
        assert_eq!(items[1].next_review_on, "2026-10-10");
        assert!(is_due(&items[1], today));
        assert!(!is_due(&items[1], today - Duration::days(1)));
    }

    #[test]
    fn csv_field_escapes_special_characters() {
        assert_eq!(csv_field("abc"), "abc");
        assert_eq!(csv_field("a,b"), "\"a,b\"");
        assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_field("1行\n2行"), "\"1行\n2行\"");
        assert_eq!(csv_field("=SUM(A1)"), "'=SUM(A1)");
    }
}
