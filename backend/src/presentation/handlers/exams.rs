//! 試験マスタ・カテゴリマスタの API（F-18〜F-21、試験マスタ）。

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;

use crate::{
    domain::{
        exam_templates::{ExamTemplate, EXAM_TEMPLATES},
        models::{Category, Exam},
    },
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

/// 学習目標の入力。どちらも省略・空にすると未設定に戻す。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalRequest {
    /// 試験日（`YYYY-MM-DD`）。
    #[serde(default)]
    exam_date: Option<String>,
    /// 1 日の目標問題数。
    #[serde(default)]
    daily_goal: Option<i64>,
}

/// `GET /api/exams` 試験の一覧。
pub async fn list_exams(State(state): State<AppState>, auth: AuthUser) -> ApiResult<Json<Vec<Exam>>> {
    Ok(Json(state.exams.list_exams(auth.user.id).await?))
}

/// 試験の登録の入力。
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateExamRequest {
    /// 試験名。
    name: String,
    /// 試験のテンプレートのキー（指定するとカテゴリも作る）。
    #[serde(default)]
    template_key: Option<String>,
}

/// `GET /api/exam-templates` 試験のテンプレートの一覧（ログイン不要。紹介ページでも使う）。
pub async fn list_templates() -> Json<&'static [ExamTemplate]> {
    Json(EXAM_TEMPLATES)
}

/// `POST /api/exams` 試験の登録（テンプレートを指定するとカテゴリも作る）。
pub async fn create_exam(
    State(state): State<AppState>,
    auth: AuthUser,
    ApiJson(body): ApiJson<CreateExamRequest>,
) -> ApiResult<Json<Vec<Exam>>> {
    Ok(Json(
        state
            .exams
            .create_exam(auth.user.id, &body.name, body.template_key.as_deref())
            .await?,
    ))
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

/// `PUT /api/exams/{examId}/goal` 学習目標（試験日・1 日の目標問題数）の設定。
pub async fn update_goal(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(exam_id): Path<i64>,
    ApiJson(body): ApiJson<GoalRequest>,
) -> ApiResult<Json<Vec<Exam>>> {
    Ok(Json(
        state
            .exams
            .update_goal(auth.user.id, exam_id, body.exam_date.as_deref(), body.daily_goal)
            .await?,
    ))
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
