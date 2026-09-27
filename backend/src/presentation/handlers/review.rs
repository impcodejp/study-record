//! 集計・振り返りの API（F-08〜F-10、F-13〜F-17、CSV 出力）。

use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::Deserialize;

use crate::{
    application::review_service::QuestionFilter,
    domain::{
        error::AppError,
        models::{Attempt, Dashboard, QuestionItem},
        time::now_jst,
    },
    presentation::{
        auth::AuthUser,
        error::{ApiError, ApiJson, ApiResult},
        state::AppState,
    },
};

/// 問題一覧の絞り込み条件（クエリ）。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionsQuery {
    /// `review` を指定すると復習一覧（最新が不正解の問題だけ）。
    filter: Option<String>,
    /// 指定するとそのカテゴリの問題だけ。
    category_id: Option<i64>,
}

/// 正誤履歴の対象（クエリ）。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryQuery {
    title: String,
    question_number: i64,
}

/// 回答のカテゴリ変更の入力。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryRequest {
    category_id: i64,
}

/// 回答のメモ更新の入力。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoRequest {
    #[serde(default)]
    correct_answer: Option<String>,
    #[serde(default)]
    note: Option<String>,
}

/// `GET /api/exams/{examId}/dashboard` ダッシュボードの集計。
pub async fn dashboard(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(exam_id): Path<i64>,
) -> ApiResult<Json<Dashboard>> {
    Ok(Json(state.reviews.dashboard(auth.user.id, exam_id).await?))
}

/// `GET /api/exams/{examId}/questions` 問題一覧・復習一覧・カテゴリ別の履歴。
pub async fn questions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(exam_id): Path<i64>,
    Query(query): Query<QuestionsQuery>,
) -> ApiResult<Json<Vec<QuestionItem>>> {
    let filter = match (query.filter.as_deref(), query.category_id) {
        (_, Some(category_id)) => QuestionFilter::Category(category_id),
        (Some("review"), None) => QuestionFilter::Review,
        (None | Some("") | Some("all"), None) => QuestionFilter::All,
        (Some(_), None) => {
            return Err(ApiError(AppError::Validation("絞り込み条件が正しくありません。".into())))
        }
    };
    Ok(Json(state.reviews.questions(auth.user.id, exam_id, filter).await?))
}

/// `GET /api/exams/{examId}/history` 問題ごとの正誤履歴。
pub async fn history(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(exam_id): Path<i64>,
    Query(query): Query<HistoryQuery>,
) -> ApiResult<Json<Vec<Attempt>>> {
    Ok(Json(
        state
            .reviews
            .history(auth.user.id, exam_id, &query.title, query.question_number)
            .await?,
    ))
}

/// `GET /api/exams/{examId}/export` 採点済み回答の CSV 出力。
pub async fn export_csv(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(exam_id): Path<i64>,
) -> ApiResult<impl IntoResponse> {
    let (exam_name, csv) = state.reviews.export_csv(auth.user.id, exam_id).await?;
    let file_name = format!("{}_{}.csv", exam_name, now_jst().format("%Y%m%d"));
    let disposition = format!(
        "attachment; filename=\"export.csv\"; filename*=UTF-8''{}",
        percent_encode(&file_name)
    );
    Ok((
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (header::CONTENT_DISPOSITION, disposition),
        ],
        csv,
    ))
}

/// `PUT /api/answers/{id}/category` 回答のカテゴリ変更（F-17）。
pub async fn change_category(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(answer_id): Path<i64>,
    ApiJson(body): ApiJson<CategoryRequest>,
) -> ApiResult<StatusCode> {
    state
        .reviews
        .change_answer_category(auth.user.id, answer_id, body.category_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `PUT /api/answers/{id}/memo` 採点済み回答の正解・メモの更新。
pub async fn update_memo(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(answer_id): Path<i64>,
    ApiJson(body): ApiJson<MemoRequest>,
) -> ApiResult<StatusCode> {
    state
        .reviews
        .update_answer_memo(
            auth.user.id,
            answer_id,
            body.correct_answer.as_deref(),
            body.note.as_deref(),
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Content-Disposition の `filename*` 用に、英数字と一部記号以外を %XX に変換する（RFC 5987）。
fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}
