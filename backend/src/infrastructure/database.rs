//! データベース（SQLite）への接続とスキーマ管理。
//!
//! スキーマのバージョンは SQLite の `PRAGMA user_version` で管理し、
//! 起動時に未適用のマイグレーションを順に適用する（F-29 データ移行）。

use std::{path::Path, str::FromStr, time::Duration};

use anyhow::{Context, Result};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    Sqlite, SqlitePool, Transaction,
};
use tracing::info;

use crate::domain::error::AppError;

/// 書き込みトランザクションの型の別名。
pub type Tx = Transaction<'static, Sqlite>;

/// マイグレーション（インデックス + 1 がスキーマのバージョン）。
///
/// 一度リリースしたマイグレーションは書き換えず、変更は新しい要素として追加する。
const MIGRATIONS: &[&str] = &[
    // v1: 初期スキーマ
    r#"
    CREATE TABLE users (
        id            INTEGER PRIMARY KEY,
        name          TEXT NOT NULL,
        email         TEXT NOT NULL COLLATE NOCASE UNIQUE,
        password_hash TEXT NOT NULL,
        created_at    TEXT NOT NULL,
        updated_at    TEXT NOT NULL
    );

    -- ログインセッション（トークンはハッシュ値だけを保存する。日時は UTC）
    CREATE TABLE sessions (
        token_hash TEXT PRIMARY KEY,
        user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        csrf_token TEXT NOT NULL,
        created_at TEXT NOT NULL,
        expires_at TEXT NOT NULL
    );
    CREATE INDEX ix_sessions_user ON sessions(user_id);

    -- メールで送るワンタイムトークン（登録確認・パスワード再設定。日時は UTC）
    CREATE TABLE email_tokens (
        token_hash TEXT PRIMARY KEY,
        purpose    TEXT NOT NULL CHECK (purpose IN ('verify', 'reset')),
        email      TEXT NOT NULL COLLATE NOCASE,
        user_name  TEXT,
        created_at TEXT NOT NULL,
        expires_at TEXT NOT NULL
    );
    CREATE INDEX ix_email_tokens_email ON email_tokens(purpose, email);

    -- 試験マスタ（ユーザーごと）
    CREATE TABLE exams (
        id         INTEGER PRIMARY KEY,
        user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        name       TEXT NOT NULL COLLATE NOCASE,
        sort_order INTEGER NOT NULL,
        created_at TEXT NOT NULL,
        UNIQUE (user_id, name)
    );

    -- カテゴリマスタ（試験ごと）
    CREATE TABLE categories (
        id         INTEGER PRIMARY KEY,
        exam_id    INTEGER NOT NULL REFERENCES exams(id) ON DELETE CASCADE,
        name       TEXT NOT NULL COLLATE NOCASE,
        sort_order INTEGER NOT NULL,
        UNIQUE (exam_id, name)
    );

    -- 学習（created_at / completed_at は日本時間で保存）
    CREATE TABLE practices (
        id           INTEGER PRIMARY KEY,
        user_id      INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
        exam_id      INTEGER NOT NULL REFERENCES exams(id) ON DELETE CASCADE,
        title        TEXT NOT NULL,
        created_at   TEXT NOT NULL,
        completed_at TEXT
    );
    -- 進行中の学習はユーザーごとに最大 1 件
    CREATE UNIQUE INDEX ux_practices_active ON practices(user_id) WHERE completed_at IS NULL;
    CREATE INDEX ix_practices_exam ON practices(exam_id, id);

    -- 回答（answered_at は UTC で保存）
    CREATE TABLE answers (
        id              INTEGER PRIMARY KEY,
        practice_id     INTEGER NOT NULL REFERENCES practices(id) ON DELETE CASCADE,
        title           TEXT NOT NULL,
        question_number INTEGER NOT NULL CHECK (question_number >= 1),
        area            TEXT NOT NULL,
        response        TEXT NOT NULL,
        elapsed_seconds INTEGER NOT NULL CHECK (elapsed_seconds >= 0),
        answered_at     TEXT NOT NULL,
        correct         INTEGER CHECK (correct IN (0, 1)),
        correct_answer  TEXT,
        note            TEXT,
        UNIQUE (practice_id, title, question_number)
    );
    CREATE INDEX ix_answers_question ON answers(title, question_number);
    "#,
    // v2: 試験ごとの学習目標（試験日・1 日の目標問題数。どちらも任意）
    r#"
    ALTER TABLE exams ADD COLUMN exam_date TEXT;
    ALTER TABLE exams ADD COLUMN daily_goal INTEGER CHECK (daily_goal BETWEEN 1 AND 1000);
    "#,
];

/// SQLite に接続し、マイグレーションを適用したコネクションプールを返す。
pub async fn connect(path: &Path) -> Result<SqlitePool> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("DB フォルダ {} を作れません", parent.display()))?;
        }
    }
    let url = format!("sqlite://{}", path.display());
    let options = SqliteConnectOptions::from_str(&url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(options)
        .await
        .with_context(|| format!("DB {} に接続できません", path.display()))?;
    migrate(&pool).await?;
    Ok(pool)
}

/// 未適用のマイグレーションを適用する。
async fn migrate(pool: &SqlitePool) -> Result<()> {
    let current: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(pool)
        .await?;
    let latest = MIGRATIONS.len() as i64;
    if current > latest {
        anyhow::bail!(
            "DB のバージョン({current})がアプリの対応バージョン({latest})より新しいため起動できません"
        );
    }
    for (index, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let version = index as i64 + 1;
        let mut tx = pool.begin().await?;
        sqlx::raw_sql(sql).execute(&mut *tx).await.with_context(|| {
            format!("DB のマイグレーション v{version} に失敗しました")
        })?;
        // PRAGMA はパラメータを使えないため、数値を直接埋め込む（外部入力ではない）。
        sqlx::raw_sql(&format!("PRAGMA user_version = {version}"))
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        info!(version, "DB のマイグレーションを適用しました");
    }
    Ok(())
}

/// 書き込み用のトランザクションを開始する。
///
/// SQLite では読み取りから書き込みへの昇格時にロック競合で失敗しやすいため、
/// 最初から書き込みロックを取る `BEGIN IMMEDIATE` を使う。
pub async fn begin_write(pool: &SqlitePool) -> Result<Tx, AppError> {
    Ok(pool.begin_with("BEGIN IMMEDIATE").await?)
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        Self::Internal(anyhow::Error::new(err))
    }
}

/// SQLite の一意制約違反かどうかを判定する。
pub fn is_unique_violation(err: &sqlx::Error) -> bool {
    matches!(err, sqlx::Error::Database(db) if db.is_unique_violation())
}
