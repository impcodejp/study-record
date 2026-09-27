//! プレゼンテーション層（HTTP API）。
//!
//! HTTP の要求を解析してアプリケーション層を呼び出し、結果を JSON で返す。
//! 業務ルールはここに書かない。

pub mod auth;
pub mod error;
pub mod handlers;
pub mod router;
pub mod state;
