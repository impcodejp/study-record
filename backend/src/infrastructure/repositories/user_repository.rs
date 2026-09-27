//! ユーザー（users）・セッション（sessions）・メールトークン（email_tokens）の永続化。
//!
//! 日時はすべて UTC の `YYYY-MM-DD HH:MM:SS` 文字列で保存する。

use sqlx::{FromRow, SqliteConnection};

use crate::domain::models::User;

/// 認証用にパスワードハッシュを含むユーザー情報。
#[derive(Debug, Clone, FromRow)]
pub struct UserRecord {
    /// ユーザー ID。
    pub id: i64,
    /// ユーザー名。
    pub name: String,
    /// メールアドレス。
    pub email: String,
    /// パスワードハッシュ。
    pub password_hash: String,
}

impl UserRecord {
    /// 公開用のユーザー情報に変換する。
    pub fn to_user(&self) -> User {
        User {
            id: self.id,
            name: self.name.clone(),
            email: self.email.clone(),
        }
    }
}

/// メールで送ったワンタイムトークンの内容。
#[derive(Debug, Clone, FromRow)]
pub struct EmailTokenRecord {
    /// 宛先メールアドレス。
    pub email: String,
    /// 新規登録時のユーザー名（再設定では `None`）。
    pub user_name: Option<String>,
}

/// トークンの用途。
#[derive(Debug, Clone, Copy)]
pub enum TokenPurpose {
    /// 新規登録の確認。
    Verify,
    /// パスワード再設定。
    Reset,
}

impl TokenPurpose {
    /// DB に保存する文字列。
    fn as_str(self) -> &'static str {
        match self {
            Self::Verify => "verify",
            Self::Reset => "reset",
        }
    }
}

/// メールアドレスでユーザーを探す（大文字・小文字を区別しない）。
pub async fn find_by_email(
    conn: &mut SqliteConnection,
    email: &str,
) -> sqlx::Result<Option<UserRecord>> {
    sqlx::query_as("SELECT id, name, email, password_hash FROM users WHERE email = ?")
        .bind(email)
        .fetch_optional(conn)
        .await
}

/// ID でユーザーを探す。
pub async fn find_by_id(conn: &mut SqliteConnection, id: i64) -> sqlx::Result<Option<UserRecord>> {
    sqlx::query_as("SELECT id, name, email, password_hash FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(conn)
        .await
}

/// ユーザーを登録し、ID を返す。
pub async fn insert(
    conn: &mut SqliteConnection,
    name: &str,
    email: &str,
    password_hash: &str,
    now_utc: &str,
) -> sqlx::Result<i64> {
    let result = sqlx::query(
        "INSERT INTO users (name, email, password_hash, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(name)
    .bind(email)
    .bind(password_hash)
    .bind(now_utc)
    .bind(now_utc)
    .execute(conn)
    .await?;
    Ok(result.last_insert_rowid())
}

/// パスワードハッシュを更新する。
pub async fn update_password(
    conn: &mut SqliteConnection,
    user_id: i64,
    password_hash: &str,
    now_utc: &str,
) -> sqlx::Result<()> {
    sqlx::query("UPDATE users SET password_hash = ?, updated_at = ? WHERE id = ?")
        .bind(password_hash)
        .bind(now_utc)
        .bind(user_id)
        .execute(conn)
        .await?;
    Ok(())
}

/// 全ユーザーの ID を返す（起動時処理用）。
pub async fn list_ids(conn: &mut SqliteConnection) -> sqlx::Result<Vec<i64>> {
    sqlx::query_scalar("SELECT id FROM users ORDER BY id")
        .fetch_all(conn)
        .await
}

// ---------------------------------------------------------------------------
// セッション
// ---------------------------------------------------------------------------

/// セッションを保存する。
pub async fn insert_session(
    conn: &mut SqliteConnection,
    token_hash: &str,
    user_id: i64,
    csrf_token: &str,
    now_utc: &str,
    expires_at_utc: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO sessions (token_hash, user_id, csrf_token, created_at, expires_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(token_hash)
    .bind(user_id)
    .bind(csrf_token)
    .bind(now_utc)
    .bind(expires_at_utc)
    .execute(conn)
    .await?;
    Ok(())
}

/// 有効期限内のセッションから、ユーザーと CSRF トークンを取得する。
pub async fn find_session(
    conn: &mut SqliteConnection,
    token_hash: &str,
    now_utc: &str,
) -> sqlx::Result<Option<(User, String)>> {
    let row: Option<(i64, String, String, String)> = sqlx::query_as(
        "SELECT u.id, u.name, u.email, s.csrf_token
           FROM sessions s JOIN users u ON u.id = s.user_id
          WHERE s.token_hash = ? AND s.expires_at > ?",
    )
    .bind(token_hash)
    .bind(now_utc)
    .fetch_optional(conn)
    .await?;
    Ok(row.map(|(id, name, email, csrf)| (User { id, name, email }, csrf)))
}

/// セッションを 1 件削除する（ログアウト）。
pub async fn delete_session(conn: &mut SqliteConnection, token_hash: &str) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM sessions WHERE token_hash = ?")
        .bind(token_hash)
        .execute(conn)
        .await?;
    Ok(())
}

/// ユーザーのセッションをすべて削除する。`keep` を指定するとそのセッションだけ残す。
pub async fn delete_sessions_of_user(
    conn: &mut SqliteConnection,
    user_id: i64,
    keep: Option<&str>,
) -> sqlx::Result<u64> {
    let result = sqlx::query("DELETE FROM sessions WHERE user_id = ? AND token_hash <> ?")
        .bind(user_id)
        .bind(keep.unwrap_or(""))
        .execute(conn)
        .await?;
    Ok(result.rows_affected())
}

/// 期限切れのセッションと、`token_cutoff_utc` より前に期限切れになったメールトークンを削除する。
///
/// メールトークンは確認メールの再送（期限切れ後の再送を含む）に使うため、期限後もしばらく残す。
/// 削除件数を返す。
pub async fn delete_expired(
    conn: &mut SqliteConnection,
    now_utc: &str,
    token_cutoff_utc: &str,
) -> sqlx::Result<u64> {
    let sessions = sqlx::query("DELETE FROM sessions WHERE expires_at <= ?")
        .bind(now_utc)
        .execute(&mut *conn)
        .await?
        .rows_affected();
    let tokens = sqlx::query("DELETE FROM email_tokens WHERE expires_at <= ?")
        .bind(token_cutoff_utc)
        .execute(&mut *conn)
        .await?
        .rows_affected();
    Ok(sessions + tokens)
}

// ---------------------------------------------------------------------------
// メールトークン
// ---------------------------------------------------------------------------

/// メールトークンを保存する。同じ用途・同じアドレスの古いトークンは無効にする。
pub async fn replace_email_token(
    conn: &mut SqliteConnection,
    purpose: TokenPurpose,
    token_hash: &str,
    email: &str,
    user_name: Option<&str>,
    now_utc: &str,
    expires_at_utc: &str,
) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM email_tokens WHERE purpose = ? AND email = ?")
        .bind(purpose.as_str())
        .bind(email)
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        "INSERT INTO email_tokens (token_hash, purpose, email, user_name, created_at, expires_at)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(token_hash)
    .bind(purpose.as_str())
    .bind(email)
    .bind(user_name)
    .bind(now_utc)
    .bind(expires_at_utc)
    .execute(conn)
    .await?;
    Ok(())
}

/// 有効なメールトークンを取り出して削除する（1 回限り有効）。
pub async fn take_email_token(
    conn: &mut SqliteConnection,
    purpose: TokenPurpose,
    token_hash: &str,
    now_utc: &str,
) -> sqlx::Result<Option<EmailTokenRecord>> {
    sqlx::query_as(
        "DELETE FROM email_tokens
          WHERE token_hash = ? AND purpose = ? AND expires_at > ?
         RETURNING email, user_name",
    )
    .bind(token_hash)
    .bind(purpose.as_str())
    .bind(now_utc)
    .fetch_optional(conn)
    .await
}

/// 未完了の新規登録（確認待ち）を返す（確認メールの再送用）。
///
/// 期限切れのリンクを再送したいケースに対応するため、有効期限は見ない。
pub async fn find_pending_registration(
    conn: &mut SqliteConnection,
    email: &str,
) -> sqlx::Result<Option<EmailTokenRecord>> {
    sqlx::query_as(
        "SELECT email, user_name FROM email_tokens
          WHERE purpose = 'verify' AND email = ?
          ORDER BY created_at DESC LIMIT 1",
    )
    .bind(email)
    .fetch_optional(conn)
    .await
}
