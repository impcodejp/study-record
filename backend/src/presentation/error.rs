//! エラーを HTTP レスポンス（`{ "error": "メッセージ" }`）に変換する。

use axum::{
    extract::{rejection::JsonRejection, FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::de::DeserializeOwned;
use serde_json::json;
use tracing::{error, warn};

use crate::domain::error::AppError;

/// 内部エラー時に利用者へ返すメッセージ（詳細はログにだけ出す）。
const INTERNAL_ERROR_MESSAGE: &str = "システムエラーが発生しました。時間をおいて再度お試しください。";

/// HTTP ハンドラーが返すエラー。
pub struct ApiError(pub AppError);

impl From<AppError> for ApiError {
    fn from(err: AppError) -> Self {
        Self(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self.0 {
            AppError::Validation(m) => (StatusCode::BAD_REQUEST, m),
            AppError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, m),
            AppError::Forbidden(m) => (StatusCode::FORBIDDEN, m),
            AppError::NotFound(m) => (StatusCode::NOT_FOUND, m),
            AppError::Conflict(m) => (StatusCode::CONFLICT, m),
            AppError::TooManyRequests(m) => (StatusCode::TOO_MANY_REQUESTS, m),
            AppError::ServiceUnavailable(m) => {
                warn!(message = %m, "利用できない機能が要求されました");
                (StatusCode::SERVICE_UNAVAILABLE, m)
            }
            AppError::Internal(err) => {
                error!(error = %format!("{err:#}"), "内部エラーが発生しました");
                (StatusCode::INTERNAL_SERVER_ERROR, INTERNAL_ERROR_MESSAGE.to_string())
            }
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

/// ハンドラーの戻り値の型の別名。
pub type ApiResult<T> = Result<T, ApiError>;

/// JSON の本文を受け取る抽出子。
///
/// 標準の `Json` は解析エラーをプレーンテキストで返すため、
/// 他のエラーと同じ `{ "error": ... }` 形式にそろえる。
pub struct ApiJson<T>(pub T);

impl<S, T> FromRequest<S> for ApiJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(Self(value)),
            Err(rejection) => {
                warn!(detail = %rejection.body_text(), "リクエストの本文を解析できませんでした");
                let message = match rejection {
                    JsonRejection::MissingJsonContentType(_) => "JSON 形式で送信してください。",
                    _ => "入力内容の形式が正しくありません。",
                };
                Err(ApiError(AppError::Validation(message.into())))
            }
        }
    }
}
