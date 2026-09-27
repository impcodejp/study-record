//! API サーバーの起動処理。
//!
//! 起動のしかた：
//!
//! ```text
//! study-record-server.exe                      コンソールで起動する（Ctrl+C で停止）
//! study-record-server.exe install [--home DIR] Windows サービスとして登録する（管理者権限が必要）
//! study-record-server.exe uninstall            Windows サービスの登録を解除する（管理者権限が必要）
//! study-record-server.exe service --home DIR   サービスとしての起動（サービス管理から呼ばれる。手動では使わない）
//! ```
//!
//! 起動の流れ：
//! 1. 設定を読み込む（config/app.ini と環境変数）
//! 2. ログ出力（SystemRunningLog.log）を初期化する
//! 3. DB に接続し、マイグレーションと起動時の復旧処理を行う
//! 4. HTTP サーバーを起動する（停止の合図を受け取ったら、処理中の要求を終えてから止まる）

#[cfg(windows)]
mod service_host;

use std::{future::Future, io::Write, net::SocketAddr, path::Path, time::Duration};

use anyhow::{Context, Result};
use tokio::net::TcpListener;
use tracing::{error, info, warn};

use study_record_server::{
    application::{auth_service::AuthService, startup_service},
    build_state,
    infrastructure::{config::AppConfig, database, logging},
    presentation::router,
};

/// 期限切れのセッション・トークンを掃除する間隔。
const PURGE_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// ログ出力の初期化前に失敗したときに書き込むファイル（作業フォルダに作る）。
const FALLBACK_LOG_PATH: &str = "SystemRunningLog.log";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        None => run_console(),
        #[cfg(windows)]
        Some("install") => report(service_host::install(&args[1..])),
        #[cfg(windows)]
        Some("uninstall") => report(service_host::uninstall()),
        #[cfg(windows)]
        Some("service") => report(service_host::run_as_service(&args[1..])),
        Some("-h" | "--help" | "help") => {
            print_usage();
            0
        }
        Some(other) => {
            eprintln!("不明な指定です: {other}");
            print_usage();
            2
        }
    };
    std::process::exit(code);
}

/// 使い方を表示する。
fn print_usage() {
    eprintln!("使い方:");
    eprintln!("  study-record-server                       コンソールで起動する（Ctrl+C で停止）");
    #[cfg(windows)]
    {
        eprintln!("  study-record-server install [--home DIR]  Windows サービスとして登録する（管理者権限が必要）");
        eprintln!("  study-record-server uninstall             Windows サービスの登録を解除する（管理者権限が必要）");
    }
}

/// 管理用コマンドの結果を表示し、終了コードに変換する。
#[cfg(windows)]
fn report(result: Result<()>) -> i32 {
    match result {
        Ok(()) => 0,
        Err(err) => {
            // 画面（標準エラー出力）とログファイルの両方に書く。
            write_fallback_log(&format!("{err:#}"));
            1
        }
    }
}

/// コンソールで起動する。Ctrl+C（Linux では SIGTERM も）で停止する。
fn run_console() -> i32 {
    match run_server(shutdown_signal()) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

/// 設定・ログを初期化し、停止の合図（`shutdown`）が来るまでサーバーを動かす。
///
/// コンソール起動とサービス起動で共通の処理。失敗した内容はログに記録してからエラーを返す。
pub(crate) fn run_server<F>(shutdown: F) -> Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    let config = match AppConfig::load() {
        Ok(config) => config,
        Err(err) => {
            write_fallback_log(&format!("設定の読み込みに失敗しました: {err:#}"));
            return Err(err);
        }
    };
    if let Err(err) = logging::init(&config.log) {
        write_fallback_log(&format!("ログ出力の初期化に失敗しました: {err:#}"));
        return Err(err);
    }
    let result = tokio::runtime::Runtime::new()
        .context("非同期ランタイムを起動できませんでした")
        .and_then(|runtime| runtime.block_on(serve(config, shutdown)));
    match &result {
        Ok(()) => info!("サーバーを停止しました"),
        Err(err) => error!(error = %format!("{err:#}"), "サーバーの起動に失敗しました"),
    }
    result
}

/// DB の準備をしてから HTTP サーバーを起動し、`shutdown` が完了するまで動かす。
async fn serve<F>(config: AppConfig, shutdown: F) -> Result<()>
where
    F: Future<Output = ()> + Send + 'static,
{
    info!(
        version = env!("CARGO_PKG_VERSION"),
        work_dir = %std::env::current_dir().map(|d| d.display().to_string()).unwrap_or_default(),
        "サーバーを起動します"
    );
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
        .with_graceful_shutdown(async move {
            shutdown.await;
            info!("停止の合図を受け取りました。処理中の要求を終えてから停止します");
        })
        .await
        .context("HTTP サーバーが異常終了しました")?;
    pool.close().await;
    Ok(())
}

/// 期限切れのセッションとメールトークンを定期的に削除する。
fn spawn_purge_task(auth: AuthService) {
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

/// コンソール起動時の停止の合図：Ctrl+C（Windows / Linux）または SIGTERM（Linux）。
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
}

/// ログ出力を初期化できないときに、標準エラー出力と作業フォルダのログファイルへ書き込む。
pub(crate) fn write_fallback_log(message: &str) {
    eprintln!("{message}");
    let line = format!(
        "{} ERROR {}\n",
        study_record_server::domain::time::now_jst().format("%Y-%m-%d %H:%M:%S%.3f%:z"),
        message
    );
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(Path::new(FALLBACK_LOG_PATH))
    {
        let _ = file.write_all(line.as_bytes());
    }
}
