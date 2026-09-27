//! アプリケーション層の共通処理。

use sqlx::SqliteConnection;

use crate::{
    domain::error::{AppError, AppResult},
    infrastructure::repositories::master_repository,
};

/// ログイン中のユーザーの試験であることを確かめ、試験名を返す。
///
/// 他人の試験・存在しない試験は 404 とする（存在の有無を他人に知らせないため）。
pub async fn ensure_exam(
    conn: &mut SqliteConnection,
    user_id: i64,
    exam_id: i64,
) -> AppResult<String> {
    master_repository::find_exam_name(conn, user_id, exam_id)
        .await?
        .ok_or_else(|| AppError::NotFound("試験が見つかりません。".into()))
}

/// 入力されたカテゴリ名が試験のカテゴリマスタにあることを確かめ、マスタ上の正式な名前を返す。
pub async fn ensure_category_name(
    conn: &mut SqliteConnection,
    exam_id: i64,
    name: &str,
) -> AppResult<String> {
    master_repository::find_category_by_name(conn, exam_id, name.trim())
        .await?
        .ok_or_else(|| AppError::Validation("カテゴリを選択してください。".into()))
}
