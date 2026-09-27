//! 資格勉強回答正誤記録アプリの API サーバー。
//!
//! レイヤード設計を採用し、依存の向きは次のとおり（上の層が下の層を使う）。
//!
//! ```text
//! presentation（HTTP API）
//!     ↓
//! application（ユースケース）
//!     ↓
//! infrastructure（DB・メール・設定・ログ） → domain（モデル・業務ルール）
//! ```

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod presentation;

use anyhow::Result;
use sqlx::SqlitePool;

use application::{
    auth_service::AuthService, exam_service::ExamService, practice_service::PracticeService,
    review_service::ReviewService,
};
use infrastructure::{config::AppConfig, mailer::Mailer};
use presentation::state::{AppState, HttpSettings};

/// 設定と DB 接続から、HTTP ハンドラーが使う状態を組み立てる。
pub fn build_state(config: &AppConfig, pool: SqlitePool) -> Result<AppState> {
    let mailer = Mailer::from_config(config.smtp.as_ref())?;
    Ok(AppState {
        auth: AuthService::new(pool.clone(), mailer, config),
        exams: ExamService::new(pool.clone()),
        practices: PracticeService::new(pool.clone()),
        reviews: ReviewService::new(pool),
        http: HttpSettings {
            cookie_secure: config.server.cookie_secure,
            trust_proxy: config.server.trust_proxy,
        },
    })
}
