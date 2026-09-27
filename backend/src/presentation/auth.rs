//! ログイン状態の確認と、CSRF 対策。
//!
//! - セッションは HttpOnly / SameSite=Strict の Cookie で受け渡す
//! - 状態を変える要求（GET / HEAD 以外）には、次を必須とする
//!   - すべての要求：`X-Requested-With` ヘッダー（他サイトのフォームからの送信を防ぐ）
//!   - ログイン後の要求：`X-CSRF-Token` ヘッダーがセッションの CSRF トークンと一致すること

use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, FromRequestParts, Request},
    http::{request::Parts, HeaderMap, Method},
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use tracing::warn;

use super::{error::ApiError, state::AppState};
use crate::domain::{error::AppError, models::User};

/// セッション Cookie の名前。
pub const SESSION_COOKIE: &str = "sr_session";
/// CSRF トークンを送るヘッダー名。
const CSRF_HEADER: &str = "x-csrf-token";
/// 画面からの要求であることを示すヘッダー名。
const REQUESTED_WITH_HEADER: &str = "x-requested-with";

/// ログイン中のユーザー。ハンドラーの引数に書くと、ログインを必須にできる。
#[derive(Debug, Clone)]
pub struct AuthUser {
    /// ユーザー情報。
    pub user: User,
    /// セッショントークン（平文）。
    pub session_token: String,
    /// CSRF トークン。
    pub csrf_token: String,
}

/// 状態を変える HTTP メソッドかどうか。
fn is_unsafe(method: &Method) -> bool {
    !matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let unauthorized = || ApiError(AppError::Unauthorized("ログインしてください。".into()));
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar
            .get(SESSION_COOKIE)
            .map(|c| c.value().to_string())
            .ok_or_else(unauthorized)?;
        let session = state.auth.session(&token).await?.ok_or_else(unauthorized)?;

        if is_unsafe(&parts.method) {
            let sent = parts.headers.get(CSRF_HEADER).and_then(|v| v.to_str().ok());
            if sent != Some(session.csrf_token.as_str()) {
                warn!(user_id = session.user.id, path = %parts.uri.path(), "CSRF トークンが一致しません");
                return Err(ApiError(AppError::Forbidden(
                    "画面の有効期限が切れました。再読み込みしてください。".into(),
                )));
            }
        }
        Ok(Self {
            user: session.user,
            session_token: token,
            csrf_token: session.csrf_token,
        })
    }
}

/// 状態を変える要求に `X-Requested-With` ヘッダーを必須とするミドルウェア。
///
/// 独自ヘッダーはブラウザーの CORS により他サイトから付けられないため、
/// ログイン前の API（ログイン・新規登録など）も含めて CSRF を防げる。
pub async fn require_requested_with(request: Request, next: Next) -> Response {
    if is_unsafe(request.method()) && !request.headers().contains_key(REQUESTED_WITH_HEADER) {
        warn!(path = %request.uri().path(), "X-Requested-With ヘッダーの無い要求を拒否しました");
        return ApiError(AppError::Forbidden("不正な要求です。".into())).into_response();
    }
    next.run(request).await
}

/// セッション Cookie を作る。
pub fn session_cookie(token: String, max_age_seconds: i64, secure: bool) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, token))
        .path("/")
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Strict)
        .max_age(time_duration(max_age_seconds))
        .build()
}

/// セッション Cookie を消すための Cookie を作る。
pub fn removal_cookie(secure: bool) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, ""))
        .path("/")
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Strict)
        .max_age(time_duration(0))
        .build()
}

/// Cookie の Max-Age 用の期間を作る。
fn time_duration(seconds: i64) -> time::Duration {
    time::Duration::seconds(seconds)
}

/// 接続元 IP アドレスを取り出す抽出子（試行回数の制限とログに使う）。
pub struct ClientIp(pub String);

impl FromRequestParts<AppState> for ClientIp {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        if state.http.trust_proxy {
            if let Some(ip) = header_ip(&parts.headers) {
                return Ok(Self(ip));
            }
        }
        let ip = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(addr)| addr.ip().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        Ok(Self(ip))
    }
}

/// nginx が付ける `X-Real-IP` ヘッダーから IP を取り出す。
fn header_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}
