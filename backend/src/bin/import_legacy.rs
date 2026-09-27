//! 旧システム（基本情報技術者試験 科目B対策アプリ）の DB を取り込む移行ツール（F-29）。
//!
//! 使い方：
//!
//! ```text
//! import_legacy <旧DBのパス> <取り込み先ユーザーのメールアドレス> [試験名]
//! ```
//!
//! - 試験名を省略すると「基本情報技術者試験 科目B」として取り込む（無ければ作る）
//! - 旧形式（回答に問題名称の列が無い DB）は、学習の問題名称を各回答にコピーする
//! - 旧 DB のカテゴリマスタ（無い場合は旧システムの初期カテゴリ）と、回答にあるカテゴリ名を登録する
//! - 採点前の回答は取り込まない。完了していない学習は完了扱いで取り込む
//! - 取り込みは 1 つのトランザクションで行い、失敗したら何も変更しない
//!
//! 設定（DB・ログの場所）は API サーバーと同じ config/app.ini を使う。

use std::{path::PathBuf, str::FromStr};

use anyhow::{bail, Context, Result};
use sqlx::{sqlite::SqliteConnectOptions, Connection, SqliteConnection};
use tracing::{error, info};

use study_record_server::{
    domain::time::now_jst_string,
    infrastructure::{
        config::AppConfig,
        database,
        logging,
        repositories::{master_repository, user_repository},
    },
};

/// 試験名を省略したときの取り込み先。
const DEFAULT_EXAM_NAME: &str = "基本情報技術者試験 科目B";

/// 旧システムの初期カテゴリ（旧 DB にカテゴリマスタが無い場合に使う）。
const LEGACY_DEFAULT_CATEGORIES: [&str; 5] = [
    "アルゴリズム",
    "処理の基本要素",
    "データ構造",
    "諸分野への適用",
    "セキュリティ",
];

/// コマンドライン引数。
struct Args {
    legacy_path: PathBuf,
    email: String,
    exam_name: String,
}

fn main() {
    let args = match parse_args() {
        Ok(args) => args,
        Err(err) => {
            eprintln!("{err:#}");
            eprintln!("使い方: import_legacy <旧DBのパス> <メールアドレス> [試験名]");
            std::process::exit(2);
        }
    };
    let config = match AppConfig::load() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("設定の読み込みに失敗しました: {err:#}");
            std::process::exit(1);
        }
    };
    if let Err(err) = logging::init(&config.log) {
        eprintln!("ログ出力の初期化に失敗しました: {err:#}");
        std::process::exit(1);
    }
    let runtime = tokio::runtime::Runtime::new().expect("非同期ランタイムを起動できません");
    match runtime.block_on(run(&config, &args)) {
        Ok(()) => info!("旧 DB の取り込みが完了しました"),
        Err(err) => {
            error!(error = %format!("{err:#}"), "旧 DB の取り込みに失敗しました");
            std::process::exit(1);
        }
    }
}

/// 引数を解析する。
fn parse_args() -> Result<Args> {
    let mut args = std::env::args().skip(1);
    let legacy_path = args.next().context("旧 DB のパスを指定してください")?;
    let email = args.next().context("取り込み先のメールアドレスを指定してください")?;
    let exam_name = args.next().unwrap_or_else(|| DEFAULT_EXAM_NAME.to_string());
    Ok(Args {
        legacy_path: PathBuf::from(legacy_path),
        email,
        exam_name,
    })
}

/// 旧 DB の学習 1 件。
#[derive(sqlx::FromRow)]
struct LegacyPractice {
    id: i64,
    title: String,
    created_at: String,
    completed_at: Option<String>,
}

/// 旧 DB の回答 1 件。
#[derive(sqlx::FromRow)]
struct LegacyAnswer {
    title: String,
    question_number: i64,
    area: String,
    response: String,
    elapsed_seconds: i64,
    answered_at: String,
    correct: bool,
}

/// 取り込みを行う。
async fn run(config: &AppConfig, args: &Args) -> Result<()> {
    if !args.legacy_path.exists() {
        bail!("旧 DB が見つかりません: {}", args.legacy_path.display());
    }
    let legacy_options = SqliteConnectOptions::from_str(&format!("sqlite://{}", args.legacy_path.display()))?
        .read_only(true);
    let mut legacy = SqliteConnection::connect_with(&legacy_options)
        .await
        .context("旧 DB を開けません")?;

    let pool = database::connect(&config.database.path).await?;
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;

    let user = user_repository::find_by_email(&mut tx, &args.email)
        .await?
        .with_context(|| format!("ユーザー {} が見つかりません。先にアプリで登録してください", args.email))?;

    // 取り込み先の試験を用意する。
    let exam_id = match find_exam_id(&mut tx, user.id, &args.exam_name).await? {
        Some(id) => id,
        None => master_repository::insert_exam(&mut tx, user.id, &args.exam_name, &now_jst_string()).await?,
    };

    // 進行中の学習があると、完了扱いで取り込む学習と衝突しないが、念のため止める。
    let active: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM practices WHERE user_id = ? AND completed_at IS NULL",
    )
    .bind(user.id)
    .fetch_one(&mut *tx)
    .await?;
    if active > 0 {
        bail!("取り込み先のユーザーに進行中の学習があります。完了または中断してから実行してください");
    }

    // カテゴリマスタを登録する。
    let categories = legacy_categories(&mut legacy).await?;
    for (order, name) in categories.iter().enumerate() {
        if master_repository::find_category_by_name(&mut tx, exam_id, name).await?.is_none() {
            master_repository::insert_category(&mut tx, exam_id, name, Some(order as i64)).await?;
        }
    }

    // 学習と回答を取り込む。
    let has_answer_title = has_column(&mut legacy, "answers", "title").await?;
    let practices: Vec<LegacyPractice> =
        sqlx::query_as("SELECT id, title, created_at, completed_at FROM practices ORDER BY id")
            .fetch_all(&mut legacy)
            .await?;
    let now = now_jst_string();
    let (mut imported_practices, mut imported_answers) = (0, 0);
    for practice in &practices {
        // 旧形式は回答に問題名称が無いため、学習の問題名称をコピーする。
        let title_expr = if has_answer_title { "title" } else { "? AS title" };
        let sql = format!(
            "SELECT {title_expr}, question_number, area, response, elapsed_seconds, answered_at, correct
               FROM answers WHERE practice_id = ? AND correct IS NOT NULL ORDER BY id"
        );
        let mut query = sqlx::query_as::<_, LegacyAnswer>(&sql);
        if !has_answer_title {
            query = query.bind(&practice.title);
        }
        let answers = query.bind(practice.id).fetch_all(&mut legacy).await?;
        if answers.is_empty() {
            continue;
        }
        let practice_id: i64 = sqlx::query_scalar(
            "INSERT INTO practices (user_id, exam_id, title, created_at, completed_at)
             VALUES (?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(user.id)
        .bind(exam_id)
        .bind(&practice.title)
        .bind(&practice.created_at)
        .bind(practice.completed_at.as_deref().unwrap_or(&now))
        .fetch_one(&mut *tx)
        .await?;
        for answer in &answers {
            sqlx::query(
                "INSERT OR IGNORE INTO answers
                   (practice_id, title, question_number, area, response, elapsed_seconds, answered_at, correct)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(practice_id)
            .bind(&answer.title)
            .bind(answer.question_number)
            .bind(&answer.area)
            .bind(&answer.response)
            .bind(answer.elapsed_seconds.max(0))
            .bind(&answer.answered_at)
            .bind(answer.correct)
            .execute(&mut *tx)
            .await?;
            imported_answers += 1;
        }
        imported_practices += 1;
    }

    // 回答にあってマスタに無いカテゴリ名を登録する。
    let added = master_repository::register_orphan_areas(&mut tx).await?;
    tx.commit().await?;
    info!(
        user_id = user.id,
        exam_id,
        imported_practices,
        imported_answers,
        added_categories = added,
        "旧 DB を取り込みました"
    );
    println!("取り込み完了: 学習 {imported_practices} 件 / 回答 {imported_answers} 件");
    Ok(())
}

/// ユーザーの試験を名前で探す。
async fn find_exam_id(conn: &mut SqliteConnection, user_id: i64, name: &str) -> Result<Option<i64>> {
    Ok(sqlx::query_scalar("SELECT id FROM exams WHERE user_id = ? AND name = ?")
        .bind(user_id)
        .bind(name)
        .fetch_optional(conn)
        .await?)
}

/// 旧 DB のカテゴリ名を並び順で返す。カテゴリマスタが無ければ旧システムの初期カテゴリ。
async fn legacy_categories(legacy: &mut SqliteConnection) -> Result<Vec<String>> {
    let exists: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'categories'",
    )
    .fetch_one(&mut *legacy)
    .await?;
    if exists == 0 {
        return Ok(LEGACY_DEFAULT_CATEGORIES.iter().map(|s| s.to_string()).collect());
    }
    Ok(sqlx::query_scalar("SELECT name FROM categories ORDER BY sort_order, id")
        .fetch_all(legacy)
        .await?)
}

/// テーブルに列があるかを調べる。
async fn has_column(conn: &mut SqliteConnection, table: &str, column: &str) -> Result<bool> {
    // テーブル名はこのファイル内の固定値だけを渡す（外部入力ではない）。
    let names: Vec<String> = sqlx::query_scalar(&format!("SELECT name FROM pragma_table_info('{table}')"))
        .fetch_all(conn)
        .await?;
    Ok(names.iter().any(|name| name == column))
}
