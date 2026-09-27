//! 学習・回答・採点のユースケース（F-01〜F-07、F-11〜F-12）。

use sqlx::{SqliteConnection, SqlitePool};
use tracing::info;

use super::common::{ensure_category_name, ensure_exam};
use crate::{
    domain::{
        error::{AppError, AppResult},
        models::{Practice, PracticeSummary},
        time::{now_jst_string, now_utc_string},
        validation::{
            validate_correct_answer, validate_elapsed_seconds, validate_note,
            validate_question_number, validate_response, validate_title,
        },
    },
    infrastructure::{
        database::{begin_write, is_unique_violation},
        repositories::practice_repository::{self as repo, NewAnswer, PracticeRow},
    },
};

/// 回答の記録・修正で受け取る入力。
#[derive(Debug, Clone)]
pub struct AnswerInput {
    /// 問題名称。
    pub title: String,
    /// 問題数。
    pub question_number: i64,
    /// カテゴリ名。
    pub area: String,
    /// 回答。
    pub response: String,
    /// 回答時間（秒）。修正時は使わない。
    pub elapsed_seconds: i64,
}

/// 採点で受け取る入力。
#[derive(Debug, Clone)]
pub struct GradeInput {
    /// 正解なら true。
    pub correct: bool,
    /// 正解（任意）。
    pub correct_answer: Option<String>,
    /// メモ（任意）。
    pub note: Option<String>,
}

/// 検証済みの回答内容。
struct ValidAnswer {
    title: String,
    question_number: i64,
    area: String,
    response: String,
}

/// 完了済みの学習を操作しようとしたときのエラー。
fn already_finished() -> AppError {
    AppError::Conflict("この回の採点は終了しています。".into())
}

/// 同じ学習に同じ問題が記録済みのときのエラー。
fn duplicate_question() -> AppError {
    AppError::Conflict("この問題数はすでに記録されています。".into())
}

/// 進行中の学習があるときのエラー。
fn active_exists() -> AppError {
    AppError::Conflict("進行中の学習があります。".into())
}

/// 学習のユースケース。
#[derive(Clone)]
pub struct PracticeService {
    pool: SqlitePool,
}

impl PracticeService {
    /// サービスを作る。
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 進行中の学習を返す（F-04）。
    pub async fn get_active(&self, user_id: i64) -> AppResult<Option<Practice>> {
        let mut conn = self.pool.acquire().await?;
        match repo::find_active(&mut conn, user_id).await? {
            Some(row) => Ok(Some(load_practice(&mut conn, row).await?)),
            None => Ok(None),
        }
    }

    /// 学習を 1 件返す（F-12）。
    pub async fn get(&self, user_id: i64, practice_id: i64) -> AppResult<Practice> {
        let mut conn = self.pool.acquire().await?;
        let row = find_owned(&mut conn, user_id, practice_id).await?;
        load_practice(&mut conn, row).await
    }

    /// 試験の学習履歴一覧を返す（F-11）。
    pub async fn list_summaries(&self, user_id: i64, exam_id: i64) -> AppResult<Vec<PracticeSummary>> {
        let mut conn = self.pool.acquire().await?;
        ensure_exam(&mut conn, user_id, exam_id).await?;
        Ok(repo::list_summaries(&mut conn, exam_id).await?)
    }

    /// 学習を開始する（F-01）。同時に進行できる学習は 1 つだけ。
    pub async fn start(&self, user_id: i64, exam_id: i64, title: &str) -> AppResult<Practice> {
        let title = validate_title(title)?;
        let mut tx = begin_write(&self.pool).await?;
        ensure_exam(&mut tx, user_id, exam_id).await?;
        if repo::find_active(&mut tx, user_id).await?.is_some() {
            return Err(active_exists());
        }
        let practice_id =
            match repo::insert(&mut tx, user_id, exam_id, &title, &now_jst_string()).await {
                Ok(id) => id,
                // 部分ユニークインデックスによる同時開始の防止
                Err(err) if is_unique_violation(&err) => return Err(active_exists()),
                Err(err) => return Err(err.into()),
            };
        let row = find_owned(&mut tx, user_id, practice_id).await?;
        let practice = load_practice(&mut tx, row).await?;
        tx.commit().await?;
        info!(user_id, exam_id, practice_id, "学習を開始しました");
        Ok(practice)
    }

    /// 回答を記録する（F-02）。
    pub async fn record_answer(
        &self,
        user_id: i64,
        practice_id: i64,
        input: AnswerInput,
    ) -> AppResult<Practice> {
        let elapsed_seconds = validate_elapsed_seconds(input.elapsed_seconds)?;
        let mut tx = begin_write(&self.pool).await?;
        let row = find_owned(&mut tx, user_id, practice_id).await?;
        if row.completed_at.is_some() {
            return Err(already_finished());
        }
        let valid = validate_answer(&mut tx, row.exam_id, &input).await?;
        if repo::question_exists(&mut tx, practice_id, &valid.title, valid.question_number, None).await? {
            return Err(duplicate_question());
        }
        let now = now_utc_string();
        let new_answer = NewAnswer {
            title: &valid.title,
            question_number: valid.question_number,
            area: &valid.area,
            response: &valid.response,
            elapsed_seconds,
            answered_at_utc: &now,
        };
        let answer_id = match repo::insert_answer(&mut tx, practice_id, &new_answer).await {
            Ok(id) => id,
            Err(err) if is_unique_violation(&err) => return Err(duplicate_question()),
            Err(err) => return Err(err.into()),
        };
        let practice = load_practice(&mut tx, row).await?;
        tx.commit().await?;
        info!(user_id, practice_id, answer_id, "回答を記録しました");
        Ok(practice)
    }

    /// 未採点の回答の内容を修正する（入力ミスの訂正用）。回答時間は変えない。
    pub async fn update_answer(
        &self,
        user_id: i64,
        practice_id: i64,
        answer_id: i64,
        input: AnswerInput,
    ) -> AppResult<Practice> {
        let mut tx = begin_write(&self.pool).await?;
        let row = find_owned(&mut tx, user_id, practice_id).await?;
        ensure_editable_answer(&mut tx, user_id, &row, answer_id).await?;
        let valid = validate_answer(&mut tx, row.exam_id, &input).await?;
        if repo::question_exists(
            &mut tx,
            practice_id,
            &valid.title,
            valid.question_number,
            Some(answer_id),
        )
        .await?
        {
            return Err(duplicate_question());
        }
        match repo::update_ungraded_content(
            &mut tx,
            answer_id,
            &valid.title,
            valid.question_number,
            &valid.area,
            &valid.response,
        )
        .await
        {
            Ok(()) => {}
            Err(err) if is_unique_violation(&err) => return Err(duplicate_question()),
            Err(err) => return Err(err.into()),
        }
        let practice = load_practice(&mut tx, row).await?;
        tx.commit().await?;
        info!(user_id, practice_id, answer_id, "未採点の回答を修正しました");
        Ok(practice)
    }

    /// 未採点の回答を削除する（入力ミスの訂正用）。
    pub async fn delete_answer(
        &self,
        user_id: i64,
        practice_id: i64,
        answer_id: i64,
    ) -> AppResult<Practice> {
        let mut tx = begin_write(&self.pool).await?;
        let row = find_owned(&mut tx, user_id, practice_id).await?;
        ensure_editable_answer(&mut tx, user_id, &row, answer_id).await?;
        repo::delete_answer(&mut tx, answer_id).await?;
        // 残りがすべて採点済みなら、その時点で学習を完了にする（F-06 と同じ条件）。
        let (total, ungraded) = repo::count_answers(&mut tx, practice_id).await?;
        if total > 0 && ungraded == 0 {
            repo::complete(&mut tx, practice_id, &now_jst_string()).await?;
        }
        let row = find_owned(&mut tx, user_id, practice_id).await?;
        let practice = load_practice(&mut tx, row).await?;
        tx.commit().await?;
        info!(user_id, practice_id, answer_id, "未採点の回答を削除しました");
        Ok(practice)
    }

    /// 回答を採点する（F-05）。最後の回答を採点すると学習を完了にする（F-06）。
    pub async fn grade(
        &self,
        user_id: i64,
        practice_id: i64,
        answer_id: i64,
        input: GradeInput,
    ) -> AppResult<Practice> {
        let correct_answer = validate_correct_answer(input.correct_answer.as_deref())?;
        let note = validate_note(input.note.as_deref())?;
        let mut tx = begin_write(&self.pool).await?;
        let row = find_owned(&mut tx, user_id, practice_id).await?;
        if row.completed_at.is_some() {
            return Err(already_finished());
        }
        let owner = repo::find_answer_owner(&mut tx, user_id, answer_id).await?;
        if owner.as_ref().map(|o| o.practice_id) != Some(practice_id) {
            return Err(AppError::NotFound("回答が見つかりません。".into()));
        }
        if repo::oldest_ungraded_id(&mut tx, practice_id).await? != Some(answer_id) {
            return Err(AppError::Conflict("記録順に採点してください。".into()));
        }
        repo::grade(
            &mut tx,
            answer_id,
            input.correct,
            correct_answer.as_deref(),
            note.as_deref(),
        )
        .await?;
        let (_, ungraded) = repo::count_answers(&mut tx, practice_id).await?;
        if ungraded == 0 {
            repo::complete(&mut tx, practice_id, &now_jst_string()).await?;
            info!(user_id, practice_id, "すべての回答を採点したため学習を完了しました");
        }
        let row = find_owned(&mut tx, user_id, practice_id).await?;
        let practice = load_practice(&mut tx, row).await?;
        tx.commit().await?;
        info!(user_id, practice_id, answer_id, correct = input.correct, "採点しました");
        Ok(practice)
    }

    /// 学習を完了にする（F-06）。完了済みならそのまま返す。
    pub async fn finish(&self, user_id: i64, practice_id: i64) -> AppResult<Practice> {
        let mut tx = begin_write(&self.pool).await?;
        let row = find_owned(&mut tx, user_id, practice_id).await?;
        if row.completed_at.is_none() {
            let (total, ungraded) = repo::count_answers(&mut tx, practice_id).await?;
            if total == 0 || ungraded > 0 {
                return Err(AppError::Conflict("すべての回答を採点してください。".into()));
            }
            repo::complete(&mut tx, practice_id, &now_jst_string()).await?;
            info!(user_id, practice_id, "学習を完了しました");
        }
        let row = find_owned(&mut tx, user_id, practice_id).await?;
        let practice = load_practice(&mut tx, row).await?;
        tx.commit().await?;
        Ok(practice)
    }

    /// 進行中の学習を中断する。
    ///
    /// 起動時の復旧（F-28）と同じ考え方で、未採点の回答を削除し、
    /// 採点済みの回答が残れば完了扱いに、1 件も無ければ学習ごと削除する。
    /// 学習が残った場合はその学習を返す。
    pub async fn abort(&self, user_id: i64, practice_id: i64) -> AppResult<Option<Practice>> {
        let mut tx = begin_write(&self.pool).await?;
        let row = find_owned(&mut tx, user_id, practice_id).await?;
        if row.completed_at.is_some() {
            return Err(already_finished());
        }
        let deleted = repo::delete_ungraded(&mut tx, practice_id).await?;
        let (total, _) = repo::count_answers(&mut tx, practice_id).await?;
        let result = if total == 0 {
            repo::delete(&mut tx, practice_id).await?;
            None
        } else {
            repo::complete(&mut tx, practice_id, &now_jst_string()).await?;
            let row = find_owned(&mut tx, user_id, practice_id).await?;
            Some(load_practice(&mut tx, row).await?)
        };
        tx.commit().await?;
        info!(user_id, practice_id, deleted_answers = deleted, kept_answers = total, "学習を中断しました");
        Ok(result)
    }
}

/// ログイン中のユーザーの学習を取得する。無ければ 404。
async fn find_owned(
    conn: &mut SqliteConnection,
    user_id: i64,
    practice_id: i64,
) -> AppResult<PracticeRow> {
    repo::find(conn, user_id, practice_id)
        .await?
        .ok_or_else(AppError::practice_not_found)
}

/// 学習の行に回答を読み込んで、集計値付きの学習にする。
async fn load_practice(conn: &mut SqliteConnection, row: PracticeRow) -> AppResult<Practice> {
    let answers = repo::list_answers(conn, row.id).await?;
    let graded_count = answers.iter().filter(|a| a.correct.is_some()).count() as i64;
    let correct_count = answers.iter().filter(|a| a.correct == Some(true)).count() as i64;
    Ok(Practice {
        id: row.id,
        exam_id: row.exam_id,
        exam_name: row.exam_name,
        title: row.title,
        created_at: row.created_at,
        completed_at: row.completed_at,
        question_count: answers.len() as i64,
        graded_count,
        correct_count,
        answers,
    })
}

/// 回答の入力を検証する（カテゴリはマスタ上の正式な名前にそろえる）。
async fn validate_answer(
    conn: &mut SqliteConnection,
    exam_id: i64,
    input: &AnswerInput,
) -> AppResult<ValidAnswer> {
    let title = validate_title(&input.title)?;
    let question_number = validate_question_number(input.question_number)?;
    let response = validate_response(&input.response)?;
    let area = ensure_category_name(conn, exam_id, &input.area).await?;
    Ok(ValidAnswer {
        title,
        question_number,
        area,
        response,
    })
}

/// 修正・削除できる回答（進行中の学習の、未採点の回答）であることを確かめる。
async fn ensure_editable_answer(
    conn: &mut SqliteConnection,
    user_id: i64,
    practice: &PracticeRow,
    answer_id: i64,
) -> AppResult<()> {
    if practice.completed_at.is_some() {
        return Err(already_finished());
    }
    let owner = repo::find_answer_owner(conn, user_id, answer_id)
        .await?
        .filter(|o| o.practice_id == practice.id)
        .ok_or_else(|| AppError::NotFound("回答が見つかりません。".into()))?;
    if owner.correct.is_some() {
        return Err(AppError::Conflict(
            "採点済みの回答は修正・削除できません。".into(),
        ));
    }
    Ok(())
}
