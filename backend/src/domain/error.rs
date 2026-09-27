//! アプリケーション全体で共通に使うエラー型。
//!
//! 業務ルール違反（入力エラー・状態の矛盾など）と内部エラーを区別し、
//! プレゼンテーション層で HTTP ステータスへ変換できるようにする。

use thiserror::Error;

/// アプリケーションのエラー。
///
/// 各バリアントは HTTP ステータスに 1 対 1 で対応する
/// （変換はプレゼンテーション層の責務）。
#[derive(Debug, Error)]
pub enum AppError {
    /// 入力値が業務ルールに違反している（400）。
    #[error("{0}")]
    Validation(String),

    /// ログインしていない、またはセッションが無効（401）。
    #[error("{0}")]
    Unauthorized(String),

    /// 要求が拒否された（CSRF トークン不一致など）（403）。
    #[error("{0}")]
    Forbidden(String),

    /// 対象データが存在しない（404）。
    #[error("{0}")]
    NotFound(String),

    /// データの状態と矛盾する操作（409）。
    #[error("{0}")]
    Conflict(String),

    /// 試行回数の上限を超えた（429）。
    #[error("{0}")]
    TooManyRequests(String),

    /// 設定不足などで機能が使えない（503）。
    #[error("{0}")]
    ServiceUnavailable(String),

    /// 想定外の内部エラー（500）。利用者には詳細を返さない。
    #[error("internal error: {0:#}")]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    /// 「学習データが見つかりません。」の 404 エラーを作る。
    pub fn practice_not_found() -> Self {
        Self::NotFound("学習データが見つかりません。".to_string())
    }
}

/// アプリケーション層・インフラ層で使う Result 型の別名。
pub type AppResult<T> = Result<T, AppError>;
