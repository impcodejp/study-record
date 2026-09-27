//! ログ出力の初期化。
//!
//! すべてのログを `SystemRunningLog.log` に出力する（標準出力にも同じ内容を出す）。
//! ファイルが設定サイズ（既定 2MB）を超えたら `SystemRunningLog1.log` に切り替え、
//! 設定した世代数（既定 1 世代）だけ残す。

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use tracing_subscriber::{
    fmt::{self, format::Writer, time::FormatTime},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

use super::config::LogConfig;
use crate::domain::time::now_jst;

/// ログの時刻を日本時間（例：`2026-09-28 02:10:00.123+09:00`）で出力する。
struct JstTimer;

impl FormatTime for JstTimer {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        write!(w, "{}", now_jst().format("%Y-%m-%d %H:%M:%S%.3f%:z"))
    }
}

/// サイズでローテーションするログファイル。
struct RotatingFile {
    /// ログファイルのパス。
    path: PathBuf,
    /// ローテーションするサイズ（バイト）。
    max_bytes: u64,
    /// 残す世代数。
    backups: u32,
    /// 書き込み中のファイル。
    file: File,
    /// 現在のファイルサイズ。
    size: u64,
}

impl RotatingFile {
    /// ログファイルを追記モードで開く（無ければ作る）。
    fn open(path: &Path, max_bytes: u64, backups: u32) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        let size = file.metadata()?.len();
        Ok(Self {
            path: path.to_path_buf(),
            max_bytes,
            backups,
            file,
            size,
        })
    }

    /// n 世代目のバックアップファイルのパスを返す。
    ///
    /// 拡張子の前に世代番号を付ける（例：`SystemRunningLog.log` → `SystemRunningLog1.log`）。
    fn backup_path(&self, n: u32) -> PathBuf {
        let stem = self
            .path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let name = match self.path.extension() {
            Some(ext) => format!("{stem}{n}.{}", ext.to_string_lossy()),
            None => format!("{stem}{n}"),
        };
        self.path.with_file_name(name)
    }

    /// 古い世代を 1 つずつずらし、新しいファイルに切り替える。
    fn rotate(&mut self) -> io::Result<()> {
        self.file.flush()?;
        if self.backups == 0 {
            // 世代を残さない設定なら、ファイルを空にして使い続ける。
            self.file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&self.path)?;
        } else {
            let oldest = self.backup_path(self.backups);
            if oldest.exists() {
                fs::remove_file(&oldest)?;
            }
            for n in (1..self.backups).rev() {
                let from = self.backup_path(n);
                if from.exists() {
                    fs::rename(&from, self.backup_path(n + 1))?;
                }
            }
            // Windows では開いたままのファイルを rename できないため、
            // いったん null デバイスのハンドルに差し替えて元のファイルを閉じる。
            drop(std::mem::replace(&mut self.file, null_device()?));
            let renamed = fs::rename(&self.path, self.backup_path(1));
            // rename に失敗しても元のファイルを開き直し、ログを失わないようにする。
            self.file = OpenOptions::new().create(true).append(true).open(&self.path)?;
            if let Err(err) = renamed {
                self.size = self.file.metadata()?.len();
                return Err(err);
            }
        }
        self.size = 0;
        Ok(())
    }
}

/// rename の間だけ使う書き捨てのハンドル（OS の null デバイス）を開く。
fn null_device() -> io::Result<File> {
    let null = if cfg!(windows) { "NUL" } else { "/dev/null" };
    OpenOptions::new().write(true).open(null)
}

impl Write for RotatingFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.size > 0 && self.size + buf.len() as u64 > self.max_bytes {
            // ローテーションに失敗してもログ出力は止めない（元のファイルに書き続ける）。
            if let Err(err) = self.rotate() {
                eprintln!("ログのローテーションに失敗しました: {err}");
            }
        }
        let written = self.file.write(buf)?;
        self.size += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

/// 複数スレッドから共有するためのハンドル。
#[derive(Clone)]
struct SharedLogFile(Arc<Mutex<RotatingFile>>);

impl Write for SharedLogFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut file = self
            .0
            .lock()
            .map_err(|_| io::Error::other("ログファイルのロックが壊れています"))?;
        file.write_all(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut file = self
            .0
            .lock()
            .map_err(|_| io::Error::other("ログファイルのロックが壊れています"))?;
        file.flush()
    }
}

/// ログ出力を初期化する。アプリケーション起動時に 1 回だけ呼ぶ。
pub fn init(config: &LogConfig) -> Result<()> {
    let file = RotatingFile::open(&config.path, config.max_bytes, config.backups)
        .with_context(|| format!("ログファイル {} を開けません", config.path.display()))?;
    let shared = SharedLogFile(Arc::new(Mutex::new(file)));

    let filter = EnvFilter::try_new(&config.level)
        .with_context(|| format!("ログレベルの指定が不正です: {}", config.level))?;

    let file_layer = fmt::layer()
        .with_timer(JstTimer)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_writer(move || shared.clone());
    let stdout_layer = fmt::layer().with_timer(JstTimer).with_target(true);

    tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(stdout_layer)
        .try_init()
        .context("ログ出力の初期化に失敗しました")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotates_when_size_exceeded() {
        let dir = std::env::temp_dir().join(format!("log-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("SystemRunningLog.log");
        let mut file = RotatingFile::open(&path, 10, 1).unwrap();
        file.write_all(b"0123456789").unwrap();
        file.write_all(b"abc").unwrap();
        file.flush().unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"abc");
        assert_eq!(fs::read(dir.join("SystemRunningLog1.log")).unwrap(), b"0123456789");
        drop(file);
        let _ = fs::remove_dir_all(&dir);
    }
}
