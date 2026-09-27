//! HTTP ハンドラーで共有する状態。

use crate::application::{
    auth_service::AuthService, exam_service::ExamService, practice_service::PracticeService,
    review_service::ReviewService,
};

/// Cookie・プロキシに関する HTTP 層の設定。
#[derive(Clone, Debug)]
pub struct HttpSettings {
    /// Cookie に Secure 属性を付けるか。
    pub cookie_secure: bool,
    /// リバースプロキシの `X-Real-IP` を信頼するか。
    pub trust_proxy: bool,
}

/// ルーターに渡すアプリケーションの状態。
#[derive(Clone)]
pub struct AppState {
    /// アカウント。
    pub auth: AuthService,
    /// 試験・カテゴリ。
    pub exams: ExamService,
    /// 学習・回答・採点。
    pub practices: PracticeService,
    /// 集計・振り返り。
    pub reviews: ReviewService,
    /// HTTP 層の設定。
    pub http: HttpSettings,
}
