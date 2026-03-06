//! Adaptive polling interval and command channel for session monitoring

use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;

#[allow(dead_code)]
const POLL_INITIAL_INTERVAL: Duration = Duration::from_secs(2);
#[allow(dead_code)]
const POLL_MAX_INTERVAL: Duration = Duration::from_secs(60);
#[allow(dead_code)]
const POLL_BACKOFF_FACTOR: f64 = 1.5;
#[allow(dead_code)]
const POLL_STABLE_THRESHOLD: u32 = 3;

/// Manages adaptive polling intervals that back off when no changes are detected
#[derive(Debug)]
pub struct AdaptiveInterval {
    initial: Duration,
    current: Duration,
    max: Duration,
    backoff_factor: f64,
    stable_threshold: u32,
    stable_count: u32,
}

impl AdaptiveInterval {
    /// Create a new adaptive interval with custom parameters
    pub fn new(
        initial: Duration,
        max: Duration,
        backoff_factor: f64,
        stable_threshold: u32,
    ) -> Self {
        Self {
            initial,
            current: initial,
            max,
            backoff_factor,
            stable_threshold,
            stable_count: 0,
        }
    }

    /// Get the current interval duration
    pub fn current(&self) -> Duration {
        self.current
    }

    /// Record that no changes were detected; increases backoff if threshold is reached
    pub fn record_no_change(&mut self) {
        self.stable_count += 1;
        if self.stable_count >= self.stable_threshold {
            let next = (self.current.as_secs_f64() * self.backoff_factor) as u64;
            let next_duration = Duration::from_secs(next);
            self.current = next_duration.min(self.max);
            self.stable_count = 0;
        }
    }

    /// Record that a change was detected; reset to initial interval
    pub fn record_change(&mut self) {
        self.current = self.initial;
        self.stable_count = 0;
    }

    /// Reset interval to initial state (external trigger)
    pub fn reset(&mut self) {
        self.record_change();
    }
}

/// Command sent to the session poller thread
#[derive(Debug, Clone, Copy)]
pub enum PollCommand {
    /// Request an immediate poll
    PollNow,
    /// Stop the poller thread
    Stop,
}

/// Manages polling thread lifecycle and communication
pub struct SessionPoller {
    cmd_tx: mpsc::Sender<PollCommand>,
    #[allow(dead_code)]
    cmd_rx: Option<mpsc::Receiver<PollCommand>>,
    handle: Option<JoinHandle<()>>,
}

impl SessionPoller {
    /// Create a new poller (does not start the thread)
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            cmd_tx: tx,
            cmd_rx: Some(rx),
            handle: None,
        }
    }

    /// Send a poll command
    pub fn poll_now(&self) {
        let _ = self.cmd_tx.send(PollCommand::PollNow);
    }

    /// Stop the poller thread and wait for it to finish
    pub fn stop(&mut self) {
        let _ = self.cmd_tx.send(PollCommand::Stop);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }

    /// Check if the poller thread is running
    pub fn is_running(&self) -> bool {
        match &self.handle {
            Some(handle) => !handle.is_finished(),
            None => false,
        }
    }
}

impl Default for SessionPoller {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SessionPoller {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_interval_initial() {
        let interval =
            AdaptiveInterval::new(Duration::from_secs(2), Duration::from_secs(60), 1.5, 3);
        assert_eq!(interval.current(), Duration::from_secs(2));
    }

    #[test]
    fn test_adaptive_interval_record_no_change_increments_count() {
        let mut interval =
            AdaptiveInterval::new(Duration::from_secs(2), Duration::from_secs(60), 1.5, 3);
        assert_eq!(interval.stable_count, 0);
        interval.record_no_change();
        assert_eq!(interval.stable_count, 1);
        interval.record_no_change();
        assert_eq!(interval.stable_count, 2);
    }

    #[test]
    fn test_adaptive_interval_backoff_at_threshold() {
        let mut interval =
            AdaptiveInterval::new(Duration::from_secs(2), Duration::from_secs(60), 1.5, 3);
        interval.record_no_change();
        interval.record_no_change();
        interval.record_no_change();
        // After 3 calls: 2 * 1.5 = 3 seconds
        assert_eq!(interval.current(), Duration::from_secs(3));
        assert_eq!(interval.stable_count, 0);
    }

    #[test]
    fn test_adaptive_interval_multiple_backoffs() {
        let mut interval =
            AdaptiveInterval::new(Duration::from_secs(2), Duration::from_secs(60), 1.5, 3);
        // First backoff: 2 -> 3
        for _ in 0..3 {
            interval.record_no_change();
        }
        assert_eq!(interval.current(), Duration::from_secs(3));

        // Second backoff: 3 -> 4 (actually 4.5, but let's check)
        for _ in 0..3 {
            interval.record_no_change();
        }
        let expected = (3.0 * 1.5) as u64;
        assert_eq!(interval.current(), Duration::from_secs(expected));
    }

    #[test]
    fn test_adaptive_interval_respects_max() {
        let mut interval = AdaptiveInterval::new(
            Duration::from_secs(2),
            Duration::from_secs(60),
            1.5,
            1, // threshold of 1 for faster test
        );
        interval.record_no_change(); // 2 * 1.5 = 3
        interval.record_no_change(); // 3 * 1.5 = 4.5
        interval.record_no_change(); // 4 * 1.5 = 6
        interval.record_no_change(); // 6 * 1.5 = 9
        interval.record_no_change(); // 9 * 1.5 = 13.5
        interval.record_no_change(); // 13 * 1.5 = 19.5
        interval.record_no_change(); // 19 * 1.5 = 28.5
        interval.record_no_change(); // 28 * 1.5 = 42
        interval.record_no_change(); // 42 * 1.5 = 63 > 60, capped at 60
        assert!(interval.current() <= Duration::from_secs(60));
    }

    #[test]
    fn test_adaptive_interval_record_change_resets() {
        let mut interval =
            AdaptiveInterval::new(Duration::from_secs(2), Duration::from_secs(60), 1.5, 3);
        for _ in 0..3 {
            interval.record_no_change();
        }
        assert_eq!(interval.current(), Duration::from_secs(3));

        interval.record_change();
        assert_eq!(interval.current(), Duration::from_secs(2));
        assert_eq!(interval.stable_count, 0);
    }

    #[test]
    fn test_adaptive_interval_reset_is_alias() {
        let mut interval =
            AdaptiveInterval::new(Duration::from_secs(2), Duration::from_secs(60), 1.5, 3);
        for _ in 0..3 {
            interval.record_no_change();
        }
        assert_eq!(interval.current(), Duration::from_secs(3));

        interval.reset();
        assert_eq!(interval.current(), Duration::from_secs(2));
        assert_eq!(interval.stable_count, 0);
    }

    #[test]
    fn test_session_poller_new() {
        let poller = SessionPoller::new();
        assert!(!poller.is_running());
    }

    #[test]
    fn test_session_poller_is_running_false_initially() {
        let poller = SessionPoller::new();
        assert_eq!(poller.is_running(), false);
    }

    #[test]
    fn test_session_poller_stop_when_no_thread() {
        let mut poller = SessionPoller::new();
        poller.stop(); // Should not panic
        assert!(!poller.is_running());
    }

    #[test]
    fn test_session_poller_double_stop_safe() {
        let mut poller = SessionPoller::new();
        poller.stop();
        poller.stop(); // Should not panic
        assert!(!poller.is_running());
    }

    #[test]
    fn test_session_poller_drop_is_clean() {
        let poller = SessionPoller::new();
        drop(poller); // Should not panic
    }

    #[test]
    fn test_poll_initial_interval_constant() {
        assert_eq!(POLL_INITIAL_INTERVAL, Duration::from_secs(2));
    }

    #[test]
    fn test_poll_max_interval_constant() {
        assert_eq!(POLL_MAX_INTERVAL, Duration::from_secs(60));
    }

    #[test]
    fn test_poll_backoff_factor_constant() {
        assert_eq!(POLL_BACKOFF_FACTOR, 1.5);
    }

    #[test]
    fn test_poll_stable_threshold_constant() {
        assert_eq!(POLL_STABLE_THRESHOLD, 3);
    }

    #[test]
    fn test_adaptive_interval_with_constants() {
        let mut interval = AdaptiveInterval::new(
            POLL_INITIAL_INTERVAL,
            POLL_MAX_INTERVAL,
            POLL_BACKOFF_FACTOR,
            POLL_STABLE_THRESHOLD,
        );
        assert_eq!(interval.current(), Duration::from_secs(2));
        for _ in 0..POLL_STABLE_THRESHOLD {
            interval.record_no_change();
        }
        assert_eq!(interval.current(), Duration::from_secs(3));
    }
}
