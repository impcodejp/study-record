//! 学習計画の業務ルール（忘却曲線にもとづく復習日・習熟度・連続学習日数）。
//!
//! DB や時計に依存しない純粋な計算だけを置き、単体テストで振る舞いを固定する。
//!
//! ## 復習日の決め方（間隔反復）
//!
//! 問題ごとに、最新の回答から数えて **連続で正解した回数** を「習熟度」（0〜5）とする。
//! 最後に解いた日から、習熟度に応じた日数が経つと復習の時期になる。
//!
//! | 習熟度 | 意味 | 次の復習まで |
//! |---|---|---|
//! | 0 | 最新が不正解 | 1 日 |
//! | 1 | 1 回連続で正解 | 3 日 |
//! | 2 | 2 回連続で正解 | 7 日 |
//! | 3 | 3 回連続で正解 | 14 日 |
//! | 4 | 4 回連続で正解 | 30 日 |
//! | 5 | 5 回以上連続で正解（習得済み） | 60 日 |
//!
//! 一度でも間違えると習熟度は 0 に戻る（忘れかけた問題を短い間隔で解き直させるため）。

use std::collections::BTreeSet;

use chrono::{Duration, NaiveDate};

/// 習熟度の上限（この値で「習得済み」とみなす）。
pub const MAX_MASTERY_LEVEL: u8 = 5;

/// 習熟度ごとの、次の復習までの日数（添字が習熟度）。
const REVIEW_INTERVAL_DAYS: [i64; MAX_MASTERY_LEVEL as usize + 1] = [1, 3, 7, 14, 30, 60];

/// 正誤の履歴（古い順）から習熟度を求める。
///
/// 最新から数えて連続で正解した回数を、[`MAX_MASTERY_LEVEL`] を上限として返す。
pub fn mastery_level(results_oldest_first: &[bool]) -> u8 {
    let streak = results_oldest_first
        .iter()
        .rev()
        .take_while(|correct| **correct)
        .count();
    streak.min(MAX_MASTERY_LEVEL as usize) as u8
}

/// 習熟度に応じた、次の復習までの日数を返す。
pub fn review_interval_days(level: u8) -> i64 {
    REVIEW_INTERVAL_DAYS[level.min(MAX_MASTERY_LEVEL) as usize]
}

/// 最後に解いた日と習熟度から、次に復習する日を求める。
pub fn next_review_date(last_answered_on: NaiveDate, level: u8) -> NaiveDate {
    last_answered_on + Duration::days(review_interval_days(level))
}

/// 連続学習日数。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Streak {
    /// 今の連続学習日数。今日まだ解いていなくても、昨日まで続いていれば途切れていない扱いにする。
    pub current: i64,
    /// これまでの最長の連続学習日数。
    pub longest: i64,
}

/// 学習した日の集合から、連続学習日数を求める。
///
/// 今日まだ学習していない場合は、昨日までの連続日数を「今の連続日数」とする
/// （1 日の途中で連続記録が 0 に見えて、やる気をそがないようにするため）。
pub fn study_streak(study_days: &BTreeSet<NaiveDate>, today: NaiveDate) -> Streak {
    // 最長記録：日付順に見て、前日と連続していれば伸ばす。
    let mut longest = 0;
    let mut run = 0;
    let mut previous: Option<NaiveDate> = None;
    for day in study_days.iter().filter(|day| **day <= today) {
        run = match previous {
            Some(prev) if *day - prev == Duration::days(1) => run + 1,
            _ => 1,
        };
        longest = longest.max(run);
        previous = Some(*day);
    }

    // 今の記録：今日（無ければ昨日）からさかのぼって数える。
    let start = if study_days.contains(&today) {
        Some(today)
    } else {
        let yesterday = today - Duration::days(1);
        study_days.contains(&yesterday).then_some(yesterday)
    };
    let mut current = 0;
    if let Some(mut day) = start {
        while study_days.contains(&day) {
            current += 1;
            day -= Duration::days(1);
        }
    }
    Streak { current, longest }
}

/// 試験日までの残り日数を返す（試験日当日は 0、過ぎていれば負の値）。
pub fn days_until(exam_date: NaiveDate, today: NaiveDate) -> i64 {
    (exam_date - today).num_days()
}

/// 準備度（0〜100%）。解いたことのある問題の習熟度の平均を、上限に対する割合で表す。
///
/// すべての問題を習得済み（習熟度が上限）にすると 100%。問題が無ければ 0%。
pub fn readiness_percent(levels: &[u8]) -> f64 {
    if levels.is_empty() {
        return 0.0;
    }
    let total: u32 = levels.iter().map(|level| u32::from((*level).min(MAX_MASTERY_LEVEL))).sum();
    f64::from(total) / (levels.len() as f64 * f64::from(MAX_MASTERY_LEVEL)) * 100.0
}

/// すべての問題を習得済みにするまでに必要な正解の回数（習熟度の不足分の合計）。
pub fn remaining_correct_answers(levels: &[u8]) -> i64 {
    levels
        .iter()
        .map(|level| i64::from(MAX_MASTERY_LEVEL - (*level).min(MAX_MASTERY_LEVEL)))
        .sum()
}

/// 試験日までに `remaining` 回の正解を積み上げるのに必要な、1 日あたりの問題数（切り上げ）。
///
/// 試験日が今日以前なら計算できないため `None`。
pub fn required_daily_pace(remaining: i64, days_left: i64) -> Option<i64> {
    (days_left > 0).then(|| (remaining + days_left - 1) / days_left)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn mastery_counts_trailing_correct_answers() {
        assert_eq!(mastery_level(&[]), 0);
        assert_eq!(mastery_level(&[false]), 0);
        assert_eq!(mastery_level(&[true]), 1);
        assert_eq!(mastery_level(&[true, true, false]), 0);
        assert_eq!(mastery_level(&[false, true, true]), 2);
        assert_eq!(mastery_level(&[true; 9]), MAX_MASTERY_LEVEL);
    }

    #[test]
    fn next_review_grows_with_mastery() {
        let day = date("2026-10-01");
        assert_eq!(next_review_date(day, 0), date("2026-10-02"));
        assert_eq!(next_review_date(day, 1), date("2026-10-04"));
        assert_eq!(next_review_date(day, 2), date("2026-10-08"));
        assert_eq!(next_review_date(day, 5), date("2026-11-30"));
        // 上限を超える値は上限として扱う
        assert_eq!(review_interval_days(99), 60);
    }

    #[test]
    fn streak_continues_until_today_or_yesterday() {
        let days: BTreeSet<NaiveDate> = ["2026-09-25", "2026-09-26", "2026-09-28", "2026-09-29", "2026-09-30"]
            .iter()
            .map(|d| date(d))
            .collect();
        // 今日（9/30）まで 3 日連続、最長も 3 日
        assert_eq!(study_streak(&days, date("2026-09-30")), Streak { current: 3, longest: 3 });
        // 今日（10/1）はまだ解いていないが、昨日まで続いているので途切れていない
        assert_eq!(study_streak(&days, date("2026-10-01")), Streak { current: 3, longest: 3 });
        // 2 日空くと途切れる
        assert_eq!(study_streak(&days, date("2026-10-02")), Streak { current: 0, longest: 3 });
        assert_eq!(study_streak(&BTreeSet::new(), date("2026-10-02")), Streak::default());
    }

    #[test]
    fn readiness_and_pace() {
        assert_eq!(readiness_percent(&[]), 0.0);
        assert_eq!(readiness_percent(&[5, 5]), 100.0);
        assert_eq!(readiness_percent(&[0, 5]), 50.0);
        assert_eq!(readiness_percent(&[1, 2, 3, 4]), 50.0);
        assert_eq!(remaining_correct_answers(&[0, 3, 5, 9]), 7);
        assert_eq!(required_daily_pace(7, 3), Some(3));
        assert_eq!(required_daily_pace(6, 3), Some(2));
        assert_eq!(required_daily_pace(0, 3), Some(0));
        assert_eq!(required_daily_pace(7, 0), None);
    }

    #[test]
    fn days_until_exam() {
        assert_eq!(days_until(date("2026-10-18"), date("2026-10-01")), 17);
        assert_eq!(days_until(date("2026-10-01"), date("2026-10-01")), 0);
        assert_eq!(days_until(date("2026-09-30"), date("2026-10-01")), -1);
    }
}
