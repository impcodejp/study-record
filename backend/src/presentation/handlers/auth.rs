//! アカウント関連の API（F-22〜F-27）。

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use axum_extra::extract::cookie::CookieJar;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    domain::models::User,
    presentation::{
        auth::{removal_cookie, session_cookie, AuthUser, ClientIp, SESSION_COOKIE},
        error::{ApiJson, ApiResult},
        state::AppState,
    },
};

/// 新規登録の入力。
#[derive(Deserialize)]
pub struct RegisterRequest {
    name: String,
    email: String,
}

/// メールアドレスだけの入力（再送・再設定メール）。
#[derive(Deserialize)]
pub struct EmailRequest {
    email: String,
}

/// トークンとパスワードの入力（登録完了・パスワード再設定）。
#[derive(Deserialize)]
pub struct TokenPasswordRequest {
    token: String,
    password: String,
}

/// ログインの入力。
#[derive(Deserialize)]
pub struct LoginRequest {
    email: String,
    password: String,
}

/// パスワード変更の入力。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordRequest {
    current_password: String,
    new_password: String,
}

/// ログイン中のユーザーと CSRF トークン。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResponse {
    user: User,
    csrf_token: String,
}

/// メール送信系の API の共通応答（登録状況によらず同じ内容を返す）。
fn mail_accepted() -> impl IntoResponse {
    (
        StatusCode::ACCEPTED,
        Json(json!({
            "message": "入力されたメールアドレス宛てに案内を送信しました。届かない場合は迷惑メールフォルダも確認してください。"
        })),
    )
}

/// `POST /api/auth/register` 新規登録（確認メールの送信）。
pub async fn register(
    State(state): State<AppState>,
    ApiJson(body): ApiJson<RegisterRequest>,
) -> ApiResult<impl IntoResponse> {
    state.auth.register(&body.name, &body.email).await?;
    Ok(mail_accepted())
}

/// `POST /api/auth/resend-verification` 確認メールの再送。
pub async fn resend_verification(
    State(state): State<AppState>,
    ApiJson(body): ApiJson<EmailRequest>,
) -> ApiResult<impl IntoResponse> {
    state.auth.resend_verification(&body.email).await?;
    Ok(mail_accepted())
}

/// `POST /api/auth/verify-email` 確認リンクによる登録の完了。
pub async fn verify_email(
    State(state): State<AppState>,
    ApiJson(body): ApiJson<TokenPasswordRequest>,
) -> ApiResult<Json<User>> {
    Ok(Json(state.auth.verify_email(&body.token, &body.password).await?))
}

/// `POST /api/auth/forgot-password` 再設定メールの送信。
pub async fn forgot_password(
    State(state): State<AppState>,
    ApiJson(body): ApiJson<EmailRequest>,
) -> ApiResult<impl IntoResponse> {
    state.auth.forgot_password(&body.email).await?;
    Ok(mail_accepted())
}

/// `POST /api/auth/reset-password` 再設定リンクによるパスワード設定。
pub async fn reset_password(
    State(state): State<AppState>,
    ApiJson(body): ApiJson<TokenPasswordRequest>,
) -> ApiResult<StatusCode> {
    state.auth.reset_password(&body.token, &body.password).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/auth/login` ログイン。セッション Cookie を発行する。
pub async fn login(
    State(state): State<AppState>,
    ClientIp(ip): ClientIp,
    jar: CookieJar,
    ApiJson(body): ApiJson<LoginRequest>,
) -> ApiResult<impl IntoResponse> {
    let result = state.auth.login(&body.email, &body.password, &ip).await?;
    let jar = jar.add(session_cookie(
        result.session_token,
        result.max_age_seconds,
        state.http.cookie_secure,
    ));
    Ok((
        jar,
        Json(SessionResponse {
            user: result.user,
            csrf_token: result.csrf_token,
        }),
    ))
}

/// `POST /api/auth/logout` ログアウト。
///
/// 未ログインでもエラーにせず、Cookie を消して終わる。
pub async fn logout(State(state): State<AppState>, jar: CookieJar) -> ApiResult<impl IntoResponse> {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        state.auth.logout(cookie.value()).await?;
    }
    let jar = jar.add(removal_cookie(state.http.cookie_secure));
    Ok((jar, StatusCode::NO_CONTENT))
}

/// `GET /api/auth/me` ログインユーザー情報。
pub async fn me(auth: AuthUser) -> Json<SessionResponse> {
    Json(SessionResponse {
        user: auth.user,
        csrf_token: auth.csrf_token,
    })
}

/// `POST /api/auth/password` パスワード変更。
pub async fn change_password(
    State(state): State<AppState>,
    auth: AuthUser,
    ApiJson(body): ApiJson<ChangePasswordRequest>,
) -> ApiResult<StatusCode> {
    state
        .auth
        .change_password(
            auth.user.id,
            &auth.session_token,
            &body.current_password,
            &body.new_password,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
