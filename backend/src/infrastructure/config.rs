//! 設定ファイル（ini）と環境変数からアプリケーション設定を読み込む。
//!
//! 読み込み順（後のものが優先）：
//! 1. 既定値
//! 2. ini ファイル（既定は `config/app.ini`。環境変数 `APP_CONFIG` で変更可）
//! 3. 環境変数 `APP_<セクション>_<キー>`（例：`APP_SMTP_PASSWORD`）
//!
//! 認証情報（SMTP のパスワードなど）はソースコードに書かず、
//! 必ず ini ファイルか環境変数で与える。

use std::{env, path::PathBuf};

use anyhow::{Context, Result};
use ini::Ini;

/// 設定ファイルの既定パス。
const DEFAULT_CONFIG_PATH: &str = "config/app.ini";

/// アプリケーション全体の設定。
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// HTTP サーバーの設定。
    pub server: ServerConfig,
    /// データベースの設定。
    pub database: DatabaseConfig,
    /// ログの設定。
    pub log: LogConfig,
    /// セッション・認証の設定。
    pub auth: AuthConfig,
    /// メール送信の設定。`None` の場合はメール送信を使う機能が無効になる。
    pub smtp: Option<SmtpConfig>,
}

/// HTTP サーバーの設定。
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// 待ち受けアドレス（例：`127.0.0.1:8080`）。
    pub bind: String,
    /// メール本文のリンクに使う公開 URL（例：`https://study.example.com`）。
    pub public_url: String,
    /// Cookie に Secure 属性を付けるか（HTTPS 運用時は true）。
    pub cookie_secure: bool,
    /// リバースプロキシ（nginx）の `X-Forwarded-For` を信頼するか。
    pub trust_proxy: bool,
}

/// データベースの設定。
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// SQLite ファイルのパス。
    pub path: PathBuf,
}

/// ログの設定。
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// ログファイルのパス（ファイル名は SystemRunningLog.log）。
    pub path: PathBuf,
    /// ローテーションするサイズ（バイト）。
    pub max_bytes: u64,
    /// 残す世代数。
    pub backups: u32,
    /// ログレベル（`tracing` の EnvFilter 書式）。
    pub level: String,
}

/// セッション・認証の設定。
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// セッションの有効時間（時間）。
    pub session_ttl_hours: i64,
    /// 登録確認リンクの有効時間（分）。
    pub verify_token_ttl_minutes: i64,
    /// パスワード再設定リンクの有効時間（分）。
    pub reset_token_ttl_minutes: i64,
    /// ログイン失敗を許す回数（これを超えると一時的にロック）。
    pub login_max_failures: u32,
    /// ログインロックの時間（分）。
    pub login_lock_minutes: i64,
}

/// SMTP の設定。
#[derive(Debug, Clone)]
pub struct SmtpConfig {
    /// SMTP サーバーのホスト名。
    pub host: String,
    /// ポート番号。
    pub port: u16,
    /// 暗号化方式（`starttls` / `tls` / `none`）。
    pub security: SmtpSecurity,
    /// 認証ユーザー名（空なら認証しない）。
    pub username: Option<String>,
    /// 認証パスワード。
    pub password: Option<String>,
    /// 差出人アドレス（例：`学習記録 <noreply@example.com>`）。
    pub from: String,
}

/// SMTP の暗号化方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmtpSecurity {
    /// STARTTLS（587 番ポートなど）。
    StartTls,
    /// 最初から TLS（465 番ポートなど）。
    Tls,
    /// 暗号化なし（社内の中継サーバーや開発用）。
    None,
}

/// ini と環境変数から値を取り出すための小さな補助。
struct Source {
    ini: Option<Ini>,
}

impl Source {
    /// 環境変数 → ini の順に値を探す。空文字は未設定として扱う。
    fn get(&self, section: &str, key: &str) -> Option<String> {
        let env_key = format!("APP_{}_{}", section.to_uppercase(), key.to_uppercase());
        if let Ok(value) = env::var(&env_key) {
            if !value.trim().is_empty() {
                return Some(value.trim().to_string());
            }
        }
        self.ini
            .as_ref()
            .and_then(|ini| ini.get_from(Some(section), key))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }

    /// 文字列を取り出す。無ければ既定値。
    fn string(&self, section: &str, key: &str, default: &str) -> String {
        self.get(section, key).unwrap_or_else(|| default.to_string())
    }

    /// 数値などを取り出す。無ければ既定値、解析できなければエラー。
    fn parse<T>(&self, section: &str, key: &str, default: T) -> Result<T>
    where
        T: std::str::FromStr,
        T::Err: std::error::Error + Send + Sync + 'static,
    {
        match self.get(section, key) {
            Some(value) => value
                .parse()
                .with_context(|| format!("設定 [{section}] {key} の値が不正です: {value}")),
            None => Ok(default),
        }
    }
}

impl AppConfig {
    /// 設定を読み込む。ini ファイルが無い場合は既定値と環境変数だけで動く。
    pub fn load() -> Result<Self> {
        let path = env::var("APP_CONFIG").unwrap_or_else(|_| DEFAULT_CONFIG_PATH.to_string());
        let ini = if std::path::Path::new(&path).exists() {
            Some(Ini::load_from_file(&path).with_context(|| format!("設定ファイル {path} を読めません"))?)
        } else {
            None
        };
        Self::from_source(&Source { ini })
    }

    /// 値の取り出し元から設定を組み立てる。
    fn from_source(src: &Source) -> Result<Self> {
        let server = ServerConfig {
            bind: src.string("server", "bind", "127.0.0.1:8080"),
            public_url: src
                .string("server", "public_url", "http://localhost:5173")
                .trim_end_matches('/')
                .to_string(),
            cookie_secure: src.parse("server", "cookie_secure", false)?,
            trust_proxy: src.parse("server", "trust_proxy", false)?,
        };
        let database = DatabaseConfig {
            path: PathBuf::from(src.string("database", "path", "data/study_record.sqlite3")),
        };
        let log = LogConfig {
            path: PathBuf::from(src.string("log", "path", "logs/SystemRunningLog.log")),
            max_bytes: src.parse("log", "max_bytes", 2 * 1024 * 1024)?,
            backups: src.parse("log", "backups", 1)?,
            level: src.string("log", "level", "info"),
        };
        let auth = AuthConfig {
            session_ttl_hours: src.parse("auth", "session_ttl_hours", 24 * 7)?,
            verify_token_ttl_minutes: src.parse("auth", "verify_token_ttl_minutes", 24 * 60)?,
            reset_token_ttl_minutes: src.parse("auth", "reset_token_ttl_minutes", 30)?,
            login_max_failures: src.parse("auth", "login_max_failures", 5)?,
            login_lock_minutes: src.parse("auth", "login_lock_minutes", 15)?,
        };
        let smtp = match src.get("smtp", "host") {
            None => None,
            Some(host) => {
                let security = match src.string("smtp", "security", "starttls").to_lowercase().as_str() {
                    "starttls" => SmtpSecurity::StartTls,
                    "tls" => SmtpSecurity::Tls,
                    "none" => SmtpSecurity::None,
                    other => anyhow::bail!("設定 [smtp] security の値が不正です: {other}"),
                };
                let from = src
                    .get("smtp", "from")
                    .context("設定 [smtp] from（差出人アドレス）がありません")?;
                Some(SmtpConfig {
                    host,
                    port: src.parse("smtp", "port", 587)?,
                    security,
                    username: src.get("smtp", "username"),
                    password: src.get("smtp", "password"),
                    from,
                })
            }
        };
        Ok(Self {
            server,
            database,
            log,
            auth,
            smtp,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_used_without_ini() {
        let config = AppConfig::from_source(&Source { ini: None }).unwrap();
        assert_eq!(config.log.max_bytes, 2 * 1024 * 1024);
        assert!(config.log.path.ends_with("SystemRunningLog.log"));
    }

    #[test]
    fn ini_values_are_read() {
        let ini = Ini::load_from_str(
            "[smtp]\nhost = mail.example.com\nport = 465\nsecurity = tls\nfrom = a@example.com\n",
        )
        .unwrap();
        let config = AppConfig::from_source(&Source { ini: Some(ini) }).unwrap();
        let smtp = config.smtp.unwrap();
        assert_eq!(smtp.port, 465);
        assert_eq!(smtp.security, SmtpSecurity::Tls);
    }
}
