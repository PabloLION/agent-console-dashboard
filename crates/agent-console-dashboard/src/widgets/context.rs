//! Widget context providing shared state for widget rendering.
//!
//! The [`WidgetContext`] struct carries all the data that widgets may need
//! during rendering, including session state, timing, and API usage data.
//! It is passed by reference to each widget's `render` method.

use crate::Session;
use claude_usage::UsageData;
use std::time::Instant;

/// Shared context passed to widgets during rendering.
///
/// Contains references to application state that widgets may need to
/// display their content. All fields are borrowed to avoid cloning
/// large data structures on every render tick.
///
/// # Lifetime
///
/// The `'a` lifetime ties all borrowed data to the same scope,
/// ensuring the context does not outlive the data it references.
///
/// # Example
///
/// ```
/// use agent_console::widgets::context::WidgetContext;
/// use agent_console::Session;
///
/// let sessions = vec![Session::default()];
/// let ctx = WidgetContext {
///     sessions: &sessions,
///     now: std::time::Instant::now(),
///     selected_index: None,
///     usage: None,
/// };
/// assert_eq!(ctx.sessions.len(), 1);
/// ```
#[derive(Debug)]
pub struct WidgetContext<'a> {
    /// Currently tracked sessions.
    pub sessions: &'a [Session],

    /// Current instant for elapsed-time calculations.
    pub now: Instant,

    /// Index of the currently selected session, if any.
    pub selected_index: Option<usize>,

    /// API usage data from Anthropic, if available.
    ///
    /// `None` when usage data has not been fetched or credentials
    /// are unavailable.
    pub usage: Option<&'a UsageData>,
}

impl<'a> WidgetContext<'a> {
    /// Creates a new `WidgetContext` with the given sessions and current time.
    ///
    /// Usage and selection default to `None`.
    ///
    /// # Example
    ///
    /// ```
    /// use agent_console::widgets::context::WidgetContext;
    /// use agent_console::Session;
    ///
    /// let sessions = vec![Session::default()];
    /// let ctx = WidgetContext::new(&sessions);
    /// assert!(ctx.usage.is_none());
    /// assert!(ctx.selected_index.is_none());
    /// ```
    pub fn new(sessions: &'a [Session]) -> Self {
        Self {
            sessions,
            now: Instant::now(),
            selected_index: None,
            usage: None,
        }
    }

    /// Sets the selected session index.
    pub fn with_selected(mut self, index: usize) -> Self {
        self.selected_index = Some(index);
        self
    }

    /// Sets the usage data reference.
    pub fn with_usage(mut self, usage: &'a UsageData) -> Self {
        self.usage = Some(usage);
        self
    }

    /// Returns the currently selected session, if the index is valid.
    pub fn selected_session(&self) -> Option<&'a Session> {
        self.selected_index
            .and_then(|i| self.sessions.get(i))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentType, Session, Status};
    use claude_usage::{UsageData, UsagePeriod};
    use std::path::PathBuf;

    fn sample_sessions() -> Vec<Session> {
        vec![
            Session::new(
                "s1".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/tmp/a"),
            ),
            Session::new(
                "s2".to_string(),
                AgentType::ClaudeCode,
                PathBuf::from("/tmp/b"),
            ),
        ]
    }

    fn sample_usage() -> UsageData {
        UsageData {
            five_hour: UsagePeriod {
                utilization: 25.0,
                resets_at: None,
            },
            seven_day: UsagePeriod {
                utilization: 50.0,
                resets_at: None,
            },
            seven_day_sonnet: None,
            extra_usage: None,
        }
    }

    #[test]
    fn test_context_new_defaults() {
        let sessions = sample_sessions();
        let ctx = WidgetContext::new(&sessions);
        assert_eq!(ctx.sessions.len(), 2);
        assert!(ctx.selected_index.is_none());
        assert!(ctx.usage.is_none());
    }

    #[test]
    fn test_context_with_selected() {
        let sessions = sample_sessions();
        let ctx = WidgetContext::new(&sessions).with_selected(1);
        assert_eq!(ctx.selected_index, Some(1));
    }

    #[test]
    fn test_context_with_usage() {
        let sessions = sample_sessions();
        let usage = sample_usage();
        let ctx = WidgetContext::new(&sessions).with_usage(&usage);
        assert!(ctx.usage.is_some());
        let u = ctx.usage.expect("usage should be present");
        assert!((u.five_hour.utilization - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_context_selected_session_valid() {
        let sessions = sample_sessions();
        let ctx = WidgetContext::new(&sessions).with_selected(0);
        let selected = ctx.selected_session().expect("session at index 0");
        assert_eq!(selected.id, "s1");
    }

    #[test]
    fn test_context_selected_session_out_of_bounds() {
        let sessions = sample_sessions();
        let ctx = WidgetContext::new(&sessions).with_selected(99);
        assert!(ctx.selected_session().is_none());
    }

    #[test]
    fn test_context_selected_session_no_selection() {
        let sessions = sample_sessions();
        let ctx = WidgetContext::new(&sessions);
        assert!(ctx.selected_session().is_none());
    }

    #[test]
    fn test_context_empty_sessions() {
        let sessions: Vec<Session> = vec![];
        let ctx = WidgetContext::new(&sessions);
        assert!(ctx.sessions.is_empty());
        assert!(ctx.selected_session().is_none());
    }

    #[test]
    fn test_context_builder_chain() {
        let sessions = sample_sessions();
        let usage = sample_usage();
        let ctx = WidgetContext::new(&sessions)
            .with_selected(1)
            .with_usage(&usage);
        assert_eq!(ctx.selected_index, Some(1));
        assert!(ctx.usage.is_some());
        let selected = ctx.selected_session().expect("session at index 1");
        assert_eq!(selected.id, "s2");
    }

    #[test]
    fn test_context_accesses_session_status() {
        let mut sessions = sample_sessions();
        sessions[0].set_status(Status::Attention);
        let ctx = WidgetContext::new(&sessions).with_selected(0);
        let selected = ctx.selected_session().expect("session present");
        assert_eq!(selected.status, Status::Attention);
    }

    #[test]
    fn test_context_debug_format() {
        let sessions = sample_sessions();
        let ctx = WidgetContext::new(&sessions);
        let debug = format!("{:?}", ctx);
        assert!(debug.contains("WidgetContext"));
    }
}
