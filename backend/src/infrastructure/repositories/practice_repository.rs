//! 学習（practices）と回答（answers）の永続化・集計。
//!
//! - `practices.created_at` / `completed_at` は日本時間で保存する
//! - `answers.answered_at` は UTC で保存し、取り出すときに日本時間へ変換する

use sqlx::{FromRow, SqliteConnection};

use crate::domain::{
    models::{Answer, Attempt, NoteItem, PracticeSummary},
    time::utc_string_to_jst_string,
};

/// 学習の行（回答を含まない）。
#[derive(Debug, Clone, FromRow)]
pub struct PracticeRow {
    /// 学習 ID。
    pub id: i64,
    /// 試験 ID。
    pub exam_id: i64,
    /// 試験名。
    pub exam_name: String,
    /// 開始時の問題名称。
    pub title: String,
    /// 開始日時（日本時間）。
    pub created_at: String,
    /// 完了日時（日本時間）。
    pub completed_at: Option<String>,
}

/// 回答の行。
#[derive(Debug, Clone, FromRow)]
struct AnswerRow {
    id: i64,
    title: String,
    question_number: i64,
    area: String,
    response: String,
    elapsed_seconds: i64,
    answered_at: String,
    correct: Option<bool>,
    correct_answer: Option<String>,
    note: Option<String>,
}

impl From<AnswerRow> for Answer {
    fn from(row: AnswerRow) -> Self {
        Self {
            id: row.id,
            title: row.title,
            question_number: row.question_number,
            area: row.area,
            response: row.response,
            elapsed_seconds: row.elapsed_seconds,
            answered_at: utc_string_to_jst_string(&row.answered_at),
            correct: row.correct,
            correct_answer: row.correct_answer,
            note: row.note,
        }
    }
}

/// 問題一覧の行（問題名称＋問題数ごとの最新の採点結果と回数）。
///
/// 習熟度・次の復習日はアプリケーション層で正誤の履歴から求める。
#[derive(Debug, FromRow)]
pub struct QuestionRecord {
    /// 問題名称。
    pub title: String,
    /// 問題数。
    pub question_number: i64,
    /// 最新回答のカテゴリ名。
    pub area: String,
    /// 最新の回答。
    pub response: String,
    /// 最終回答日時（UTC）。
    pub answered_at: String,
    /// 最新の正誤。
    pub correct: bool,
    /// 回答回数。
    pub attempt_count: i64,
    /// 正解した回数。
    pub correct_count: i64,
    /// 平均回答時間（秒）。
    pub average_seconds: f64,
}

/// 採点済みの回答 1 件分の正誤（習熟度の計算用）。
#[derive(Debug, FromRow)]
pub struct ResultRecord {
    /// 問題名称。
    pub title: String,
    /// 問題数。
    pub question_number: i64,
    /// 正誤。
    pub correct: bool,
    /// 回答日時（UTC。準備度の推移の計算用）。
    pub answered_at: String,
}

/// 正誤履歴の行。
#[derive(Debug, FromRow)]
struct AttemptRow {
    id: i64,
    practice_id: i64,
    area: String,
    response: String,
    elapsed_seconds: i64,
    answered_at: String,
    correct: bool,
    correct_answer: Option<String>,
    note: Option<String>,
}

/// 回答の所有者確認用の情報。
#[derive(Debug, Clone, FromRow)]
pub struct AnswerOwner {
    /// 回答 ID。
    pub answer_id: i64,
    /// 学習 ID。
    pub practice_id: i64,
    /// 試験 ID。
    pub exam_id: i64,
    /// 学習が完了しているか。
    pub practice_completed: bool,
    /// 正誤（未採点は `None`）。
    pub correct: Option<bool>,
}

/// 新しく記録する回答。
#[derive(Debug, Clone)]
pub struct NewAnswer<'a> {
    /// 問題名称。
    pub title: &'a str,
    /// 問題数。
    pub question_number: i64,
    /// カテゴリ名。
    pub area: &'a str,
    /// 回答。
    pub response: &'a str,
    /// 回答時間（秒）。
    pub elapsed_seconds: i64,
    /// 記録日時（UTC）。
    pub answered_at_utc: &'a str,
}

/// 学習を取得するときの共通 SELECT 句。
const PRACTICE_SELECT: &str = "SELECT p.id, p.exam_id, e.name AS exam_name, p.title, p.created_at, p.completed_at
                                 FROM practices p JOIN exams e ON e.id = p.exam_id";

// ---------------------------------------------------------------------------
// 学習
// ---------------------------------------------------------------------------

/// ユーザーの進行中の学習を返す。
pub async fn find_active(
    conn: &mut SqliteConnection,
    user_id: i64,
) -> sqlx::Result<Option<PracticeRow>> {
    sqlx::query_as(&format!(
        "{PRACTICE_SELECT} WHERE p.user_id = ? AND p.completed_at IS NULL"
    ))
    .bind(user_id)
    .fetch_optional(conn)
    .await
}

/// ユーザーの学習を ID で返す（他人の学習は `None`）。
pub async fn find(
    conn: &mut SqliteConnection,
    user_id: i64,
    practice_id: i64,
) -> sqlx::Result<Option<PracticeRow>> {
    sqlx::query_as(&format!("{PRACTICE_SELECT} WHERE p.id = ? AND p.user_id = ?"))
        .bind(practice_id)
        .bind(user_id)
        .fetch_optional(conn)
        .await
}

/// 学習を開始する。ID を返す。
pub async fn insert(
    conn: &mut SqliteConnection,
    user_id: i64,
    exam_id: i64,
    title: &str,
    created_at_jst: &str,
) -> sqlx::Result<i64> {
    let result = sqlx::query(
        "INSERT INTO practices (user_id, exam_id, title, created_at) VALUES (?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind(exam_id)
    .bind(title)
    .bind(created_at_jst)
    .execute(conn)
    .await?;
    Ok(result.last_insert_rowid())
}

/// 学習を完了にする。
pub async fn complete(
    conn: &mut SqliteConnection,
    practice_id: i64,
    completed_at_jst: &str,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE practices SET completed_at = ? WHERE id = ? AND completed_at IS NULL")
        .bind(completed_at_jst)
        .bind(practice_id)
        .execute(conn)
        .await?;
    Ok(())
}

/// 学習を削除する（回答は CASCADE で削除される）。
pub async fn delete(conn: &mut SqliteConnection, practice_id: i64) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM practices WHERE id = ?")
        .bind(practice_id)
        .execute(conn)
        .await?;
    Ok(())
}

/// 試験の学習履歴を「学習 × 問題名称」単位で新しい順に返す（F-11）。
///
/// 集計は採点済みの回答だけを対象にする。
pub async fn list_summaries(
    conn: &mut SqliteConnection,
    exam_id: i64,
) -> sqlx::Result<Vec<PracticeSummary>> {
    let rows: Vec<(i64, String, String, Option<String>, i64, i64)> = sqlx::query_as(
        "SELECT p.id, a.title, p.created_at, p.completed_at,
                COUNT(*) AS question_count,
                COALESCE(SUM(a.correct), 0) AS correct_count
           FROM practices p JOIN answers a ON a.practice_id = p.id
          WHERE p.exam_id = ? AND a.correct IS NOT NULL
          GROUP BY p.id, a.title
          ORDER BY p.id DESC, MIN(a.id)",
    )
    .bind(exam_id)
    .fetch_all(conn)
    .await?;
    Ok(rows
        .into_iter()
        .map(
            |(id, title, created_at, completed_at, question_count, correct_count)| PracticeSummary {
                id,
                title,
                created_at,
                completed_at,
                question_count,
                correct_count,
            },
        )
        .collect())
}

// ---------------------------------------------------------------------------
// 回答
// ---------------------------------------------------------------------------

/// 学習の回答を記録順に返す。
pub async fn list_answers(
    conn: &mut SqliteConnection,
    practice_id: i64,
) -> sqlx::Result<Vec<Answer>> {
    let rows: Vec<AnswerRow> = sqlx::query_as(
        "SELECT id, title, question_number, area, response, elapsed_seconds, answered_at,
                correct, correct_answer, note
           FROM answers WHERE practice_id = ? ORDER BY id",
    )
    .bind(practice_id)
    .fetch_all(conn)
    .await?;
    Ok(rows.into_iter().map(Answer::from).collect())
}

/// 回答を記録する。
pub async fn insert_answer(
    conn: &mut SqliteConnection,
    practice_id: i64,
    answer: &NewAnswer<'_>,
) -> sqlx::Result<i64> {
    let result = sqlx::query(
        "INSERT INTO answers (practice_id, title, question_number, area, response, elapsed_seconds, answered_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(practice_id)
    .bind(answer.title)
    .bind(answer.question_number)
    .bind(answer.area)
    .bind(answer.response)
    .bind(answer.elapsed_seconds)
    .bind(answer.answered_at_utc)
    .execute(conn)
    .await?;
    Ok(result.last_insert_rowid())
}

/// 同じ学習に「問題名称＋問題数」が記録済みか。`except_id` は判定から除く。
pub async fn question_exists(
    conn: &mut SqliteConnection,
    practice_id: i64,
    title: &str,
    question_number: i64,
    except_id: Option<i64>,
) -> sqlx::Result<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM answers
          WHERE practice_id = ? AND title = ? AND question_number = ? AND id <> ?",
    )
    .bind(practice_id)
    .bind(title)
    .bind(question_number)
    .bind(except_id.unwrap_or(0))
    .fetch_one(conn)
    .await?;
    Ok(count > 0)
}

/// ユーザーの回答の所有者情報を返す（他人の回答は `None`）。
pub async fn find_answer_owner(
    conn: &mut SqliteConnection,
    user_id: i64,
    answer_id: i64,
) -> sqlx::Result<Option<AnswerOwner>> {
    sqlx::query_as(
        "SELECT a.id AS answer_id, a.practice_id, p.exam_id,
                (p.completed_at IS NOT NULL) AS practice_completed, a.correct
           FROM answers a JOIN practices p ON p.id = a.practice_id
          WHERE a.id = ? AND p.user_id = ?",
    )
    .bind(answer_id)
    .bind(user_id)
    .fetch_optional(conn)
    .await
}

/// 学習の未採点の回答のうち、最も古いものの ID を返す。
pub async fn oldest_ungraded_id(
    conn: &mut SqliteConnection,
    practice_id: i64,
) -> sqlx::Result<Option<i64>> {
    sqlx::query_scalar(
        "SELECT MIN(id) FROM answers WHERE practice_id = ? AND correct IS NULL",
    )
    .bind(practice_id)
    .fetch_one(conn)
    .await
}

/// 学習の回答件数と未採点件数を返す。
pub async fn count_answers(
    conn: &mut SqliteConnection,
    practice_id: i64,
) -> sqlx::Result<(i64, i64)> {
    sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(correct IS NULL), 0) FROM answers WHERE practice_id = ?",
    )
    .bind(practice_id)
    .fetch_one(conn)
    .await
}

/// 回答を採点し、正解・メモを記録する。
pub async fn grade(
    conn: &mut SqliteConnection,
    answer_id: i64,
    correct: bool,
    correct_answer: Option<&str>,
    note: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE answers SET correct = ?, correct_answer = ?, note = ? WHERE id = ?")
        .bind(correct)
        .bind(correct_answer)
        .bind(note)
        .bind(answer_id)
        .execute(conn)
        .await?;
    Ok(())
}

/// 採点済みの回答の正解・メモを更新する。
pub async fn update_memo(
    conn: &mut SqliteConnection,
    answer_id: i64,
    correct_answer: Option<&str>,
    note: Option<&str>,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE answers SET correct_answer = ?, note = ? WHERE id = ?")
        .bind(correct_answer)
        .bind(note)
        .bind(answer_id)
        .execute(conn)
        .await?;
    Ok(())
}

/// 未採点の回答の内容（問題名称・問題数・カテゴリ・回答）を修正する。
pub async fn update_ungraded_content(
    conn: &mut SqliteConnection,
    answer_id: i64,
    title: &str,
    question_number: i64,
    area: &str,
    response: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE answers SET title = ?, question_number = ?, area = ?, response = ?
          WHERE id = ? AND correct IS NULL",
    )
    .bind(title)
    .bind(question_number)
    .bind(area)
    .bind(response)
    .bind(answer_id)
    .execute(conn)
    .await?;
    Ok(())
}

/// 回答を 1 件削除する。
pub async fn delete_answer(conn: &mut SqliteConnection, answer_id: i64) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM answers WHERE id = ?")
        .bind(answer_id)
        .execute(conn)
        .await?;
    Ok(())
}

/// 回答のカテゴリを変更する。
pub async fn update_area(
    conn: &mut SqliteConnection,
    answer_id: i64,
    area: &str,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE answers SET area = ? WHERE id = ?")
        .bind(area)
        .bind(answer_id)
        .execute(conn)
        .await?;
    Ok(())
}

/// 学習の未採点の回答をすべて削除する。削除件数を返す。
pub async fn delete_ungraded(conn: &mut SqliteConnection, practice_id: i64) -> sqlx::Result<u64> {
    let result = sqlx::query("DELETE FROM answers WHERE practice_id = ? AND correct IS NULL")
        .bind(practice_id)
        .execute(conn)
        .await?;
    Ok(result.rows_affected())
}

// ---------------------------------------------------------------------------
// 振り返り
// ---------------------------------------------------------------------------

/// 試験の問題一覧（問題名称＋問題数ごとの最新の採点結果）を最終回答が新しい順に返す（F-13）。
pub async fn list_questions(
    conn: &mut SqliteConnection,
    exam_id: i64,
) -> sqlx::Result<Vec<QuestionRecord>> {
    sqlx::query_as(
        "WITH graded AS (
             SELECT a.* FROM answers a JOIN practices p ON p.id = a.practice_id
              WHERE p.exam_id = ? AND a.correct IS NOT NULL
         ),
         grouped AS (
             SELECT title, question_number, MAX(id) AS last_id,
                    COUNT(*) AS attempt_count, SUM(correct) AS correct_count,
                    AVG(elapsed_seconds) AS average_seconds
               FROM graded GROUP BY title, question_number
         )
         SELECT g.title, g.question_number, a.area, a.response, a.answered_at, a.correct,
                g.attempt_count, g.correct_count, g.average_seconds
           FROM grouped g JOIN answers a ON a.id = g.last_id
          ORDER BY a.answered_at DESC, a.id DESC",
    )
    .bind(exam_id)
    .fetch_all(conn)
    .await
}

/// 試験の採点済み回答の正誤を、問題ごとに記録順（古い順）で返す（習熟度の計算用）。
pub async fn list_results(
    conn: &mut SqliteConnection,
    exam_id: i64,
) -> sqlx::Result<Vec<ResultRecord>> {
    sqlx::query_as(
        "SELECT a.title, a.question_number, a.correct, a.answered_at
           FROM answers a JOIN practices p ON p.id = a.practice_id
          WHERE p.exam_id = ? AND a.correct IS NOT NULL
          ORDER BY a.title, a.question_number, a.id",
    )
    .bind(exam_id)
    .fetch_all(conn)
    .await
}

/// 試験で採点済みの回答がある日（日本時間、`YYYY-MM-DD`）を古い順に返す（連続学習日数の計算用）。
pub async fn study_days(conn: &mut SqliteConnection, exam_id: i64) -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar(
        "SELECT DISTINCT date(a.answered_at, '+9 hours') AS day
           FROM answers a JOIN practices p ON p.id = a.practice_id
          WHERE p.exam_id = ? AND a.correct IS NOT NULL
          ORDER BY day",
    )
    .bind(exam_id)
    .fetch_all(conn)
    .await
}

/// 試験の「問題名称＋問題数」の採点済み回答を、学習をまたいで新しい順に返す（F-16）。
pub async fn list_attempts(
    conn: &mut SqliteConnection,
    exam_id: i64,
    title: &str,
    question_number: i64,
) -> sqlx::Result<Vec<Attempt>> {
    let rows: Vec<AttemptRow> = sqlx::query_as(
        "SELECT a.id, a.practice_id, a.area, a.response, a.elapsed_seconds, a.answered_at,
                a.correct, a.correct_answer, a.note
           FROM answers a JOIN practices p ON p.id = a.practice_id
          WHERE p.exam_id = ? AND a.title = ? AND a.question_number = ? AND a.correct IS NOT NULL
          ORDER BY a.id DESC",
    )
    .bind(exam_id)
    .bind(title)
    .bind(question_number)
    .fetch_all(conn)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| Attempt {
            id: row.id,
            practice_id: row.practice_id,
            area: row.area,
            response: row.response,
            elapsed_seconds: row.elapsed_seconds,
            answered_at: utc_string_to_jst_string(&row.answered_at),
            correct: row.correct,
            correct_answer: row.correct_answer,
            note: row.note,
        })
        .collect())
}

/// 見直しノートの行。
#[derive(Debug, FromRow)]
struct NoteRow {
    id: i64,
    practice_id: i64,
    title: String,
    question_number: i64,
    area: String,
    response: String,
    correct: bool,
    correct_answer: Option<String>,
    note: Option<String>,
    answered_at: String,
}

/// 正解またはメモを残した採点済みの回答を、新しい順に返す（見直しノート）。
pub async fn list_notes(conn: &mut SqliteConnection, exam_id: i64) -> sqlx::Result<Vec<NoteItem>> {
    let rows: Vec<NoteRow> = sqlx::query_as(
        "SELECT a.id, a.practice_id, a.title, a.question_number, a.area, a.response, a.correct,
                a.correct_answer, a.note, a.answered_at
           FROM answers a JOIN practices p ON p.id = a.practice_id
          WHERE p.exam_id = ? AND a.correct IS NOT NULL
            AND (a.correct_answer IS NOT NULL OR a.note IS NOT NULL)
          ORDER BY a.answered_at DESC, a.id DESC",
    )
    .bind(exam_id)
    .fetch_all(conn)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| NoteItem {
            answer_id: row.id,
            practice_id: row.practice_id,
            title: row.title,
            question_number: row.question_number,
            area: row.area,
            response: row.response,
            correct: row.correct,
            correct_answer: row.correct_answer,
            note: row.note,
            answered_at: utc_string_to_jst_string(&row.answered_at),
        })
        .collect())
}

// ---------------------------------------------------------------------------
// 集計
// ---------------------------------------------------------------------------

/// 試験の採点済み回答の件数・正解数・平均回答時間を返す（F-08）。
pub async fn totals(conn: &mut SqliteConnection, exam_id: i64) -> sqlx::Result<(i64, i64, f64)> {
    sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(a.correct), 0), COALESCE(AVG(a.elapsed_seconds), 0.0)
           FROM answers a JOIN practices p ON p.id = a.practice_id
          WHERE p.exam_id = ? AND a.correct IS NOT NULL",
    )
    .bind(exam_id)
    .fetch_one(conn)
    .await
}

/// 指定した日（日本時間）以降の、日本時間の日付ごとの件数・正解数を返す（F-09）。
pub async fn daily_counts_since(
    conn: &mut SqliteConnection,
    exam_id: i64,
    since_date_jst: &str,
) -> sqlx::Result<Vec<(String, i64, i64)>> {
    sqlx::query_as(
        "SELECT date(a.answered_at, '+9 hours') AS day, COUNT(*), COALESCE(SUM(a.correct), 0)
           FROM answers a JOIN practices p ON p.id = a.practice_id
          WHERE p.exam_id = ? AND a.correct IS NOT NULL
            AND date(a.answered_at, '+9 hours') >= ?
          GROUP BY day",
    )
    .bind(exam_id)
    .bind(since_date_jst)
    .fetch_all(conn)
    .await
}

/// カテゴリ名ごとの解答数・正解数を返す（F-10）。
pub async fn area_counts(
    conn: &mut SqliteConnection,
    exam_id: i64,
) -> sqlx::Result<Vec<(String, i64, i64)>> {
    sqlx::query_as(
        "SELECT a.area, COUNT(*), COALESCE(SUM(a.correct), 0)
           FROM answers a JOIN practices p ON p.id = a.practice_id
          WHERE p.exam_id = ? AND a.correct IS NOT NULL
          GROUP BY a.area",
    )
    .bind(exam_id)
    .fetch_all(conn)
    .await
}

/// CSV 出力用の 1 行。
#[derive(Debug, Clone, FromRow)]
pub struct ExportRow {
    /// 学習 ID。
    pub practice_id: i64,
    /// 学習の開始日時（日本時間）。
    pub practice_created_at: String,
    /// 問題名称。
    pub title: String,
    /// 問題数。
    pub question_number: i64,
    /// カテゴリ名。
    pub area: String,
    /// 回答。
    pub response: String,
    /// 回答時間（秒）。
    pub elapsed_seconds: i64,
    /// 回答日時（UTC）。
    pub answered_at: String,
    /// 正誤。
    pub correct: bool,
    /// 正解。
    pub correct_answer: Option<String>,
    /// メモ。
    pub note: Option<String>,
}

/// 試験の採点済み回答を記録順に返す（CSV 出力用）。
pub async fn export_rows(conn: &mut SqliteConnection, exam_id: i64) -> sqlx::Result<Vec<ExportRow>> {
    sqlx::query_as(
        "SELECT a.practice_id, p.created_at AS practice_created_at, a.title, a.question_number,
                a.area, a.response, a.elapsed_seconds, a.answered_at, a.correct,
                a.correct_answer, a.note
           FROM answers a JOIN practices p ON p.id = a.practice_id
          WHERE p.exam_id = ? AND a.correct IS NOT NULL
          ORDER BY a.id",
    )
    .bind(exam_id)
    .fetch_all(conn)
    .await
}

// ---------------------------------------------------------------------------
// 起動時の復旧（F-28）
// ---------------------------------------------------------------------------

/// 起動時の復旧処理を行う。
///
/// 1. 未採点の回答を削除する
/// 2. 未完了で回答が 0 件の学習を削除する
/// 3. 残った未完了の学習を完了扱いにする
///
/// 呼び出し側でトランザクションを張ること。各処理の件数を返す。
pub async fn recover_on_startup(
    conn: &mut SqliteConnection,
    now_jst: &str,
) -> sqlx::Result<(u64, u64, u64)> {
    let deleted_answers = sqlx::query("DELETE FROM answers WHERE correct IS NULL")
        .execute(&mut *conn)
        .await?
        .rows_affected();
    let deleted_practices = sqlx::query(
        "DELETE FROM practices
          WHERE completed_at IS NULL
            AND NOT EXISTS (SELECT 1 FROM answers a WHERE a.practice_id = practices.id)",
    )
    .execute(&mut *conn)
    .await?
    .rows_affected();
    let completed = sqlx::query("UPDATE practices SET completed_at = ? WHERE completed_at IS NULL")
        .bind(now_jst)
        .execute(conn)
        .await?
        .rows_affected();
    Ok((deleted_answers, deleted_practices, completed))
}
