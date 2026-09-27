//! 管理者がユーザーを直接登録するツール。
//!
//! メール送信（SMTP）を用意できない環境や、最初の利用者を作るときに使う。
//!
//! 使い方：
//!
//! ```text
//! create_user <メールアドレス> <ユーザー名>
//! ```
//!
//! パスワードは標準入力から 1 行で読み込む（コマンドライン引数に書くと履歴に残るため）。
//! 環境変数 `APP_NEW_USER_PASSWORD` があればそちらを使う。
//! 設定（DB・ログの場所）は API サーバーと同じ config/app.ini を使う。

use std::io::{self, BufRead, Write};

use anyhow::{bail, Context, Result};
use tracing::{error, info};

use study_record_server::{
    domain::{
        time::now_utc_string,
        validation::{validate_email, validate_password, validate_user_name},
    },
    infrastructure::{
        config::AppConfig, database, logging, repositories::user_repository, security::hash_password,
        security::mask_email,
    },
};

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(email), Some(name)) = (args.next(), args.next()) else {
        eprintln!("使い方: create_user <メールアドレス> <ユーザー名>");
        std::process::exit(2);
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
    if let Err(err) = runtime.block_on(run(&config, &email, &name)) {
        error!(error = %format!("{err:#}"), "ユーザーの登録に失敗しました");
        std::process::exit(1);
    }
}

/// 入力を検証してユーザーを登録する。
async fn run(config: &AppConfig, email: &str, name: &str) -> Result<()> {
    let email = validate_email(email).map_err(|e| anyhow::anyhow!("{e}"))?;
    let name = validate_user_name(name).map_err(|e| anyhow::anyhow!("{e}"))?;
    let password = read_password()?;
    validate_password(&password).map_err(|e| anyhow::anyhow!("{e}"))?;

    let pool = database::connect(&config.database.path).await?;
    let mut conn = pool.acquire().await?;
    if user_repository::find_by_email(&mut conn, &email).await?.is_some() {
        bail!("このメールアドレスはすでに登録されています");
    }
    let hash = hash_password(&password).map_err(|e| anyhow::anyhow!("{e}"))?;
    let user_id = user_repository::insert(&mut conn, &name, &email, &hash, &now_utc_string()).await?;
    info!(user_id, email = %mask_email(&email), "管理ツールでユーザーを登録しました");
    println!("ユーザーを登録しました（ID: {user_id}）");
    Ok(())
}

/// パスワードを環境変数または標準入力から読み込む。
fn read_password() -> Result<String> {
    if let Ok(password) = std::env::var("APP_NEW_USER_PASSWORD") {
        return Ok(password);
    }
    print!("パスワード（8文字以上）: ");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin()
        .lock()
        .read_line(&mut line)
        .context("パスワードを読み込めません")?;
    Ok(line.trim_end_matches(['\r', '\n']).to_string())
}
