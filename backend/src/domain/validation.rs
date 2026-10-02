//! 入力値の業務ルール（機能一覧 3 章・7 章）を検証する関数群。
//!
//! 検証に成功した場合は正規化済みの値（前後の空白を除去したものなど）を返す。

use chrono::NaiveDate;

use super::error::{AppError, AppResult};

/// 問題名称の最大文字数。
pub const TITLE_MAX_CHARS: usize = 100;
/// 試験名の最大文字数。
pub const EXAM_NAME_MAX_CHARS: usize = 100;
/// 回答の最大文字数（改行も 1 文字として数える）。
pub const RESPONSE_MAX_CHARS: usize = 255;
/// 採点時のメモ（解説）の最大文字数。
pub const NOTE_MAX_CHARS: usize = 1000;
/// カテゴリ名の最大文字数。
pub const CATEGORY_NAME_MAX_CHARS: usize = 50;
/// ユーザー名の最大文字数。
pub const USER_NAME_MAX_CHARS: usize = 50;
/// メールアドレスの最大文字数（RFC 5321 の上限）。
pub const EMAIL_MAX_CHARS: usize = 254;
/// パスワードの最小文字数。
pub const PASSWORD_MIN_CHARS: usize = 8;
/// パスワードの最大文字数（ハッシュ計算の負荷を抑えるため）。
pub const PASSWORD_MAX_CHARS: usize = 128;
/// 1 日の目標問題数の上限。
pub const DAILY_GOAL_MAX: i64 = 1000;

/// 改行コード（CRLF / CR）を LF に統一する。
fn normalize_newlines(raw: &str) -> String {
    raw.replace("\r\n", "\n").replace('\r', "\n")
}

/// 問題名称を検証し、前後の空白を除去した値を返す。
pub fn validate_title(raw: &str) -> AppResult<String> {
    let title = raw.trim();
    if title.is_empty() {
        return Err(AppError::Validation("問題名称を入力してください。".into()));
    }
    if title.chars().count() > TITLE_MAX_CHARS {
        return Err(AppError::Validation(
            "問題名称は100文字以内で入力してください。".into(),
        ));
    }
    Ok(title.to_string())
}

/// 問題数（1 以上の整数）を検証する。
pub fn validate_question_number(value: i64) -> AppResult<i64> {
    if value < 1 {
        return Err(AppError::Validation(
            "問題数は1以上の数字で入力してください。".into(),
        ));
    }
    Ok(value)
}

/// 回答（自由記述、1〜255 文字、改行可）を検証し、正規化した値を返す。
///
/// 汎用的な資格試験に対応するため形式は問わない。
/// - 改行コードは CRLF / CR を LF に統一する
/// - 空白・改行だけの回答は不可
/// - 前後の空白は利用者の意図を尊重してそのまま残す
pub fn validate_response(raw: &str) -> AppResult<String> {
    let response = normalize_newlines(raw);
    if response.trim().is_empty() {
        return Err(AppError::Validation("回答を入力してください。".into()));
    }
    if response.chars().count() > RESPONSE_MAX_CHARS {
        return Err(AppError::Validation(format!(
            "回答は{RESPONSE_MAX_CHARS}文字以内で入力してください。"
        )));
    }
    Ok(response)
}

/// 採点時に任意で記録する「正解」を検証する。
///
/// 空白だけの場合は未入力（`None`）として扱う。改行コードは LF に統一する。
pub fn validate_correct_answer(raw: Option<&str>) -> AppResult<Option<String>> {
    validate_optional_text(raw, RESPONSE_MAX_CHARS, "正解")
}

/// 採点時に任意で記録する「メモ」を検証する。
///
/// 空白だけの場合は未入力（`None`）として扱う。改行コードは LF に統一する。
pub fn validate_note(raw: Option<&str>) -> AppResult<Option<String>> {
    validate_optional_text(raw, NOTE_MAX_CHARS, "メモ")
}

/// 任意入力のテキストを検証する共通処理。
fn validate_optional_text(
    raw: Option<&str>,
    max: usize,
    label: &str,
) -> AppResult<Option<String>> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let text = normalize_newlines(raw);
    if text.trim().is_empty() {
        return Ok(None);
    }
    if text.chars().count() > max {
        return Err(AppError::Validation(format!(
            "{label}は{max}文字以内で入力してください。"
        )));
    }
    Ok(Some(text))
}

/// 試験名を検証し、前後の空白を除去した値を返す。
pub fn validate_exam_name(raw: &str) -> AppResult<String> {
    let name = raw.trim();
    let len = name.chars().count();
    if len == 0 || len > EXAM_NAME_MAX_CHARS {
        return Err(AppError::Validation(format!(
            "試験名は1～{EXAM_NAME_MAX_CHARS}文字で入力してください。"
        )));
    }
    Ok(name.to_string())
}

/// 試験日（`YYYY-MM-DD`、任意）を検証する。空・未指定なら `None`（試験日を設定しない）。
pub fn validate_exam_date(raw: Option<&str>) -> AppResult<Option<NaiveDate>> {
    let text = raw.map(str::trim).unwrap_or_default();
    if text.is_empty() {
        return Ok(None);
    }
    NaiveDate::parse_from_str(text, "%Y-%m-%d")
        .map(Some)
        .map_err(|_| AppError::Validation("試験日は YYYY-MM-DD の形で入力してください。".into()))
}

/// 1 日の目標問題数（任意、1〜[`DAILY_GOAL_MAX`]）を検証する。未指定なら `None`（目標を設定しない）。
pub fn validate_daily_goal(value: Option<i64>) -> AppResult<Option<i64>> {
    match value {
        None => Ok(None),
        Some(goal) if (1..=DAILY_GOAL_MAX).contains(&goal) => Ok(Some(goal)),
        Some(_) => Err(AppError::Validation(format!(
            "1日の目標は1～{DAILY_GOAL_MAX}問で入力してください。"
        ))),
    }
}

/// 回答時間（0 以上の秒数）を検証する。
pub fn validate_elapsed_seconds(value: i64) -> AppResult<i64> {
    if value < 0 {
        return Err(AppError::Validation("回答時間が不正です。".into()));
    }
    Ok(value)
}

/// カテゴリ名を検証し、前後の空白を除去した値を返す。
pub fn validate_category_name(raw: &str) -> AppResult<String> {
    let name = raw.trim();
    let len = name.chars().count();
    if len == 0 || len > CATEGORY_NAME_MAX_CHARS {
        return Err(AppError::Validation(
            "カテゴリ名は1～50文字で入力してください。".into(),
        ));
    }
    Ok(name.to_string())
}

/// ユーザー名を検証し、前後の空白を除去した値を返す。
pub fn validate_user_name(raw: &str) -> AppResult<String> {
    let name = raw.trim();
    let len = name.chars().count();
    if len == 0 || len > USER_NAME_MAX_CHARS {
        return Err(AppError::Validation(
            "ユーザー名は1～50文字で入力してください。".into(),
        ));
    }
    Ok(name.to_string())
}

/// メールアドレスを簡易検証し、前後の空白を除去した値を返す。
///
/// 厳密な形式チェックは行わず、「@ の前後に文字があり、ドメインに . を含む」ことだけを確認する。
/// 実在確認は確認メールで行う。
pub fn validate_email(raw: &str) -> AppResult<String> {
    let email = raw.trim();
    let invalid = || AppError::Validation("メールアドレスを正しく入力してください。".into());
    if email.is_empty() || email.chars().count() > EMAIL_MAX_CHARS {
        return Err(invalid());
    }
    let (local, domain) = email.split_once('@').ok_or_else(invalid)?;
    if local.is_empty()
        || domain.is_empty()
        || !domain.contains('.')
        || domain.contains('@')
        || email.chars().any(char::is_whitespace)
    {
        return Err(invalid());
    }
    Ok(email.to_string())
}

/// パスワードの文字数を検証する。
pub fn validate_password(raw: &str) -> AppResult<()> {
    let len = raw.chars().count();
    if !(PASSWORD_MIN_CHARS..=PASSWORD_MAX_CHARS).contains(&len) {
        return Err(AppError::Validation(format!(
            "パスワードは{PASSWORD_MIN_CHARS}～{PASSWORD_MAX_CHARS}文字で入力してください。"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exam_date_is_optional_and_must_be_a_date() {
        assert_eq!(validate_exam_date(None).unwrap(), None);
        assert_eq!(validate_exam_date(Some(" ")).unwrap(), None);
        assert_eq!(
            validate_exam_date(Some("2026-10-18")).unwrap(),
            NaiveDate::from_ymd_opt(2026, 10, 18)
        );
        assert!(validate_exam_date(Some("2026-02-30")).is_err());
        assert!(validate_exam_date(Some("10/18")).is_err());
    }

    #[test]
    fn daily_goal_must_be_in_range() {
        assert_eq!(validate_daily_goal(None).unwrap(), None);
        assert_eq!(validate_daily_goal(Some(20)).unwrap(), Some(20));
        assert!(validate_daily_goal(Some(0)).is_err());
        assert!(validate_daily_goal(Some(DAILY_GOAL_MAX + 1)).is_err());
    }

    #[test]
    fn title_is_trimmed() {
        assert_eq!(validate_title("  令和5年 問1 ").unwrap(), "令和5年 問1");
    }

    #[test]
    fn title_rejects_empty_and_too_long() {
        assert!(validate_title("   ").is_err());
        assert!(validate_title(&"あ".repeat(100)).is_ok());
        assert!(validate_title(&"あ".repeat(101)).is_err());
    }

    #[test]
    fn response_allows_free_text_with_newlines() {
        assert_eq!(validate_response("ア").unwrap(), "ア");
        assert_eq!(validate_response("1行目\r\n2行目").unwrap(), "1行目\n2行目");
        assert!(validate_response(&"x".repeat(255)).is_ok());
        assert!(validate_response(&"x".repeat(256)).is_err());
        assert!(validate_response(" \n ").is_err());
        assert!(validate_response("").is_err());
    }

    #[test]
    fn optional_text_treats_blank_as_none() {
        assert_eq!(validate_note(Some("  ")).unwrap(), None);
        assert_eq!(validate_note(None).unwrap(), None);
        assert!(validate_note(Some(&"x".repeat(1001))).is_err());
        assert_eq!(validate_correct_answer(Some("イ")).unwrap().as_deref(), Some("イ"));
    }

    #[test]
    fn exam_name_rules() {
        assert_eq!(validate_exam_name(" 簿記2級 ").unwrap(), "簿記2級");
        assert!(validate_exam_name("").is_err());
        assert!(validate_exam_name(&"a".repeat(101)).is_err());
    }

    #[test]
    fn numbers_are_range_checked() {
        assert!(validate_question_number(0).is_err());
        assert!(validate_question_number(1).is_ok());
        assert!(validate_elapsed_seconds(-1).is_err());
        assert!(validate_elapsed_seconds(0).is_ok());
    }

    #[test]
    fn category_name_rules() {
        assert_eq!(validate_category_name(" 追加 ").unwrap(), "追加");
        assert!(validate_category_name("").is_err());
        assert!(validate_category_name(&"a".repeat(51)).is_err());
    }

    #[test]
    fn email_rules() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("user@localhost").is_err());
        assert!(validate_email("@example.com").is_err());
        assert!(validate_email("us er@example.com").is_err());
    }
}
