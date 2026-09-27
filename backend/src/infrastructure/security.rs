//! パスワードのハッシュ化とランダムトークンの生成。

use anyhow::anyhow;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::domain::error::{AppError, AppResult};

/// トークンのバイト数（256 ビット）。
const TOKEN_BYTES: usize = 32;

/// パスワードを Argon2id でハッシュ化する（PHC 文字列形式）。
///
/// 計算に時間がかかるため、非同期処理からは `spawn_blocking` 経由で呼ぶこと。
pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|err| AppError::Internal(anyhow!("パスワードのハッシュ化に失敗しました: {err}")))
}

/// パスワードがハッシュと一致するかを確かめる。
///
/// ハッシュの形式が壊れている場合も「一致しない」として扱う。
pub fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .map(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
        .unwrap_or(false)
}

/// ログイン失敗時の比較に使うダミーのハッシュ。
///
/// 存在しないメールアドレスでも同じだけ時間をかけ、応答時間から登録有無を推測されにくくする。
pub fn dummy_password_hash() -> &'static str {
    use std::sync::OnceLock;
    static DUMMY: OnceLock<String> = OnceLock::new();
    DUMMY.get_or_init(|| hash_password("dummy-password-for-timing").unwrap_or_default())
}

/// URL に埋め込めるランダムなトークンを生成する。
pub fn generate_token() -> String {
    let mut bytes = [0u8; TOKEN_BYTES];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// トークンを DB 保存用の SHA-256 ハッシュ（16 進）に変換する。
///
/// DB が漏えいしても、トークンそのものは復元できないようにする。
pub fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// ログに出すためにメールアドレスを一部伏せる（例：`ab***@example.com`）。
pub fn mask_email(email: &str) -> String {
    match email.split_once('@') {
        Some((local, domain)) => {
            let head: String = local.chars().take(2).collect();
            format!("{head}***@{domain}")
        }
        None => "***".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_round_trip() {
        let hash = hash_password("correct horse").unwrap();
        assert!(verify_password("correct horse", &hash));
        assert!(!verify_password("wrong", &hash));
        assert!(!verify_password("x", "not-a-hash"));
    }

    #[test]
    fn tokens_are_unique_and_hashed() {
        let a = generate_token();
        let b = generate_token();
        assert_ne!(a, b);
        assert_eq!(hash_token(&a).len(), 64);
        assert_eq!(hash_token(&a), hash_token(&a));
    }

    #[test]
    fn email_is_masked() {
        assert_eq!(mask_email("taro@example.com"), "ta***@example.com");
    }
}
