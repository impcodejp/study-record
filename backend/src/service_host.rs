//! Windows サービスとしての登録・起動・停止。
//!
//! - `install`   : サービスを登録する（自動起動・異常終了時は自動で再起動）
//! - `uninstall` : サービスを停止して登録を解除する
//! - `service`   : サービス管理（SCM）から呼ばれ、サーバーを動かす
//!
//! サービスとして動くときは、作業フォルダを「ホームフォルダ」に切り替える。
//! 設定（config/app.ini）・データ（data/）・ログ（logs/）はすべてホームフォルダの中に置かれる。
//! ホームフォルダは登録時に `--home` で指定でき、省略すると exe のあるフォルダになる。
//!
//! サービスは権限の低い組み込みアカウント「LocalService」で動かし、
//! 書き込み権限は data/ と logs/ にだけ与える（インターネット公開を前提とした最小権限）。

use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
    time::Duration,
};

use anyhow::{bail, Context, Result};
use tokio::sync::watch;
use windows_service::{
    define_windows_service,
    service::{
        ServiceAccess, ServiceAction, ServiceActionType, ServiceControl, ServiceControlAccept,
        ServiceErrorControl, ServiceExitCode, ServiceFailureActions, ServiceFailureResetPeriod,
        ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
    service_manager::{ServiceManager, ServiceManagerAccess},
};

use crate::{run_server, write_fallback_log};

/// サービス名（`sc query StudyRecordServer` などで使う）。
pub const SERVICE_NAME: &str = "StudyRecordServer";
/// サービスの表示名。
const DISPLAY_NAME: &str = "資格勉強 回答正誤記録アプリ API サーバー";
/// サービスの説明。
const DESCRIPTION: &str = "資格勉強 回答正誤記録アプリの API サーバー。設定・データ・ログはホームフォルダ（config / data / logs）に置く。";
/// サービスを動かすアカウント（権限の低い組み込みアカウント）。
const SERVICE_ACCOUNT: &str = r"NT AUTHORITY\LocalService";
/// LocalService アカウントの SID（言語設定によらず icacls で指定できる）。
const LOCAL_SERVICE_SID: &str = "*S-1-5-19";
/// 異常終了したときに再起動するまでの待ち時間。
const RESTART_DELAY: Duration = Duration::from_secs(10);
/// 異常終了の回数を数え直すまでの期間。
const FAILURE_RESET_PERIOD: Duration = Duration::from_secs(24 * 60 * 60);
/// 停止にかかる時間の目安（サービス管理に伝える）。
const STOP_WAIT_HINT: Duration = Duration::from_secs(30);

/// `service` の起動引数で受け取ったホームフォルダ（サービスの入口関数へ渡すため）。
static SERVICE_HOME: OnceLock<PathBuf> = OnceLock::new();

// サービス管理（SCM）から呼ばれる入口関数 `ffi_service_main` を定義する。
define_windows_service!(ffi_service_main, service_main);

/// サービスとして登録する（`install [--home DIR]`）。
pub fn install(args: &[String]) -> Result<()> {
    let exe = std::env::current_exe().context("exe のパスを取得できません")?;
    let home = match parse_home(args)? {
        Some(home) => home,
        None => exe
            .parent()
            .context("exe のフォルダを取得できません")?
            .to_path_buf(),
    };
    let home = std::fs::canonicalize(&home)
        .with_context(|| format!("ホームフォルダ {} が見つかりません", home.display()))?;
    let home = strip_verbatim_prefix(&home);

    // 先にサービス管理へ接続し、管理者権限が無ければフォルダの権限を変える前に止める。
    let manager = ServiceManager::local_computer(
        None::<&str>,
        ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE,
    )
    .context("サービス管理に接続できません。管理者として実行してください")?;
    prepare_home(&home)?;
    let info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(DISPLAY_NAME),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: exe.clone(),
        launch_arguments: vec![
            OsString::from("service"),
            OsString::from("--home"),
            home.clone().into_os_string(),
        ],
        dependencies: vec![],
        account_name: Some(OsString::from(SERVICE_ACCOUNT)),
        account_password: None,
    };
    let service = manager
        .create_service(&info, ServiceAccess::CHANGE_CONFIG | ServiceAccess::START)
        .context("サービスを登録できません（すでに登録されている場合は、先に uninstall してください）")?;
    service.set_description(DESCRIPTION)?;
    // 異常終了したら 10 秒後に再起動する（3 回まで。24 時間で回数を数え直す）。
    service.update_failure_actions(ServiceFailureActions {
        reset_period: ServiceFailureResetPeriod::After(FAILURE_RESET_PERIOD),
        reboot_msg: None,
        command: None,
        actions: Some(vec![
            ServiceAction {
                action_type: ServiceActionType::Restart,
                delay: RESTART_DELAY,
            };
            3
        ]),
    })?;
    // 起動に失敗して終了コード付きで止まった場合も、再起動の対象にする。
    service.set_failure_actions_on_non_crash_failures(true)?;

    println!("サービス「{SERVICE_NAME}」を登録しました。");
    println!("  exe            : {}", exe.display());
    println!("  ホームフォルダ : {}", home.display());
    println!("  実行アカウント : {SERVICE_ACCOUNT}");
    println!("開始するには: sc start {SERVICE_NAME}（または「サービス」画面から開始）");
    Ok(())
}

/// サービスを停止して登録を解除する（`uninstall`）。
pub fn uninstall() -> Result<()> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .context("サービス管理に接続できません。管理者として実行してください")?;
    let service = manager
        .open_service(
            SERVICE_NAME,
            ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE,
        )
        .with_context(|| format!("サービス「{SERVICE_NAME}」が見つかりません"))?;
    if service.query_status()?.current_state != ServiceState::Stopped {
        service.stop().context("サービスを停止できません")?;
        // 停止を最大 60 秒待つ。
        for _ in 0..60 {
            if service.query_status()?.current_state == ServiceState::Stopped {
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    }
    service.delete().context("サービスの登録を解除できません")?;
    println!("サービス「{SERVICE_NAME}」の登録を解除しました（データとログは残しています）。");
    Ok(())
}

/// サービス管理から起動されたときの処理（`service --home DIR`）。
pub fn run_as_service(args: &[String]) -> Result<()> {
    let home = parse_home(args)?.context("--home が指定されていません")?;
    // 作業フォルダをホームフォルダにする。以後の相対パス（config / data / logs）はここが基準になる。
    std::env::set_current_dir(&home)
        .with_context(|| format!("ホームフォルダ {} に移動できません", home.display()))?;
    SERVICE_HOME
        .set(home)
        .map_err(|_| anyhow::anyhow!("ホームフォルダの設定が重複しています"))?;
    service_dispatcher::start(SERVICE_NAME, ffi_service_main).context(
        "サービスとして起動できません（このコマンドはサービス管理から呼ばれるものです。手動で起動する場合は引数なしで実行してください）",
    )?;
    Ok(())
}

/// サービスの本体。サービス管理が別スレッドで呼び出す。
fn service_main(_arguments: Vec<OsString>) {
    if let Err(err) = service_body() {
        write_fallback_log(&format!("サービスの実行に失敗しました: {err:#}"));
    }
}

/// 停止の要求を受け付けながらサーバーを動かし、状態をサービス管理に報告する。
fn service_body() -> Result<()> {
    let (stop_tx, mut stop_rx) = watch::channel(false);
    let status_handle = service_control_handler::register(SERVICE_NAME, move |control| match control {
        ServiceControl::Stop | ServiceControl::Shutdown => {
            // 受け取り側が終了済みでも問題ないため、送信結果は見ない。
            let _ = stop_tx.send(true);
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
    })?;

    let status = |state: ServiceState, exit_code: ServiceExitCode, wait_hint: Duration| ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: state,
        controls_accepted: if state == ServiceState::Running {
            ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN
        } else {
            ServiceControlAccept::empty()
        },
        exit_code,
        checkpoint: 0,
        wait_hint,
        process_id: None,
    };

    status_handle.set_service_status(status(
        ServiceState::Running,
        ServiceExitCode::Win32(0),
        Duration::default(),
    ))?;

    let shutdown = async move {
        // 停止の要求（true）が来るまで待つ。送信側が無くなった場合も停止する。
        while !*stop_rx.borrow() {
            if stop_rx.changed().await.is_err() {
                break;
            }
        }
    };
    let result = run_server(shutdown);

    // 停止処理中であることを伝えてから、最終的な状態を報告する。
    status_handle.set_service_status(status(
        ServiceState::StopPending,
        ServiceExitCode::Win32(0),
        STOP_WAIT_HINT,
    ))?;
    let exit_code = match &result {
        Ok(()) => ServiceExitCode::Win32(0),
        // 独自の終了コード 1 = 起動または実行に失敗（詳細は SystemRunningLog.log）。
        Err(_) => ServiceExitCode::ServiceSpecific(1),
    };
    status_handle.set_service_status(status(ServiceState::Stopped, exit_code, Duration::default()))?;
    result
}

/// 引数から `--home DIR` を取り出す。
fn parse_home(args: &[String]) -> Result<Option<PathBuf>> {
    let mut iter = args.iter();
    let mut home = None;
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--home" => {
                let value = iter.next().context("--home の後にフォルダを指定してください")?;
                home = Some(PathBuf::from(value));
            }
            other => bail!("不明な指定です: {other}"),
        }
    }
    Ok(home)
}

/// ホームフォルダに data / logs を作り、サービスのアカウントに必要な権限を与える。
///
/// - ホームフォルダ（config を含む）：読み取りと実行だけ
/// - data / logs：変更（読み書き・作成・削除）
fn prepare_home(home: &Path) -> Result<()> {
    for dir in ["data", "logs"] {
        std::fs::create_dir_all(home.join(dir))
            .with_context(|| format!("{} フォルダを作れません", home.join(dir).display()))?;
    }
    grant(home, "(OI)(CI)RX")?;
    grant(&home.join("data"), "(OI)(CI)M")?;
    grant(&home.join("logs"), "(OI)(CI)M")?;
    if !home.join("config").join("app.ini").exists() {
        println!(
            "注意: {} がありません。config\\app.ini.example をコピーして作成してください（無い場合は既定値で動きます）。",
            home.join("config").join("app.ini").display()
        );
    }
    Ok(())
}

/// icacls でフォルダに LocalService の権限を追加する。
fn grant(path: &Path, permission: &str) -> Result<()> {
    let output = Command::new("icacls")
        .arg(path)
        .arg("/grant")
        .arg(format!("{LOCAL_SERVICE_SID}:{permission}"))
        .output()
        .context("icacls を実行できません")?;
    if !output.status.success() {
        bail!(
            "{} の権限を設定できません: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

/// `canonicalize` が付ける `\\?\` を取り除く（サービスの起動引数や表示を読みやすくするため）。
fn strip_verbatim_prefix(path: &Path) -> PathBuf {
    let text = path.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC\\") => PathBuf::from(rest),
        _ => path.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_home_argument() {
        let args = vec!["--home".to_string(), r"C:\app".to_string()];
        assert_eq!(parse_home(&args).unwrap(), Some(PathBuf::from(r"C:\app")));
        assert_eq!(parse_home(&[]).unwrap(), None);
        assert!(parse_home(&["--home".to_string()]).is_err());
        assert!(parse_home(&["--x".to_string()]).is_err());
    }

    #[test]
    fn strips_verbatim_prefix() {
        assert_eq!(strip_verbatim_prefix(Path::new(r"\\?\C:\app")), PathBuf::from(r"C:\app"));
        assert_eq!(strip_verbatim_prefix(Path::new(r"C:\app")), PathBuf::from(r"C:\app"));
    }
}
