//! Slows down guessing passwords: after a few failed sign-ins of a user name the next
//! attempts wait, longer with every further failure.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Failed sign-ins allowed before attempts are delayed.
pub const FREE_ATTEMPTS: u32 = 5;
/// Longest wait between two attempts.
pub const MAX_WAIT: Duration = Duration::from_secs(300);

#[derive(Default)]
pub struct Throttle {
    failures: HashMap<String, (u32, Instant)>,
}

impl Throttle {
    fn key(name: &str) -> String {
        name.trim().to_lowercase()
    }

    /// Seconds to wait before `name` may try again; None if it may now.
    pub fn wait(&self, name: &str, now: Instant) -> Option<u64> {
        let (n, last) = self.failures.get(&Self::key(name))?;
        if *n < FREE_ATTEMPTS {
            return None;
        }
        let wait = Duration::from_secs(1 << (n - FREE_ATTEMPTS).min(9)).min(MAX_WAIT);
        let left = wait.saturating_sub(now.saturating_duration_since(*last));
        (!left.is_zero()).then(|| left.as_secs().max(1))
    }

    pub fn failed(&mut self, name: &str, now: Instant) {
        let e = self.failures.entry(Self::key(name)).or_insert((0, now));
        e.0 += 1;
        e.1 = now;
        // Forget names that have not failed for a long time.
        if self.failures.len() > 10_000 {
            self.failures.retain(|_, (_, t)| now.saturating_duration_since(*t) < MAX_WAIT);
        }
    }

    pub fn succeeded(&mut self, name: &str) {
        self.failures.remove(&Self::key(name));
    }
}

#[cfg(test)]
mod tests;
