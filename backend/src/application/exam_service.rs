//! 試験マスタとカテゴリマスタのユースケース（F-18〜F-21、試験マスタ）。

use sqlx::SqlitePool;
use tracing::info;

use super::common::ensure_exam;
use crate::{
    domain::{
        error::{AppError, AppResult},
        exam_templates::find_template,
        models::{Category, Exam},
        time::now_jst_string,
        validation::{validate_category_name, validate_daily_goal, validate_exam_date, validate_exam_name},
    },
    infrastructure::{
        database::{begin_write, is_unique_violation},
        repositories::master_repository as repo,
    },
};

/// 試験を作ったときに自動で作るカテゴリの名前。
pub const DEFAULT_CATEGORY_NAME: &str = "未分類";

/// 試験名の重複エラー。
fn duplicate_exam() -> AppError {
    AppError::Conflict("同じ試験名が既にあります。".into())
}

/// カテゴリ名の重複エラー。
fn duplicate_category() -> AppError {
    AppError::Conflict("同じカテゴリ名が既にあります。".into())
}

/// 試験・カテゴリのユースケース。
#[derive(Clone)]
pub struct ExamService {
    pool: SqlitePool,
}

impl ExamService {
    /// サービスを作る。
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    // -----------------------------------------------------------------------
    // 試験
    // -----------------------------------------------------------------------

    /// ユーザーの試験一覧を返す。
    pub async fn list_exams(&self, user_id: i64) -> AppResult<Vec<Exam>> {
        let mut conn = self.pool.acquire().await?;
        Ok(repo::list_exams(&mut conn, user_id).await?)
    }

    /// 試験を登録する。同時に「未分類」カテゴリを作り、すぐに学習を始められるようにする。
    ///
    /// `template_key` を指定すると、テンプレートのカテゴリ（試験の出題分野）も「未分類」の後に作る。
    pub async fn create_exam(
        &self,
        user_id: i64,
        name: &str,
        template_key: Option<&str>,
    ) -> AppResult<Vec<Exam>> {
        let name = validate_exam_name(name)?;
        let template = match template_key.map(str::trim).filter(|k| !k.is_empty()) {
            Some(key) => Some(
                find_template(key)
                    .ok_or_else(|| AppError::Validation("試験のテンプレートが見つかりません。".into()))?,
            ),
            None => None,
        };
        let mut tx = begin_write(&self.pool).await?;
        if repo::exam_name_exists(&mut tx, user_id, &name, None).await? {
            return Err(duplicate_exam());
        }
        let exam_id = match repo::insert_exam(&mut tx, user_id, &name, &now_jst_string()).await {
            Ok(id) => id,
            Err(err) if is_unique_violation(&err) => return Err(duplicate_exam()),
            Err(err) => return Err(err.into()),
        };
        repo::insert_category(&mut tx, exam_id, DEFAULT_CATEGORY_NAME, Some(0)).await?;
        if let Some(template) = template {
            for (index, category) in template.categories.iter().enumerate() {
                repo::insert_category(&mut tx, exam_id, category, Some(index as i64 + 1)).await?;
            }
        }
        tx.commit().await?;
        info!(
            user_id,
            exam_id,
            template = template.map_or("なし", |t| t.key),
            "試験を登録しました"
        );
        self.list_exams(user_id).await
    }

    /// 試験名を変更する。
    pub async fn rename_exam(&self, user_id: i64, exam_id: i64, name: &str) -> AppResult<Vec<Exam>> {
        let name = validate_exam_name(name)?;
        let mut tx = begin_write(&self.pool).await?;
        ensure_exam(&mut tx, user_id, exam_id).await?;
        if repo::exam_name_exists(&mut tx, user_id, &name, Some(exam_id)).await? {
            return Err(duplicate_exam());
        }
        repo::rename_exam(&mut tx, exam_id, &name).await?;
        tx.commit().await?;
        info!(user_id, exam_id, "試験名を変更しました");
        self.list_exams(user_id).await
    }

    /// 試験の学習目標（試験日・1 日の目標問題数）を設定する。どちらも空にすると未設定に戻る。
    pub async fn update_goal(
        &self,
        user_id: i64,
        exam_id: i64,
        exam_date: Option<&str>,
        daily_goal: Option<i64>,
    ) -> AppResult<Vec<Exam>> {
        let exam_date = validate_exam_date(exam_date)?.map(|d| d.format("%Y-%m-%d").to_string());
        let daily_goal = validate_daily_goal(daily_goal)?;
        let mut tx = begin_write(&self.pool).await?;
        ensure_exam(&mut tx, user_id, exam_id).await?;
        repo::update_exam_goal(&mut tx, exam_id, exam_date.as_deref(), daily_goal).await?;
        tx.commit().await?;
        info!(
            user_id,
            exam_id,
            exam_date = exam_date.as_deref().unwrap_or("未設定"),
            daily_goal = daily_goal.unwrap_or(0),
            "試験の学習目標を更新しました"
        );
        self.list_exams(user_id).await
    }

    /// 試験を削除する。学習記録がある試験は、誤操作で履歴を失わないよう削除できない。
    pub async fn delete_exam(&self, user_id: i64, exam_id: i64) -> AppResult<Vec<Exam>> {
        let mut tx = begin_write(&self.pool).await?;
        ensure_exam(&mut tx, user_id, exam_id).await?;
        if repo::count_practices(&mut tx, exam_id).await? > 0 {
            return Err(AppError::Conflict(
                "学習記録がある試験は削除できません。".into(),
            ));
        }
        repo::delete_exam(&mut tx, exam_id).await?;
        tx.commit().await?;
        info!(user_id, exam_id, "試験を削除しました");
        self.list_exams(user_id).await
    }

    // -----------------------------------------------------------------------
    // カテゴリ
    // -----------------------------------------------------------------------

    /// 試験のカテゴリ一覧を返す（F-18）。
    pub async fn list_categories(&self, user_id: i64, exam_id: i64) -> AppResult<Vec<Category>> {
        let mut conn = self.pool.acquire().await?;
        ensure_exam(&mut conn, user_id, exam_id).await?;
        Ok(repo::list_categories(&mut conn, exam_id).await?)
    }

    /// カテゴリを追加する（F-19）。並び順は既存の最大値 + 1。
    pub async fn add_category(
        &self,
        user_id: i64,
        exam_id: i64,
        name: &str,
    ) -> AppResult<Vec<Category>> {
        let name = validate_category_name(name)?;
        let mut tx = begin_write(&self.pool).await?;
        ensure_exam(&mut tx, user_id, exam_id).await?;
        if repo::category_name_exists(&mut tx, exam_id, &name, None).await? {
            return Err(duplicate_category());
        }
        match repo::insert_category(&mut tx, exam_id, &name, None).await {
            Ok(()) => {}
            Err(err) if is_unique_violation(&err) => return Err(duplicate_category()),
            Err(err) => return Err(err.into()),
        }
        tx.commit().await?;
        info!(user_id, exam_id, "カテゴリを追加しました");
        self.list_categories(user_id, exam_id).await
    }

    /// カテゴリ名を変更し、そのカテゴリの既存回答にも反映する（F-20）。
    pub async fn rename_category(
        &self,
        user_id: i64,
        exam_id: i64,
        category_id: i64,
        name: &str,
    ) -> AppResult<Vec<Category>> {
        let name = validate_category_name(name)?;
        let mut tx = begin_write(&self.pool).await?;
        ensure_exam(&mut tx, user_id, exam_id).await?;
        let old_name = repo::find_category_name(&mut tx, exam_id, category_id)
            .await?
            .ok_or_else(category_not_found)?;
        if repo::category_name_exists(&mut tx, exam_id, &name, Some(category_id)).await? {
            return Err(duplicate_category());
        }
        let updated = repo::rename_category(&mut tx, exam_id, category_id, &old_name, &name).await?;
        tx.commit().await?;
        info!(user_id, exam_id, category_id, updated_answers = updated, "カテゴリ名を変更しました");
        self.list_categories(user_id, exam_id).await
    }

    /// カテゴリを削除する（F-21）。回答で使われているカテゴリは削除できない。
    pub async fn delete_category(
        &self,
        user_id: i64,
        exam_id: i64,
        category_id: i64,
    ) -> AppResult<Vec<Category>> {
        let mut tx = begin_write(&self.pool).await?;
        ensure_exam(&mut tx, user_id, exam_id).await?;
        let name = repo::find_category_name(&mut tx, exam_id, category_id)
            .await?
            .ok_or_else(category_not_found)?;
        if repo::count_answers_with_area(&mut tx, exam_id, &name).await? > 0 {
            return Err(AppError::Conflict(
                "回答で使用中のカテゴリは削除できません。先に回答のカテゴリを変更してください。".into(),
            ));
        }
        repo::delete_category(&mut tx, category_id).await?;
        tx.commit().await?;
        info!(user_id, exam_id, category_id, "カテゴリを削除しました");
        self.list_categories(user_id, exam_id).await
    }

    /// カテゴリの並び順を、指定された ID の順に並べ替える。
    ///
    /// 試験のカテゴリをすべて、重複なく指定する必要がある。
    pub async fn reorder_categories(
        &self,
        user_id: i64,
        exam_id: i64,
        category_ids: &[i64],
    ) -> AppResult<Vec<Category>> {
        let mut tx = begin_write(&self.pool).await?;
        ensure_exam(&mut tx, user_id, exam_id).await?;
        let mut current: Vec<i64> = repo::list_categories(&mut tx, exam_id)
            .await?
            .into_iter()
            .map(|c| c.id)
            .collect();
        let mut requested = category_ids.to_vec();
        current.sort_unstable();
        requested.sort_unstable();
        if current != requested {
            return Err(AppError::Validation(
                "並び替えるカテゴリの指定が正しくありません。画面を再読み込みしてください。".into(),
            ));
        }
        for (order, id) in category_ids.iter().enumerate() {
            repo::update_category_order(&mut tx, *id, order as i64).await?;
        }
        tx.commit().await?;
        info!(user_id, exam_id, "カテゴリを並び替えました");
        self.list_categories(user_id, exam_id).await
    }
}

/// カテゴリが見つからないエラー。
fn category_not_found() -> AppError {
    AppError::NotFound("カテゴリが見つかりません。".into())
}
