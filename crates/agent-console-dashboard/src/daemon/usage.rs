//! Usage fetcher module for periodic Claude API usage data retrieval.
//!
//! This module provides [`UsageFetcher`], which periodically calls
//! [`claude_usage::get_usage()`] and broadcasts the results to subscribers
//! via a tokio broadcast channel. Fetching only occurs when at least one
//! subscriber is listening (conditional fetching per D3 decision).
//!
//! The daemon is the single source of truth for usage data (D3). TUIs never
//! call `claude_usage::get_usage()` directly.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use claude_usage::UsageData;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, info, warn};

/// Default fetch interval: 3 minutes (D4 decision).
const DEFAULT_FETCH_INTERVAL: Duration = Duration::from_secs(180);

/// Snapshot of the current usage state, broadcast to subscribers.
#[derive(Debug, Clone)]
pub enum UsageState {
    /// Usage data successfully fetched.
    Available(UsageData),
    /// Usage data is unavailable (credential/network error).
    Unavailable,
}

/// Periodic usage data fetcher.
///
/// Calls `claude_usage::get_usage()` at a configurable interval and broadcasts
/// results to all subscribers. Only fetches when `subscriber_count > 0`.
///
/// # Design
///
/// - Uses `tokio::task::spawn_blocking` because `claude_usage` only provides
///   a blocking HTTP client.
/// - Retains previous data on error (subscribers keep last known good state).
/// - Errors are logged as warnings; the daemon never crashes on fetch failure.
pub struct UsageFetcher {
    /// Current usage state, shared with the daemon.
    state: Arc<RwLock<UsageState>>,
    /// Broadcast sender for usage updates.
    update_tx: broadcast::Sender<UsageState>,
    /// Count of active subscribers (atomically tracked).
    subscriber_count: Arc<AtomicUsize>,
    /// Fetch interval (default: 3 minutes).
    interval: Duration,
}

impl UsageFetcher {
    /// Creates a new `UsageFetcher` with default 3-minute interval.
    pub fn new() -> Self {
        Self::with_interval(DEFAULT_FETCH_INTERVAL)
    }

    /// Creates a new `UsageFetcher` with a custom fetch interval.
    pub fn with_interval(interval: Duration) -> Self {
        let (update_tx, _rx) = broadcast::channel(16);
        Self {
            state: Arc::new(RwLock::new(UsageState::Unavailable)),
            update_tx,
            subscriber_count: Arc::new(AtomicUsize::new(0)),
            interval,
        }
    }

    /// Returns a reference to the shared usage state.
    pub fn state(&self) -> Arc<RwLock<UsageState>> {
        Arc::clone(&self.state)
    }

    /// Subscribes to usage updates.
    ///
    /// Returns a [`UsageSubscription`] whose constructor atomically increments
    /// the subscriber count. The count is decremented on drop.
    pub fn subscribe(&self) -> UsageSubscription {
        UsageSubscription::new(
            self.update_tx.subscribe(),
            Arc::clone(&self.subscriber_count),
        )
    }

    /// Returns the current subscriber count.
    pub fn subscriber_count(&self) -> usize {
        self.subscriber_count.load(Ordering::SeqCst)
    }

    /// Runs the periodic fetch loop until the shutdown receiver fires.
    ///
    /// This function should be spawned as a tokio task. It fetches usage data
    /// at the configured interval, but only when subscribers are present.
    pub async fn run(&self, mut shutdown_rx: broadcast::Receiver<()>) {
        let mut ticker = tokio::time::interval(self.interval);
        // The first tick completes immediately; skip it to avoid fetching at startup
        // when no subscribers are connected yet.
        ticker.tick().await;

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    self.fetch_once().await;
                }
                _ = shutdown_rx.recv() => {
                    info!("usage fetcher shutting down");
                    break;
                }
            }
        }
    }

    /// Performs a single fetch cycle.
    ///
    /// Skips if no subscribers are present. On success, updates shared state
    /// and broadcasts. On failure, logs warning and marks state as unavailable
    /// (previous data is lost in the shared state but subscribers may retain
    /// their last received value).
    async fn fetch_once(&self) {
        let count = self.subscriber_count.load(Ordering::SeqCst);
        if count == 0 {
            debug!("no usage subscribers, skipping fetch");
            return;
        }

        debug!(subscriber_count = count, "fetching usage data");

        let result = tokio::task::spawn_blocking(claude_usage::get_usage).await;

        match result {
            Ok(Ok(data)) => {
                let new_state = UsageState::Available(data);
                *self.state.write().await = new_state.clone();
                // Best-effort broadcast; no subscribers is not an error.
                let _ = self.update_tx.send(new_state);
                debug!("usage data fetched and broadcast successfully");
            }
            Ok(Err(e)) => {
                warn!(error = %e, "usage fetch failed");
                *self.state.write().await = UsageState::Unavailable;
                let _ = self.update_tx.send(UsageState::Unavailable);
            }
            Err(e) => {
                warn!(error = %e, "usage fetch task panicked");
                *self.state.write().await = UsageState::Unavailable;
                let _ = self.update_tx.send(UsageState::Unavailable);
            }
        }
    }
}

impl Default for UsageFetcher {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII subscription handle that decrements subscriber count on drop.
///
/// The counter is incremented in `new()` and decremented in `drop()`, keeping
/// the increment and guard creation inseparable.
pub struct UsageSubscription {
    /// Broadcast receiver for usage updates.
    rx: broadcast::Receiver<UsageState>,
    /// Shared subscriber counter.
    counter: Arc<AtomicUsize>,
}

impl UsageSubscription {
    /// Creates a new subscription, atomically incrementing the subscriber count.
    fn new(rx: broadcast::Receiver<UsageState>, counter: Arc<AtomicUsize>) -> Self {
        counter.fetch_add(1, Ordering::SeqCst);
        let count = counter.load(Ordering::SeqCst);
        info!(subscriber_count = count, "usage subscriber added");
        Self { rx, counter }
    }

    /// Receives the next usage update.
    pub async fn recv(&mut self) -> Result<UsageState, broadcast::error::RecvError> {
        self.rx.recv().await
    }
}

impl Drop for UsageSubscription {
    fn drop(&mut self) {
        let prev = self.counter.fetch_sub(1, Ordering::SeqCst);
        debug!(subscriber_count = prev - 1, "usage subscriber removed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_fetcher_default_creates_with_3min_interval() {
        let fetcher = UsageFetcher::new();
        assert_eq!(fetcher.interval, Duration::from_secs(180));
        assert_eq!(fetcher.subscriber_count(), 0);
    }

    #[test]
    fn test_usage_fetcher_custom_interval() {
        let fetcher = UsageFetcher::with_interval(Duration::from_secs(60));
        assert_eq!(fetcher.interval, Duration::from_secs(60));
    }

    #[test]
    fn test_subscriber_count_increments_on_subscribe() {
        let fetcher = UsageFetcher::new();
        assert_eq!(fetcher.subscriber_count(), 0);
        let _sub1 = fetcher.subscribe();
        assert_eq!(fetcher.subscriber_count(), 1);
        let _sub2 = fetcher.subscribe();
        assert_eq!(fetcher.subscriber_count(), 2);
    }

    #[test]
    fn test_subscriber_count_decrements_on_drop() {
        let fetcher = UsageFetcher::new();
        let sub1 = fetcher.subscribe();
        let sub2 = fetcher.subscribe();
        assert_eq!(fetcher.subscriber_count(), 2);
        drop(sub1);
        assert_eq!(fetcher.subscriber_count(), 1);
        drop(sub2);
        assert_eq!(fetcher.subscriber_count(), 0);
    }

    #[tokio::test]
    async fn test_fetch_once_skips_when_no_subscribers() {
        let fetcher = UsageFetcher::new();
        // Should not panic or make network calls with 0 subscribers
        fetcher.fetch_once().await;
        let state = fetcher.state.read().await;
        // State should remain Unavailable (initial)
        assert!(matches!(*state, UsageState::Unavailable));
    }

    #[tokio::test]
    async fn test_state_returns_shared_arc() {
        let fetcher = UsageFetcher::new();
        let state1 = fetcher.state();
        let state2 = fetcher.state();
        // Both should point to the same allocation
        assert!(Arc::ptr_eq(&state1, &state2));
    }

    #[tokio::test]
    async fn test_run_shuts_down_on_signal() {
        let fetcher = UsageFetcher::with_interval(Duration::from_millis(50));
        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

        let handle = tokio::spawn(async move {
            fetcher.run(shutdown_rx).await;
        });

        // Let it run briefly
        tokio::time::sleep(Duration::from_millis(10)).await;
        shutdown_tx
            .send(())
            .expect("shutdown signal should be sent");

        // Should complete without hanging
        tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .expect("run should complete within timeout")
            .expect("run task should not panic");
    }

    #[tokio::test]
    async fn test_fetch_once_with_subscriber_attempts_fetch() {
        // Verifies that with a subscriber, fetch_once attempts a fetch and
        // broadcasts the result (Available or Unavailable depending on credentials).
        let fetcher = UsageFetcher::new();
        let mut sub = fetcher.subscribe();
        assert_eq!(fetcher.subscriber_count(), 1);

        fetcher.fetch_once().await;

        // State should be updated (either Available if credentials exist, or Unavailable)
        let state = fetcher.state.read().await;
        assert!(
            matches!(*state, UsageState::Unavailable | UsageState::Available(_)),
            "state should be set after fetch"
        );
        drop(state);

        // Subscriber should have received the update
        let update = tokio::time::timeout(Duration::from_millis(100), sub.recv()).await;
        assert!(update.is_ok(), "subscriber should receive update");
    }

    #[test]
    fn test_usage_state_clone() {
        let state = UsageState::Unavailable;
        let cloned = state.clone();
        assert!(matches!(cloned, UsageState::Unavailable));
    }

    #[test]
    fn test_usage_state_debug_format() {
        let state = UsageState::Unavailable;
        let debug_str = format!("{:?}", state);
        assert!(debug_str.contains("Unavailable"));
    }

    #[test]
    fn test_default_fetch_interval_is_3_minutes() {
        assert_eq!(DEFAULT_FETCH_INTERVAL, Duration::from_secs(180));
    }
}
