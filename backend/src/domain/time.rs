//! 日時の扱いをまとめたモジュール。
//!
//! データモデルの取り決め（機能一覧 5 章）により、保存形式が列ごとに異なる。
//! - `practices.created_at` / `completed_at` : 日本時間で保存
//! - `answers.answered_at`                   : UTC で保存し、表示・集計時に +9 時間する
//!
//! どちらも `YYYY-MM-DD HH:MM:SS` 形式の文字列で保存する（SQLite の日時関数と互換）。

use chrono::{DateTime, Duration, FixedOffset, NaiveDate, NaiveDateTime, Utc};

/// DB に保存する日時文字列の書式。
pub const DB_DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

/// 日本時間（UTC+9）のオフセット秒数。
const JST_OFFSET_SECONDS: i32 = 9 * 3600;

/// 日本時間のタイムゾーンを返す。
pub fn jst() -> FixedOffset {
    // 9 時間は常に有効なオフセットなので失敗しない。
    FixedOffset::east_opt(JST_OFFSET_SECONDS).expect("UTC+9 は有効なオフセット")
}

/// 現在の日本時間を返す。
pub fn now_jst() -> DateTime<FixedOffset> {
    Utc::now().with_timezone(&jst())
}

/// 現在の日本時間を DB 保存用の文字列で返す（`practices` 用）。
pub fn now_jst_string() -> String {
    now_jst().format(DB_DATETIME_FORMAT).to_string()
}

/// 現在の UTC を DB 保存用の文字列で返す（`answers` 用）。
pub fn now_utc_string() -> String {
    Utc::now().format(DB_DATETIME_FORMAT).to_string()
}

/// 今日（日本時間）の日付を返す。
pub fn today_jst() -> NaiveDate {
    now_jst().date_naive()
}

/// UTC で保存された日時文字列を日本時間の文字列に変換する。
///
/// 解析できない文字列は、そのまま返す（表示を壊さないため）。
pub fn utc_string_to_jst_string(utc: &str) -> String {
    match NaiveDateTime::parse_from_str(utc, DB_DATETIME_FORMAT) {
        Ok(naive) => (naive + Duration::hours(9))
            .format(DB_DATETIME_FORMAT)
            .to_string(),
        Err(_) => utc.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_to_jst_adds_nine_hours_across_date_boundary() {
        assert_eq!(
            utc_string_to_jst_string("2026-01-31 20:30:00"),
            "2026-02-01 05:30:00"
        );
    }

    #[test]
    fn invalid_string_is_returned_as_is() {
        assert_eq!(utc_string_to_jst_string("abc"), "abc");
    }
}
