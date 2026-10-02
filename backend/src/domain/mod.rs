//! ドメイン層。
//!
//! 業務データのモデル・業務ルール（入力検証）・エラー型・日時の取り決めを持つ。
//! 他の層に依存しない。

pub mod error;
pub mod exam_templates;
pub mod models;
pub mod study_plan;
pub mod time;
pub mod validation;
