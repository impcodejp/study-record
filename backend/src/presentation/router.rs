//! URL と HTTP ハンドラーの対応表。

use axum::{
    middleware,
    routing::{get, post, put},
    Router,
};
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

use super::{
    auth::require_requested_with,
    handlers::{auth, exams, health, practices, review},
    state::AppState,
};

/// API のルーターを組み立てる。
pub fn build(state: AppState) -> Router {
    let auth_routes = Router::new()
        .route("/register", post(auth::register))
        .route("/resend-verification", post(auth::resend_verification))
        .route("/verify-email", post(auth::verify_email))
        .route("/forgot-password", post(auth::forgot_password))
        .route("/reset-password", post(auth::reset_password))
        .route("/login", post(auth::login))
        .route("/logout", post(auth::logout))
        .route("/me", get(auth::me))
        .route("/password", post(auth::change_password));

    let exam_routes = Router::new()
        .route("/", get(exams::list_exams).post(exams::create_exam))
        .route("/{exam_id}", put(exams::rename_exam).delete(exams::delete_exam))
        .route("/{exam_id}/goal", put(exams::update_goal))
        .route(
            "/{exam_id}/categories",
            get(exams::list_categories).post(exams::add_category),
        )
        .route(
            "/{exam_id}/categories/{category_id}",
            put(exams::rename_category).delete(exams::delete_category),
        )
        .route("/{exam_id}/category-order", put(exams::reorder_categories))
        .route(
            "/{exam_id}/practices",
            get(practices::list).post(practices::start),
        )
        .route("/{exam_id}/dashboard", get(review::dashboard))
        .route("/{exam_id}/questions", get(review::questions))
        .route("/{exam_id}/history", get(review::history))
        .route("/{exam_id}/notes", get(review::notes))
        .route("/{exam_id}/export", get(review::export_csv));

    let practice_routes = Router::new()
        .route("/{id}", get(practices::get))
        .route("/{id}/answers", post(practices::record_answer))
        .route(
            "/{id}/answers/{answer_id}",
            put(practices::update_answer).delete(practices::delete_answer),
        )
        .route("/{id}/answers/{answer_id}/grade", post(practices::grade))
        .route("/{id}/finish", post(practices::finish))
        .route("/{id}/abort", post(practices::abort));

    let answer_routes = Router::new()
        .route("/{id}/category", put(review::change_category))
        .route("/{id}/memo", put(review::update_memo));

    let api = Router::new()
        .route("/health", get(health::health))
        .route("/active", get(practices::active))
        .route("/exam-templates", get(exams::list_templates))
        .nest("/auth", auth_routes)
        .nest("/exams", exam_routes)
        .nest("/practices", practice_routes)
        .nest("/answers", answer_routes)
        .fallback(health::not_found);

    Router::new()
        .nest("/api", api)
        .layer(middleware::from_fn(require_requested_with))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .with_state(state)
}
