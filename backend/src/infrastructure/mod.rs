//! インフラストラクチャ層。
//!
//! 設定の読み込み・ログ出力・DB アクセス・メール送信・暗号処理など、
//! 外部の仕組みに依存する処理をまとめる。

pub mod config;
pub mod database;
pub mod logging;
pub mod mailer;
pub mod repositories;
pub mod security;
