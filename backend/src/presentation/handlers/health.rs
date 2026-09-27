//! 稼働確認の API（F-30）と、存在しない API への応答。

use crate::{domain::error::AppError, presentation::error::ApiError};

/// `GET /api/health` 稼働確認。ログイン不要で `ok` を返す。
pub async fn health() -> &'static str {
    "ok"
}

/// 存在しない API への応答（404、JSON 形式）。
pub async fn not_found() -> ApiError {
    ApiError(AppError::NotFound("API が見つかりません。".into()))
}
