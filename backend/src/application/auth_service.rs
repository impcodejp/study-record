//! アカウントのユースケース（F-22〜F-27）。
//!
//! 方針：
//! - パスワードは Argon2id でハッシュ化して保存する
//! - セッション・メールのトークンはランダム値を発行し、DB にはハッシュ値だけを保存する
//! - 登録済みかどうかを第三者に推測されないよう、メール送信系の応答は常に同じにする
//! - メール送信はバックグラウンドで行い、失敗はログに記録する

use std::time::Duration as StdDuration;

use chrono::{Duration, Utc};
use sqlx::SqlitePool;
use tracing::{error, info, warn};

use super::{exam_service::DEFAULT_CATEGORY_NAME, login_limiter::AttemptLimiter};
use crate::{
    domain::{
        error::{AppError, AppResult},
        models::User,
        time::{now_utc_string, DB_DATETIME_FORMAT},
        validation::{validate_email, validate_password, validate_user_name},
    },
    infrastructure::{
        config::{AppConfig, AuthConfig},
        database::{begin_write, is_unique_violation},
        mailer::{Mailer, OutgoingMail},
        repositories::user_repository::{self as repo, TokenPurpose},
        security::{
            dummy_password_hash, generate_token, hash_password, hash_token, mask_email,
            verify_password,
        },
    },
};

/// メール送信系の API で、同じアドレスに再送できるまでの間隔。
const MAIL_INTERVAL: StdDuration = StdDuration::from_secs(60);

/// 期限切れのメールトークンを残しておく日数（期限切れ後の再送に対応するため）。
const EXPIRED_TOKEN_KEEP_DAYS: i64 = 7;

/// ログインに成功したときの結果。
#[derive(Debug, Clone)]
pub struct LoginResult {
    /// ログインしたユーザー。
    pub user: User,
    /// Cookie に入れるセッショントークン（平文。DB にはハッシュだけを保存）。
    pub session_token: String,
    /// 状態を変える API で送り返してもらう CSRF トークン。
    pub csrf_token: String,
    /// セッションの有効時間（秒）。
    pub max_age_seconds: i64,
}

/// セッションから取り出したログイン情報。
#[derive(Debug, Clone)]
pub struct SessionInfo {
    /// ログイン中のユーザー。
    pub user: User,
    /// CSRF トークン。
    pub csrf_token: String,
}

/// アカウントのユースケース。
#[derive(Clone)]
pub struct AuthService {
    pool: SqlitePool,
    mailer: Mailer,
    auth: AuthConfig,
    public_url: String,
    /// ログイン失敗の回数制限（キー：メールアドレス / 接続元 IP）。
    login_limiter: AttemptLimiter,
    /// メール送信の連打対策（キー：用途 + メールアドレス）。
    mail_limiter: AttemptLimiter,
}

impl AuthService {
    /// サービスを作る。
    pub fn new(pool: SqlitePool, mailer: Mailer, config: &AppConfig) -> Self {
        let lock = StdDuration::from_secs(config.auth.login_lock_minutes.max(1) as u64 * 60);
        Self {
            pool,
            mailer,
            auth: config.auth.clone(),
            public_url: config.server.public_url.clone(),
            login_limiter: AttemptLimiter::new(config.auth.login_max_failures, lock),
            mail_limiter: AttemptLimiter::new(1, MAIL_INTERVAL),
        }
    }

    // -----------------------------------------------------------------------
    // 新規登録（F-22, F-23）
    // -----------------------------------------------------------------------

    /// 新規登録を受け付け、確認メールを送る（F-22）。
    ///
    /// 確認が終わるまでアカウントは作らない。登録済みのアドレスには「登録済み」の案内を送る。
    pub async fn register(&self, name: &str, email: &str) -> AppResult<()> {
        self.ensure_mail_enabled()?;
        let name = validate_user_name(name)?;
        let email = validate_email(email)?;
        self.throttle_mail("register", &email)?;

        let mut tx = begin_write(&self.pool).await?;
        if repo::find_by_email(&mut tx, &email).await?.is_some() {
            tx.commit().await?;
            info!(email = %mask_email(&email), "登録済みのアドレスで新規登録が要求されました");
            self.send_in_background(OutgoingMail {
                to: email,
                subject: "【学習記録】このメールアドレスは登録済みです".into(),
                body: format!(
                    "このメールアドレスはすでに登録されています。\n\
                     パスワードをお忘れの場合は、次のページから再設定してください。\n\n\
                     {}/forgot-password\n\n\
                     このメールに心当たりがない場合は、破棄してください。\n",
                    self.public_url
                ),
            });
            return Ok(());
        }
        let token = self.issue_token(&mut tx, TokenPurpose::Verify, &email, Some(&name)).await?;
        tx.commit().await?;
        info!(email = %mask_email(&email), "新規登録の確認メールを送信します");
        self.send_in_background(self.verify_mail(&email, &name, &token));
        Ok(())
    }

    /// 新規登録の確認メールを再送する（F-23）。
    ///
    /// 確認待ちの登録が無い場合も、同じ応答を返す（登録状況を推測されないため）。
    pub async fn resend_verification(&self, email: &str) -> AppResult<()> {
        self.ensure_mail_enabled()?;
        let email = validate_email(email)?;
        self.throttle_mail("register", &email)?;

        let mut tx = begin_write(&self.pool).await?;
        let pending = repo::find_pending_registration(&mut tx, &email).await?;
        let already = repo::find_by_email(&mut tx, &email).await?.is_some();
        match (pending, already) {
            (Some(pending), false) => {
                let name = pending.user_name.unwrap_or_default();
                let token = self.issue_token(&mut tx, TokenPurpose::Verify, &email, Some(&name)).await?;
                tx.commit().await?;
                info!(email = %mask_email(&email), "確認メールを再送します");
                self.send_in_background(self.verify_mail(&email, &name, &token));
            }
            _ => {
                tx.commit().await?;
                info!(email = %mask_email(&email), "確認待ちの登録が無いため再送しませんでした");
            }
        }
        Ok(())
    }

    /// 確認リンクのトークンとパスワードで登録を完了する（F-22）。
    pub async fn verify_email(&self, token: &str, password: &str) -> AppResult<User> {
        validate_password(password)?;
        let password_hash = hash_in_background(password.to_string()).await?;
        let now = now_utc_string();

        let mut tx = begin_write(&self.pool).await?;
        let record = repo::take_email_token(&mut tx, TokenPurpose::Verify, &hash_token(token), &now)
            .await?
            .ok_or_else(invalid_link)?;
        let name = record.user_name.unwrap_or_default();
        let user_id = match repo::insert(&mut tx, &name, &record.email, &password_hash, &now).await {
            Ok(id) => id,
            Err(err) if is_unique_violation(&err) => {
                return Err(AppError::Conflict(
                    "このメールアドレスはすでに登録されています。".into(),
                ))
            }
            Err(err) => return Err(err.into()),
        };
        tx.commit().await?;
        info!(user_id, email = %mask_email(&record.email), "新規登録が完了しました");
        Ok(User {
            id: user_id,
            name,
            email: record.email,
        })
    }

    // -----------------------------------------------------------------------
    // ログイン・ログアウト（F-24, F-27）
    // -----------------------------------------------------------------------

    /// メールアドレスとパスワードでログインする（F-24）。
    ///
    /// `client_ip` は試行回数の制限に使う。
    pub async fn login(&self, email: &str, password: &str, client_ip: &str) -> AppResult<LoginResult> {
        let email = email.trim().to_string();
        let email_key = format!("email:{}", email.to_lowercase());
        let ip_key = format!("ip:{client_ip}");
        if self.login_limiter.locked_for(&email_key).is_some()
            || self.login_limiter.locked_for(&ip_key).is_some()
        {
            warn!(email = %mask_email(&email), client_ip, "ロック中のためログインを拒否しました");
            return Err(AppError::TooManyRequests(
                "ログインの失敗が続いたため、しばらくログインできません。時間をおいて再度お試しください。".into(),
            ));
        }

        let mut conn = self.pool.acquire().await?;
        let record = repo::find_by_email(&mut conn, &email).await?;
        drop(conn);
        // ユーザーがいない場合もダミーのハッシュで同じだけ計算し、応答時間をそろえる。
        let hash = record
            .as_ref()
            .map(|r| r.password_hash.clone())
            .unwrap_or_else(|| dummy_password_hash().to_string());
        let matched = verify_in_background(password.to_string(), hash).await? && record.is_some();

        let Some(record) = record.filter(|_| matched) else {
            let locked_email = self.login_limiter.record_failure(&email_key);
            let locked_ip = self.login_limiter.record_failure(&ip_key);
            warn!(
                email = %mask_email(&email),
                client_ip,
                locked = locked_email || locked_ip,
                "ログインに失敗しました"
            );
            return Err(AppError::Unauthorized(
                "メールアドレスまたはパスワードが正しくありません。".into(),
            ));
        };
        self.login_limiter.reset(&email_key);
        self.login_limiter.reset(&ip_key);

        let result = self.create_session(record.to_user()).await?;
        info!(user_id = result.user.id, client_ip, "ログインしました");
        Ok(result)
    }

    /// ログアウトする（セッションを削除する）。
    pub async fn logout(&self, session_token: &str) -> AppResult<()> {
        let mut conn = self.pool.acquire().await?;
        repo::delete_session(&mut conn, &hash_token(session_token)).await?;
        info!("ログアウトしました");
        Ok(())
    }

    /// セッショントークンからログイン中のユーザーを取得する（F-27）。
    pub async fn session(&self, session_token: &str) -> AppResult<Option<SessionInfo>> {
        let mut conn = self.pool.acquire().await?;
        let found = repo::find_session(&mut conn, &hash_token(session_token), &now_utc_string()).await?;
        Ok(found.map(|(user, csrf_token)| SessionInfo { user, csrf_token }))
    }

    // -----------------------------------------------------------------------
    // パスワード（F-25, F-26）
    // -----------------------------------------------------------------------

    /// パスワード再設定メールを送る（F-25）。未登録のアドレスでも同じ応答を返す。
    pub async fn forgot_password(&self, email: &str) -> AppResult<()> {
        self.ensure_mail_enabled()?;
        let email = validate_email(email)?;
        self.throttle_mail("reset", &email)?;

        let mut tx = begin_write(&self.pool).await?;
        let Some(user) = repo::find_by_email(&mut tx, &email).await? else {
            tx.commit().await?;
            info!(email = %mask_email(&email), "未登録のアドレスでパスワード再設定が要求されました");
            return Ok(());
        };
        let token = self.issue_token(&mut tx, TokenPurpose::Reset, &user.email, None).await?;
        tx.commit().await?;
        info!(user_id = user.id, "パスワード再設定メールを送信します");
        self.send_in_background(OutgoingMail {
            to: user.email.clone(),
            subject: "【学習記録】パスワードの再設定".into(),
            body: format!(
                "{} 様\n\n次のリンクを開いて、新しいパスワードを設定してください。\n\
                 リンクの有効期限は {} 分です。\n\n{}/reset-password?token={}\n\n\
                 このメールに心当たりがない場合は、破棄してください（パスワードは変更されません）。\n",
                user.name, self.auth.reset_token_ttl_minutes, self.public_url, token
            ),
        });
        Ok(())
    }

    /// 再設定リンクのトークンで新しいパスワードを設定する（F-25）。
    ///
    /// 安全のため、そのユーザーのすべてのセッションをログアウトさせる。
    pub async fn reset_password(&self, token: &str, password: &str) -> AppResult<()> {
        validate_password(password)?;
        let password_hash = hash_in_background(password.to_string()).await?;
        let now = now_utc_string();

        let mut tx = begin_write(&self.pool).await?;
        let record = repo::take_email_token(&mut tx, TokenPurpose::Reset, &hash_token(token), &now)
            .await?
            .ok_or_else(invalid_link)?;
        let user = repo::find_by_email(&mut tx, &record.email)
            .await?
            .ok_or_else(invalid_link)?;
        repo::update_password(&mut tx, user.id, &password_hash, &now).await?;
        let sessions = repo::delete_sessions_of_user(&mut tx, user.id, None).await?;
        tx.commit().await?;
        info!(user_id = user.id, revoked_sessions = sessions, "パスワードを再設定しました");
        self.send_in_background(self.password_changed_mail(&user.email, &user.name));
        Ok(())
    }

    /// ログイン中に現在のパスワードを確認して、新しいパスワードに変更する（F-26）。
    ///
    /// 今使っているセッション以外はログアウトさせる。
    pub async fn change_password(
        &self,
        user_id: i64,
        session_token: &str,
        current_password: &str,
        new_password: &str,
    ) -> AppResult<()> {
        validate_password(new_password)?;
        let mut conn = self.pool.acquire().await?;
        let user = repo::find_by_id(&mut conn, user_id)
            .await?
            .ok_or_else(|| AppError::Unauthorized("ログインしてください。".into()))?;
        drop(conn);
        if !verify_in_background(current_password.to_string(), user.password_hash.clone()).await? {
            warn!(user_id, "パスワード変更で現在のパスワードが一致しませんでした");
            return Err(AppError::Validation("現在のパスワードが正しくありません。".into()));
        }
        let password_hash = hash_in_background(new_password.to_string()).await?;

        let mut tx = begin_write(&self.pool).await?;
        repo::update_password(&mut tx, user_id, &password_hash, &now_utc_string()).await?;
        let revoked =
            repo::delete_sessions_of_user(&mut tx, user_id, Some(&hash_token(session_token))).await?;
        tx.commit().await?;
        info!(user_id, revoked_sessions = revoked, "パスワードを変更しました");
        self.send_in_background(self.password_changed_mail(&user.email, &user.name));
        Ok(())
    }

    /// 期限切れのセッションとメールトークンを削除する（定期実行用）。
    pub async fn purge_expired(&self) -> AppResult<u64> {
        let now = Utc::now();
        let cutoff = (now - Duration::days(EXPIRED_TOKEN_KEEP_DAYS)).format(DB_DATETIME_FORMAT).to_string();
        let mut tx = begin_write(&self.pool).await?;
        let deleted = repo::delete_expired(&mut tx, &now.format(DB_DATETIME_FORMAT).to_string(), &cutoff).await?;
        tx.commit().await?;
        Ok(deleted)
    }

    // -----------------------------------------------------------------------
    // 内部処理
    // -----------------------------------------------------------------------

    /// セッションを作る。
    async fn create_session(&self, user: User) -> AppResult<LoginResult> {
        let session_token = generate_token();
        let csrf_token = generate_token();
        let now = Utc::now();
        let ttl = Duration::hours(self.auth.session_ttl_hours.max(1));
        let mut tx = begin_write(&self.pool).await?;
        repo::insert_session(
            &mut tx,
            &hash_token(&session_token),
            user.id,
            &csrf_token,
            &now.format(DB_DATETIME_FORMAT).to_string(),
            &(now + ttl).format(DB_DATETIME_FORMAT).to_string(),
        )
        .await?;
        tx.commit().await?;
        Ok(LoginResult {
            user,
            session_token,
            csrf_token,
            max_age_seconds: ttl.num_seconds(),
        })
    }

    /// メールのワンタイムトークンを発行して保存し、平文のトークンを返す。
    async fn issue_token(
        &self,
        conn: &mut sqlx::SqliteConnection,
        purpose: TokenPurpose,
        email: &str,
        user_name: Option<&str>,
    ) -> AppResult<String> {
        let ttl_minutes = match purpose {
            TokenPurpose::Verify => self.auth.verify_token_ttl_minutes,
            TokenPurpose::Reset => self.auth.reset_token_ttl_minutes,
        };
        let token = generate_token();
        let now = Utc::now();
        repo::replace_email_token(
            conn,
            purpose,
            &hash_token(&token),
            email,
            user_name,
            &now.format(DB_DATETIME_FORMAT).to_string(),
            &(now + Duration::minutes(ttl_minutes.max(1))).format(DB_DATETIME_FORMAT).to_string(),
        )
        .await?;
        Ok(token)
    }

    /// 新規登録の確認メールを組み立てる。
    fn verify_mail(&self, email: &str, name: &str, token: &str) -> OutgoingMail {
        OutgoingMail {
            to: email.to_string(),
            subject: "【学習記録】メールアドレスの確認".into(),
            body: format!(
                "{name} 様\n\nご登録ありがとうございます。\n\
                 次のリンクを開いてパスワードを設定すると、登録が完了します。\n\
                 リンクの有効期限は {} 分です。\n\n{}/verify-email?token={token}\n\n\
                 このメールに心当たりがない場合は、破棄してください。\n\
                 登録後は、試験を登録し、カテゴリ（初期状態は「{DEFAULT_CATEGORY_NAME}」）を整えてから学習を始めてください。\n",
                self.auth.verify_token_ttl_minutes, self.public_url
            ),
        }
    }

    /// パスワード変更の通知メールを組み立てる。
    fn password_changed_mail(&self, email: &str, name: &str) -> OutgoingMail {
        OutgoingMail {
            to: email.to_string(),
            subject: "【学習記録】パスワードが変更されました".into(),
            body: format!(
                "{name} 様\n\nパスワードが変更されました。\n\
                 心当たりがない場合は、次のページからすぐにパスワードを再設定してください。\n\n\
                 {}/forgot-password\n",
                self.public_url
            ),
        }
    }

    /// メール送信が設定されていなければ 503 を返す。
    fn ensure_mail_enabled(&self) -> AppResult<()> {
        if self.mailer.is_enabled() {
            Ok(())
        } else {
            Err(AppError::ServiceUnavailable(
                "メール送信が設定されていないため、この機能は利用できません。管理者に連絡してください。".into(),
            ))
        }
    }

    /// 同じアドレスへのメール送信を一定間隔に制限する。
    fn throttle_mail(&self, purpose: &str, email: &str) -> AppResult<()> {
        let key = format!("{purpose}:{}", email.to_lowercase());
        if self.mail_limiter.locked_for(&key).is_some() {
            return Err(AppError::TooManyRequests(
                "メールを送信したばかりです。1分ほど待ってから再度お試しください。".into(),
            ));
        }
        self.mail_limiter.record_failure(&key);
        Ok(())
    }

    /// メールをバックグラウンドで送信する。失敗はログに記録する。
    fn send_in_background(&self, mail: OutgoingMail) {
        let mailer = self.mailer.clone();
        tokio::spawn(async move {
            let to = mask_email(&mail.to);
            let subject = mail.subject.clone();
            match mailer.send(mail).await {
                Ok(()) => info!(to = %to, subject = %subject, "メールを送信しました"),
                Err(err) => error!(to = %to, subject = %subject, error = %format!("{err:#}"), "メールの送信に失敗しました"),
            }
        });
    }
}

/// リンクが無効・期限切れのときのエラー。
fn invalid_link() -> AppError {
    AppError::Validation(
        "リンクが無効か、有効期限が切れています。もう一度手続きをやり直してください。".into(),
    )
}

/// パスワードのハッシュ化を別スレッドで行う（非同期処理を止めないため）。
async fn hash_in_background(password: String) -> AppResult<String> {
    tokio::task::spawn_blocking(move || hash_password(&password))
        .await
        .map_err(|err| AppError::Internal(err.into()))?
}

/// パスワードの照合を別スレッドで行う。
async fn verify_in_background(password: String, hash: String) -> AppResult<bool> {
    tokio::task::spawn_blocking(move || verify_password(&password, &hash))
        .await
        .map_err(|err| AppError::Internal(err.into()))
}
