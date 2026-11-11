//! Fixed-window rate limiter with express-rate-limit 7 MemoryStore semantics
//! and `standardHeaders: true` (draft-6) headers.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct RateLimiter {
    window: Duration,
    pub max: u64,
    clients: Mutex<HashMap<String, Client>>,
}

struct Client {
    hits: u64,
    reset: Instant,
}

pub struct Hit {
    pub limit: u64,
    pub remaining: u64,
    /// `Math.ceil((resetTime - now) / 1000)`, floored at 0.
    pub reset_secs: u64,
    pub window_secs: u64,
    pub exceeded: bool,
}

impl RateLimiter {
    pub fn new(window_ms: u64, max: u64) -> Self {
        RateLimiter {
            window: Duration::from_millis(window_ms),
            max,
            clients: Mutex::new(HashMap::new()),
        }
    }

    pub fn hit(&self, key: &str) -> Hit {
        let now = Instant::now();
        let mut map = self.clients.lock().unwrap_or_else(|e| e.into_inner());
        let c = map.entry(key.to_owned()).or_insert(Client {
            hits: 0,
            reset: now + self.window,
        });
        if c.reset <= now {
            c.hits = 0;
            c.reset = now + self.window;
        }
        c.hits += 1;
        let left_ms = c.reset.saturating_duration_since(now).as_millis() as u64;
        Hit {
            limit: self.max,
            remaining: self.max.saturating_sub(c.hits),
            reset_secs: left_ms.div_ceil(1000),
            window_secs: (self.window.as_millis() as u64).div_ceil(1000),
            exceeded: c.hits > self.max,
        }
    }

    /// Drop expired windows (Node rotates its maps every window).
    pub fn sweep(&self) {
        let now = Instant::now();
        self.clients
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .retain(|_, c| c.reset > now);
    }

    pub fn window(&self) -> Duration {
        self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_and_blocks() {
        let rl = RateLimiter::new(900_000, 2);
        let a = rl.hit("a");
        assert_eq!(
            (a.remaining, a.exceeded, a.reset_secs, a.window_secs),
            (1, false, 900, 900)
        );
        assert!(!rl.hit("a").exceeded);
        let c = rl.hit("a");
        assert!(c.exceeded);
        assert_eq!(c.remaining, 0);
        assert!(!rl.hit("b").exceeded);
    }

    #[test]
    fn window_resets() {
        let rl = RateLimiter::new(1, 1);
        rl.hit("a");
        std::thread::sleep(Duration::from_millis(5));
        assert!(!rl.hit("a").exceeded);
        std::thread::sleep(Duration::from_millis(5));
        rl.sweep();
        assert!(rl.clients.lock().unwrap().is_empty());
    }
}
