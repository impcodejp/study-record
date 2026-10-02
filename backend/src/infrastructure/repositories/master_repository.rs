//! 試験マスタ（exams）とカテゴリマスタ（categories）の永続化。

use sqlx::{FromRow, SqliteConnection};

use crate::domain::models::{Category, Exam};

/// 試験の行（件数の集計を含む）。
#[derive(Debug, FromRow)]
struct ExamRow {
    id: i64,
    name: String,
    sort_order: i64,
    created_at: String,
    practice_count: i64,
    answer_count: i64,
    exam_date: Option<String>,
    daily_goal: Option<i64>,
}

impl From<ExamRow> for Exam {
    fn from(row: ExamRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            sort_order: row.sort_order,
            created_at: row.created_at,
            practice_count: row.practice_count,
            answer_count: row.answer_count,
            exam_date: row.exam_date,
            daily_goal: row.daily_goal,
        }
    }
}

/// カテゴリの行（使用件数を含む）。
#[derive(Debug, FromRow)]
struct CategoryRow {
    id: i64,
    exam_id: i64,
    name: String,
    sort_order: i64,
    answer_count: i64,
}

impl From<CategoryRow> for Category {
    fn from(row: CategoryRow) -> Self {
        Self {
            id: row.id,
            exam_id: row.exam_id,
            name: row.name,
            sort_order: row.sort_order,
            answer_count: row.answer_count,
        }
    }
}

// ---------------------------------------------------------------------------
// 試験
// ---------------------------------------------------------------------------

/// ユーザーの試験を並び順→ID 順で返す。
pub async fn list_exams(conn: &mut SqliteConnection, user_id: i64) -> sqlx::Result<Vec<Exam>> {
    let rows: Vec<ExamRow> = sqlx::query_as(
        "SELECT e.id, e.name, e.sort_order, e.created_at, e.exam_date, e.daily_goal,
                (SELECT COUNT(*) FROM practices p WHERE p.exam_id = e.id) AS practice_count,
                (SELECT COUNT(*) FROM answers a JOIN practices p ON p.id = a.practice_id
                  WHERE p.exam_id = e.id AND a.correct IS NOT NULL) AS answer_count
           FROM exams e
          WHERE e.user_id = ?
          ORDER BY e.sort_order, e.id",
    )
    .bind(user_id)
    .fetch_all(conn)
    .await?;
    Ok(rows.into_iter().map(Exam::from).collect())
}

/// ユーザーの試験の名前を返す（他人の試験・存在しない試験は `None`）。
pub async fn find_exam_name(
    conn: &mut SqliteConnection,
    user_id: i64,
    exam_id: i64,
) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar("SELECT name FROM exams WHERE id = ? AND user_id = ?")
        .bind(exam_id)
        .bind(user_id)
        .fetch_optional(conn)
        .await
}

/// 同じ名前の試験があるか（大文字・小文字を区別しない）。`except_id` は判定から除く。
pub async fn exam_name_exists(
    conn: &mut SqliteConnection,
    user_id: i64,
    name: &str,
    except_id: Option<i64>,
) -> sqlx::Result<bool> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM exams WHERE user_id = ? AND name = ? AND id <> ?")
            .bind(user_id)
            .bind(name)
            .bind(except_id.unwrap_or(0))
            .fetch_one(conn)
            .await?;
    Ok(count > 0)
}

/// 試験を登録する（並び順は既存の最大値 + 1）。ID を返す。
pub async fn insert_exam(
    conn: &mut SqliteConnection,
    user_id: i64,
    name: &str,
    now_jst: &str,
) -> sqlx::Result<i64> {
    let result = sqlx::query(
        "INSERT INTO exams (user_id, name, sort_order, created_at)
         VALUES (?, ?, (SELECT COALESCE(MAX(sort_order), -1) + 1 FROM exams WHERE user_id = ?), ?)",
    )
    .bind(user_id)
    .bind(name)
    .bind(user_id)
    .bind(now_jst)
    .execute(conn)
    .await?;
    Ok(result.last_insert_rowid())
}

/// 試験名を変更する。
pub async fn rename_exam(conn: &mut SqliteConnection, exam_id: i64, name: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE exams SET name = ? WHERE id = ?")
        .bind(name)
        .bind(exam_id)
        .execute(conn)
        .await?;
    Ok(())
}

/// 試験の学習目標（試験日・1 日の目標問題数）を返す。試験が無ければ `None`。
pub async fn find_exam_goal(
    conn: &mut SqliteConnection,
    exam_id: i64,
) -> sqlx::Result<Option<(Option<String>, Option<i64>)>> {
    sqlx::query_as("SELECT exam_date, daily_goal FROM exams WHERE id = ?")
        .bind(exam_id)
        .fetch_optional(conn)
        .await
}

/// 試験の学習目標（試験日 `YYYY-MM-DD`・1 日の目標問題数）を更新する。`None` は未設定に戻す。
pub async fn update_exam_goal(
    conn: &mut SqliteConnection,
    exam_id: i64,
    exam_date: Option<&str>,
    daily_goal: Option<i64>,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE exams SET exam_date = ?, daily_goal = ? WHERE id = ?")
        .bind(exam_date)
        .bind(daily_goal)
        .bind(exam_id)
        .execute(conn)
        .await?;
    Ok(())
}

/// 試験を削除する（カテゴリは外部キーの CASCADE で削除される）。
pub async fn delete_exam(conn: &mut SqliteConnection, exam_id: i64) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM exams WHERE id = ?")
        .bind(exam_id)
        .execute(conn)
        .await?;
    Ok(())
}

/// 試験の学習件数を返す。
pub async fn count_practices(conn: &mut SqliteConnection, exam_id: i64) -> sqlx::Result<i64> {
    sqlx::query_scalar("SELECT COUNT(*) FROM practices WHERE exam_id = ?")
        .bind(exam_id)
        .fetch_one(conn)
        .await
}

// ---------------------------------------------------------------------------
// カテゴリ
// ---------------------------------------------------------------------------

/// 試験のカテゴリを並び順→ID 順で返す（使用件数付き）。
pub async fn list_categories(
    conn: &mut SqliteConnection,
    exam_id: i64,
) -> sqlx::Result<Vec<Category>> {
    let rows: Vec<CategoryRow> = sqlx::query_as(
        "SELECT c.id, c.exam_id, c.name, c.sort_order,
                (SELECT COUNT(*) FROM answers a JOIN practices p ON p.id = a.practice_id
                  WHERE p.exam_id = c.exam_id AND a.area = c.name) AS answer_count
           FROM categories c
          WHERE c.exam_id = ?
          ORDER BY c.sort_order, c.id",
    )
    .bind(exam_id)
    .fetch_all(conn)
    .await?;
    Ok(rows.into_iter().map(Category::from).collect())
}

/// 試験に属するカテゴリの名前を返す（他の試験のカテゴリは `None`）。
pub async fn find_category_name(
    conn: &mut SqliteConnection,
    exam_id: i64,
    category_id: i64,
) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar("SELECT name FROM categories WHERE id = ? AND exam_id = ?")
        .bind(category_id)
        .bind(exam_id)
        .fetch_optional(conn)
        .await
}

/// 名前が一致するカテゴリの正式な名前を返す（大文字・小文字を区別しない）。
pub async fn find_category_by_name(
    conn: &mut SqliteConnection,
    exam_id: i64,
    name: &str,
) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar("SELECT name FROM categories WHERE exam_id = ? AND name = ?")
        .bind(exam_id)
        .bind(name)
        .fetch_optional(conn)
        .await
}

/// 同じ名前のカテゴリがあるか。`except_id` は判定から除く。
pub async fn category_name_exists(
    conn: &mut SqliteConnection,
    exam_id: i64,
    name: &str,
    except_id: Option<i64>,
) -> sqlx::Result<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM categories WHERE exam_id = ? AND name = ? AND id <> ?",
    )
    .bind(exam_id)
    .bind(name)
    .bind(except_id.unwrap_or(0))
    .fetch_one(conn)
    .await?;
    Ok(count > 0)
}

/// カテゴリを登録する。`sort_order` が `None` なら既存の最大値 + 1。
pub async fn insert_category(
    conn: &mut SqliteConnection,
    exam_id: i64,
    name: &str,
    sort_order: Option<i64>,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO categories (exam_id, name, sort_order)
         VALUES (?, ?, COALESCE(?, (SELECT COALESCE(MAX(sort_order), -1) + 1 FROM categories WHERE exam_id = ?)))",
    )
    .bind(exam_id)
    .bind(name)
    .bind(sort_order)
    .bind(exam_id)
    .execute(conn)
    .await?;
    Ok(())
}

/// カテゴリ名を変更し、その試験で旧名称を使っている回答もすべて新名称に書き換える。
///
/// 呼び出し側でトランザクションを張ること。
pub async fn rename_category(
    conn: &mut SqliteConnection,
    exam_id: i64,
    category_id: i64,
    old_name: &str,
    new_name: &str,
) -> sqlx::Result<u64> {
    sqlx::query("UPDATE categories SET name = ? WHERE id = ?")
        .bind(new_name)
        .bind(category_id)
        .execute(&mut *conn)
        .await?;
    let result = sqlx::query(
        "UPDATE answers SET area = ?
          WHERE area = ? AND practice_id IN (SELECT id FROM practices WHERE exam_id = ?)",
    )
    .bind(new_name)
    .bind(old_name)
    .bind(exam_id)
    .execute(conn)
    .await?;
    Ok(result.rows_affected())
}

/// カテゴリの並び順を更新する。
pub async fn update_category_order(
    conn: &mut SqliteConnection,
    category_id: i64,
    sort_order: i64,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE categories SET sort_order = ? WHERE id = ?")
        .bind(sort_order)
        .bind(category_id)
        .execute(conn)
        .await?;
    Ok(())
}

/// カテゴリを削除する。
pub async fn delete_category(conn: &mut SqliteConnection, category_id: i64) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM categories WHERE id = ?")
        .bind(category_id)
        .execute(conn)
        .await?;
    Ok(())
}

/// そのカテゴリ名を使っている回答の件数（未採点を含む）。
pub async fn count_answers_with_area(
    conn: &mut SqliteConnection,
    exam_id: i64,
    name: &str,
) -> sqlx::Result<i64> {
    sqlx::query_scalar(
        "SELECT COUNT(*) FROM answers a JOIN practices p ON p.id = a.practice_id
          WHERE p.exam_id = ? AND a.area = ?",
    )
    .bind(exam_id)
    .bind(name)
    .fetch_one(conn)
    .await
}

/// 回答で使われているのにマスタに無いカテゴリを、並び順 1000 でマスタへ追加する（起動時処理）。
///
/// 全試験が対象。追加した件数を返す。
pub async fn register_orphan_areas(conn: &mut SqliteConnection) -> sqlx::Result<u64> {
    let result = sqlx::query(
        "INSERT OR IGNORE INTO categories (exam_id, name, sort_order)
         SELECT DISTINCT p.exam_id, a.area, 1000
           FROM answers a JOIN practices p ON p.id = a.practice_id
          WHERE NOT EXISTS (
                SELECT 1 FROM categories c WHERE c.exam_id = p.exam_id AND c.name = a.area)",
    )
    .execute(conn)
    .await?;
    Ok(result.rows_affected())
}
