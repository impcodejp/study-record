//! 学習・回答・採点の API（F-01〜F-07、F-11〜F-12）。

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    application::practice_service::{AnswerInput, GradeInput},
    domain::models::{Practice, PracticeSummary},
    presentation::{
        auth::AuthUser,
        error::{ApiJson, ApiResult},
        state::AppState,
    },
};

/// 学習の開始の入力。
#[derive(Deserialize)]
pub struct StartRequest {
    title: String,
}

/// 回答の記録・修正の入力。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerRequest {
    question_number: i64,
    title: String,
    area: String,
    response: String,
    /// 修正時は省略可。
    #[serde(default)]
    elapsed_seconds: i64,
}

impl From<AnswerRequest> for AnswerInput {
    fn from(body: AnswerRequest) -> Self {
        Self {
            title: body.title,
            question_number: body.question_number,
            area: body.area,
            response: body.response,
            elapsed_seconds: body.elapsed_seconds,
        }
    }
}

/// 採点の入力。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GradeRequest {
    correct: bool,
    #[serde(default)]
    correct_answer: Option<String>,
    #[serde(default)]
    note: Option<String>,
}

/// 進行中の学習（無ければ null）。
#[derive(Serialize)]
pub struct OptionalPractice {
    practice: Option<Practice>,
}

/// `GET /api/active` 進行中の学習を取得。
pub async fn active(State(state): State<AppState>, auth: AuthUser) -> ApiResult<Json<Option<Practice>>> {
    Ok(Json(state.practices.get_active(auth.user.id).await?))
}

/// `GET /api/exams/{examId}/practices` 学習履歴一覧。
pub async fn list(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(exam_id): Path<i64>,
) -> ApiResult<Json<Vec<PracticeSummary>>> {
    Ok(Json(state.practices.list_summaries(auth.user.id, exam_id).await?))
}

/// `POST /api/exams/{examId}/practices` 学習の開始。
pub async fn start(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(exam_id): Path<i64>,
    ApiJson(body): ApiJson<StartRequest>,
) -> ApiResult<Json<Practice>> {
    Ok(Json(state.practices.start(auth.user.id, exam_id, &body.title).await?))
}

/// `GET /api/practices/{id}` 学習の詳細。
pub async fn get(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(practice_id): Path<i64>,
) -> ApiResult<Json<Practice>> {
    Ok(Json(state.practices.get(auth.user.id, practice_id).await?))
}

/// `POST /api/practices/{id}/answers` 回答の記録。
pub async fn record_answer(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(practice_id): Path<i64>,
    ApiJson(body): ApiJson<AnswerRequest>,
) -> ApiResult<Json<Practice>> {
    Ok(Json(
        state
            .practices
            .record_answer(auth.user.id, practice_id, body.into())
            .await?,
    ))
}

/// `PUT /api/practices/{id}/answers/{answerId}` 未採点の回答の修正。
pub async fn update_answer(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((practice_id, answer_id)): Path<(i64, i64)>,
    ApiJson(body): ApiJson<AnswerRequest>,
) -> ApiResult<Json<Practice>> {
    Ok(Json(
        state
            .practices
            .update_answer(auth.user.id, practice_id, answer_id, body.into())
            .await?,
    ))
}

/// `DELETE /api/practices/{id}/answers/{answerId}` 未採点の回答の削除。
pub async fn delete_answer(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((practice_id, answer_id)): Path<(i64, i64)>,
) -> ApiResult<Json<Practice>> {
    Ok(Json(
        state
            .practices
            .delete_answer(auth.user.id, practice_id, answer_id)
            .await?,
    ))
}

/// `POST /api/practices/{id}/answers/{answerId}/grade` 採点。
pub async fn grade(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((practice_id, answer_id)): Path<(i64, i64)>,
    ApiJson(body): ApiJson<GradeRequest>,
) -> ApiResult<Json<Practice>> {
    let input = GradeInput {
        correct: body.correct,
        correct_answer: body.correct_answer,
        note: body.note,
    };
    Ok(Json(
        state
            .practices
            .grade(auth.user.id, practice_id, answer_id, input)
            .await?,
    ))
}

/// `POST /api/practices/{id}/finish` 学習の完了。
pub async fn finish(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(practice_id): Path<i64>,
) -> ApiResult<Json<Practice>> {
    Ok(Json(state.practices.finish(auth.user.id, practice_id).await?))
}

/// `POST /api/practices/{id}/abort` 学習の中断。
pub async fn abort(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(practice_id): Path<i64>,
) -> ApiResult<Json<OptionalPractice>> {
    let practice = state.practices.abort(auth.user.id, practice_id).await?;
    Ok(Json(OptionalPractice { practice }))
}
