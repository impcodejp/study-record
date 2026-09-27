//! SMTP によるメール送信。
//!
//! SMTP の設定が無い場合は `Mailer::Disabled` となり、
//! メール送信を必要とする機能（新規登録・再送・パスワード再設定）は使えない。

use anyhow::{Context, Result};
use lettre::{
    message::{header::ContentType, Mailbox},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use tracing::info;

use super::config::{SmtpConfig, SmtpSecurity};

/// 送信するメール 1 通分。
#[derive(Debug, Clone)]
pub struct OutgoingMail {
    /// 宛先メールアドレス。
    pub to: String,
    /// 件名。
    pub subject: String,
    /// 本文（プレーンテキスト）。
    pub body: String,
}

/// メール送信手段。
#[derive(Clone)]
pub enum Mailer {
    /// SMTP で送信する。
    Smtp {
        /// SMTP 接続（サイズが大きいためヒープに置く）。
        transport: Box<AsyncSmtpTransport<Tokio1Executor>>,
        /// 差出人。
        from: Mailbox,
    },
    /// メール送信の設定が無い。
    Disabled,
}

impl Mailer {
    /// 設定からメール送信手段を作る。
    pub fn from_config(config: Option<&SmtpConfig>) -> Result<Self> {
        let Some(config) = config else {
            return Ok(Self::Disabled);
        };
        let builder = match config.security {
            SmtpSecurity::StartTls => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(&config.host)?,
            SmtpSecurity::Tls => AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)?,
            SmtpSecurity::None => AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.host),
        };
        let builder = builder.port(config.port);
        let builder = match (&config.username, &config.password) {
            (Some(user), Some(pass)) => builder.credentials(Credentials::new(user.clone(), pass.clone())),
            _ => builder,
        };
        let from: Mailbox = config
            .from
            .parse()
            .with_context(|| format!("差出人アドレスが不正です: {}", config.from))?;
        info!(host = %config.host, port = config.port, "SMTP によるメール送信を有効にしました");
        Ok(Self::Smtp {
            transport: Box::new(builder.build()),
            from,
        })
    }

    /// メール送信が使えるかどうか。
    pub fn is_enabled(&self) -> bool {
        matches!(self, Self::Smtp { .. })
    }

    /// メールを送信する。送信できない場合はエラーを返す（ログ出力は呼び出し側で行う）。
    pub async fn send(&self, mail: OutgoingMail) -> Result<()> {
        let Self::Smtp { transport, from } = self else {
            anyhow::bail!("メール送信が設定されていません");
        };
        let to: Mailbox = mail.to.parse().context("宛先アドレスが不正です")?;
        let message = Message::builder()
            .from(from.clone())
            .to(to)
            .subject(mail.subject)
            .header(ContentType::TEXT_PLAIN)
            .body(mail.body)
            .context("メールの組み立てに失敗しました")?;
        transport
            .send(message)
            .await
            .context("SMTP サーバーへの送信に失敗しました")?;
        Ok(())
    }
}
