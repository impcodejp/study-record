//! 試験マスタ・カテゴリマスタの API（F-18〜F-21、試験マスタ）。

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;

use crate::{
    domain::models::{Category, Exam},
    presentation::{
        auth::AuthUser,
        error::{ApiJson, ApiResult},
        state::AppState,
    },
};

/// 名前だけの入力（試験・カテゴリの追加と名称変更）。
#[derive(Deserialize)]
pub struct NameRequest {
    name: String,
}

/// カテゴリの並び替えの入力。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReorderRequest {
    category_ids: Vec<i64>,
}

/// `GET /api/exams` 試験の一覧。
pub async fn list_exams(State(state): State<AppState>, auth: AuthUser) -> ApiResult<Json<Vec<Exam>>> {
    Ok(Json(state.exams.list_exams(auth.user.id).await?))
}

/// `POST /api/exams` 試験の登録。
pub async fn create_exam(
    State(state): State<AppState>,
    auth: AuthUser,
    ApiJson(body): ApiJson<NameRequest>,
) -> ApiResult<Json<Vec<Exam>>> {
    Ok(Json(state.exams.create_exam(auth.user.id, &body.name).await?))
}

/// `PUT /api/exams/{examId}` 試験名の変更。
pub async fn rename_exam(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(exam_id): Path<i64>,
    ApiJson(body): ApiJson<NameRequest>,
) -> ApiResult<Json<Vec<Exam>>> {
    Ok(Json(state.exams.rename_exam(auth.user.id, exam_id, &body.name).await?))
}

/// `DELETE /api/exams/{examId}` 試験の削除。
pub async fn delete_exam(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(exam_id): Path<i64>,
) -> ApiResult<Json<Vec<Exam>>> {
    Ok(Json(state.exams.delete_exam(auth.user.id, exam_id).await?))
}

/// `GET /api/exams/{examId}/categories` カテゴリの一覧。
pub async fn list_categories(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(exam_id): Path<i64>,
) -> ApiResult<Json<Vec<Category>>> {
    Ok(Json(state.exams.list_categories(auth.user.id, exam_id).await?))
}

/// `POST /api/exams/{examId}/categories` カテゴリの追加。
pub async fn add_category(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(exam_id): Path<i64>,
    ApiJson(body): ApiJson<NameRequest>,
) -> ApiResult<Json<Vec<Category>>> {
    Ok(Json(state.exams.add_category(auth.user.id, exam_id, &body.name).await?))
}

/// `PUT /api/exams/{examId}/categories/{categoryId}` カテゴリの名称変更。
pub async fn rename_category(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((exam_id, category_id)): Path<(i64, i64)>,
    ApiJson(body): ApiJson<NameRequest>,
) -> ApiResult<Json<Vec<Category>>> {
    Ok(Json(
        state
            .exams
            .rename_category(auth.user.id, exam_id, category_id, &body.name)
            .await?,
    ))
}

/// `DELETE /api/exams/{examId}/categories/{categoryId}` カテゴリの削除。
pub async fn delete_category(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((exam_id, category_id)): Path<(i64, i64)>,
) -> ApiResult<Json<Vec<Category>>> {
    Ok(Json(
        state.exams.delete_category(auth.user.id, exam_id, category_id).await?,
    ))
}

/// `PUT /api/exams/{examId}/category-order` カテゴリの並び替え。
pub async fn reorder_categories(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(exam_id): Path<i64>,
    ApiJson(body): ApiJson<ReorderRequest>,
) -> ApiResult<Json<Vec<Category>>> {
    Ok(Json(
        state
            .exams
            .reorder_categories(auth.user.id, exam_id, &body.category_ids)
            .await?,
    ))
}
