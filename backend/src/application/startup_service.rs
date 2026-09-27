//! サーバー起動時の処理（F-28 起動時の復旧、カテゴリマスタの補完）。

use sqlx::SqlitePool;
use tracing::info;

use crate::{
    domain::{error::AppResult, time::now_jst_string},
    infrastructure::{
        database::begin_write,
        repositories::{master_repository, practice_repository},
    },
};

/// 起動時の処理を行う。
///
/// 1. 未採点の回答を削除し、進行中の学習を完了扱いにする（F-28）
/// 2. 回答で使われているのにマスタに無いカテゴリ名を、並び順 1000 でマスタへ追加する
///
/// すべてのユーザーのデータが対象。1 つのトランザクションで行う。
pub async fn run(pool: &SqlitePool) -> AppResult<()> {
    let mut tx = begin_write(pool).await?;
    let (deleted_answers, deleted_practices, completed) =
        practice_repository::recover_on_startup(&mut tx, &now_jst_string()).await?;
    let added_categories = master_repository::register_orphan_areas(&mut tx).await?;
    tx.commit().await?;
    info!(
        deleted_answers,
        deleted_practices,
        completed_practices = completed,
        added_categories,
        "起動時の復旧処理を行いました"
    );
    Ok(())
}
