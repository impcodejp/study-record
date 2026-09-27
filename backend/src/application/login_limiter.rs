//! 試行回数の制限（ログインの総当たり対策・メール送信の連打対策）。
//!
//! 状態はメモリ上だけに持つ（サーバーを再起動するとリセットされる）。
//! 1 台のサーバーで動かす前提の簡易な仕組み。

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

/// キーごとの失敗記録。
#[derive(Debug, Clone)]
struct Entry {
    /// 失敗回数。
    failures: u32,
    /// 最初の失敗の時刻（この時刻から `window` の間を数える）。
    first_failure: Instant,
    /// ロックが解ける時刻。
    locked_until: Option<Instant>,
}

/// 一定時間内の失敗回数を数え、上限を超えたキーを一時的にロックする。
#[derive(Clone)]
pub struct AttemptLimiter {
    entries: Arc<Mutex<HashMap<String, Entry>>>,
    /// ロックまでに許す失敗回数。
    max_failures: u32,
    /// 失敗を数える期間とロックの時間。
    window: Duration,
}

impl AttemptLimiter {
    /// 制限を作る。
    pub fn new(max_failures: u32, window: Duration) -> Self {
        Self {
            entries: Arc::new(Mutex::new(HashMap::new())),
            max_failures: max_failures.max(1),
            window,
        }
    }

    /// キーがロック中なら残り時間を返す。
    pub fn locked_for(&self, key: &str) -> Option<Duration> {
        let now = Instant::now();
        let mut entries = self.lock();
        let entry = entries.get(key)?;
        match entry.locked_until {
            Some(until) if until > now => Some(until - now),
            Some(_) => {
                entries.remove(key);
                None
            }
            None => None,
        }
    }

    /// 失敗を 1 回記録する。上限に達したらロックし、true を返す。
    pub fn record_failure(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut entries = self.lock();
        // 古いエントリが溜まり続けないよう、ついでに掃除する。
        if entries.len() > 10_000 {
            let window = self.window;
            entries.retain(|_, e| {
                e.locked_until.is_some_and(|u| u > now) || now - e.first_failure < window
            });
        }
        let entry = entries.entry(key.to_string()).or_insert(Entry {
            failures: 0,
            first_failure: now,
            locked_until: None,
        });
        if now - entry.first_failure >= self.window {
            entry.failures = 0;
            entry.first_failure = now;
            entry.locked_until = None;
        }
        entry.failures += 1;
        if entry.failures >= self.max_failures {
            entry.locked_until = Some(now + self.window);
            return true;
        }
        false
    }

    /// 成功したので記録を消す。
    pub fn reset(&self, key: &str) {
        self.lock().remove(key);
    }

    /// 内部の表をロックする。ロックが壊れていても中身はそのまま使う。
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Entry>> {
        self.entries.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locks_after_max_failures() {
        let limiter = AttemptLimiter::new(3, Duration::from_secs(60));
        assert!(!limiter.record_failure("k"));
        assert!(!limiter.record_failure("k"));
        assert!(limiter.locked_for("k").is_none());
        assert!(limiter.record_failure("k"));
        assert!(limiter.locked_for("k").is_some());
        limiter.reset("k");
        assert!(limiter.locked_for("k").is_none());
    }
}
