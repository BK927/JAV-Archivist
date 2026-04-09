use std::time::Duration;

pub struct RateLimiter {
    base_delay: Duration,
    max_delay: Duration,
    current: Duration,
}

impl RateLimiter {
    pub fn new(base_delay: Duration, max_delay: Duration) -> Self {
        Self {
            base_delay,
            max_delay,
            current: base_delay,
        }
    }

    #[cfg(test)]
    pub fn current_delay(&self) -> Duration {
        self.current
    }

    pub async fn wait(&self) {
        tokio::time::sleep(self.current).await;
    }

    pub fn success(&mut self) {
        self.current = self.base_delay;
    }

    pub fn failure(&mut self) {
        self.current = (self.current * 2).min(self.max_delay);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_starts_at_base_delay() {
        let rl = RateLimiter::new(Duration::from_secs(3), Duration::from_secs(60));
        assert_eq!(rl.current_delay(), Duration::from_secs(3));
    }

    #[test]
    fn test_rate_limiter_failure_doubles_delay() {
        let mut rl = RateLimiter::new(Duration::from_secs(3), Duration::from_secs(60));
        rl.failure();
        assert_eq!(rl.current_delay(), Duration::from_secs(6));
        rl.failure();
        assert_eq!(rl.current_delay(), Duration::from_secs(12));
    }

    #[test]
    fn test_rate_limiter_failure_caps_at_max() {
        let mut rl = RateLimiter::new(Duration::from_secs(3), Duration::from_secs(10));
        rl.failure();
        rl.failure();
        rl.failure();
        assert_eq!(rl.current_delay(), Duration::from_secs(10));
    }

    #[test]
    fn test_rate_limiter_success_resets_to_base() {
        let mut rl = RateLimiter::new(Duration::from_secs(3), Duration::from_secs(60));
        rl.failure();
        rl.failure();
        assert_eq!(rl.current_delay(), Duration::from_secs(12));
        rl.success();
        assert_eq!(rl.current_delay(), Duration::from_secs(3));
    }
}
