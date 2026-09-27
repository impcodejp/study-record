//! API サーバーの起動処理。
//!
//! 1. 設定を読み込む（config/app.ini と環境変数）
//! 2. ログ出力（SystemRunningLog.log）を初期化する
//! 3. DB に接続し、マイグレーションと起動時の復旧処理を行う
//! 4. HTTP サーバーを起動する（Ctrl+C / 終了シグナルで停止）

use std::{io::Write, net::SocketAddr, time::Duration};

use anyhow::{Context, Result};
use tokio::net::TcpListener;
use tracing::{error, info, warn};

use study_record_server::{
    application::startup_service,
    build_state,
    infrastructure::{config::AppConfig, database, logging},
    presentation::router,
};

/// 期限切れのセッション・トークンを掃除する間隔。
const PURGE_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// ログ出力の初期化前に失敗したときに書き込むファイル。
const FALLBACK_LOG_PATH: &str = "SystemRunningLog.log";

fn main() {
    let config = match AppConfig::load() {
        Ok(config) => config,
        Err(err) => {
            write_fallback_log(&format!("設定の読み込みに失敗しました: {err:#}"));
            std::process::exit(1);
        }
    };
    if let Err(err) = logging::init(&config.log) {
        write_fallback_log(&format!("ログ出力の初期化に失敗しました: {err:#}"));
        std::process::exit(1);
    }

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(err) => {
            error!(error = %err, "非同期ランタイムを起動できませんでした");
            std::process::exit(1);
        }
    };
    if let Err(err) = runtime.block_on(run(config)) {
        error!(error = %format!("{err:#}"), "サーバーの起動に失敗しました");
        std::process::exit(1);
    }
    info!("サーバーを停止しました");
}

/// サーバーを起動し、停止の合図まで動かす。
async fn run(config: AppConfig) -> Result<()> {
    info!(version = env!("CARGO_PKG_VERSION"), "サーバーを起動します");
    if config.smtp.is_none() {
        warn!("SMTP が設定されていないため、新規登録・パスワード再設定のメールは使えません");
    }

    let pool = database::connect(&config.database.path).await?;
    info!(path = %config.database.path.display(), "DB に接続しました");
    startup_service::run(&pool)
        .await
        .map_err(|err| anyhow::anyhow!("起動時の復旧処理に失敗しました: {err}"))?;

    let state = build_state(&config, pool.clone())?;
    spawn_purge_task(state.auth.clone());

    let app = router::build(state);
    let listener = TcpListener::bind(&config.server.bind)
        .await
        .with_context(|| format!("{} で待ち受けできません", config.server.bind))?;
    info!(bind = %config.server.bind, "HTTP サーバーを起動しました");
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("HTTP サーバーが異常終了しました")?;
    pool.close().await;
    Ok(())
}

/// 期限切れのセッションとメールトークンを定期的に削除する。
fn spawn_purge_task(auth: study_record_server::application::auth_service::AuthService) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(PURGE_INTERVAL);
        loop {
            interval.tick().await;
            match auth.purge_expired().await {
                Ok(0) => {}
                Ok(deleted) => info!(deleted, "期限切れのセッション・トークンを削除しました"),
                Err(err) => error!(error = %err, "期限切れのセッション・トークンの削除に失敗しました"),
            }
        }
    });
}

/// Ctrl+C（Windows / Linux）または SIGTERM（Linux）を待つ。
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            error!(error = %err, "Ctrl+C の待ち受けに失敗しました");
        }
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(err) => error!(error = %err, "SIGTERM の待ち受けに失敗しました"),
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    info!("停止の合図を受け取りました。処理中の要求を終えてから停止します");
}

/// ログ出力を初期化できないときに、標準エラー出力と既定のログファイルへ書き込む。
fn write_fallback_log(message: &str) {
    eprintln!("{message}");
    let line = format!(
        "{} ERROR {}\n",
        chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ"),
        message
    );
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(FALLBACK_LOG_PATH)
    {
        let _ = file.write_all(line.as_bytes());
    }
}
