//! Session status widget for the dashboard status bar.
//!
//! Displays all tracked sessions in a compact horizontal format with
//! status symbols, colors, and elapsed time for sessions needing attention.
//!
//! # Format
//!
//! ```text
//! proj-a: ● | proj-b: 2m 34s | proj-c: ? 1m 12s
//! ```
//!
//! # Status Display
//!
//! | Status    | Display        | Color  |
//! |-----------|----------------|--------|
//! | Working   | `●`            | Green  |
//! | Attention | elapsed time   | Yellow |
//! | Question  | `?` + elapsed  | Blue   |
//! | Closed    | `×`            | Gray   |

use crate::widgets::{Widget, WidgetContext};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use std::time::Duration;

/// Widget displaying all session statuses in a compact horizontal line.
///
/// Sessions are separated by ` | ` and each shows a name plus a
/// status indicator. Working sessions show `●`, Attention/Question
/// sessions show elapsed time, and Closed sessions show `×`.
pub struct SessionStatusWidget;

impl SessionStatusWidget {
    /// Creates a new `SessionStatusWidget`.
    pub fn new() -> Self {
        Self
    }
}

impl Default for SessionStatusWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for SessionStatusWidget {
    fn render(&self, width: u16, context: &WidgetContext) -> Line<'_> {
        if context.sessions.is_empty() {
            return Line::raw("(no sessions)");
        }

        let entries: Vec<SessionEntry> = context
            .sessions
            .iter()
            .map(|s| {
                let elapsed = context.now.duration_since(s.since);
                SessionEntry {
                    name: extract_name(&s.id),
                    status: s.status,
                    elapsed,
                }
            })
            .collect();

        build_line(&entries, width)
    }

    fn id(&self) -> &'static str {
        "session-status"
    }

    fn min_width(&self) -> u16 {
        20
    }
}

/// Intermediate representation for a single session entry.
struct SessionEntry {
    name: String,
    status: crate::Status,
    elapsed: Duration,
}

/// Extracts a display name from a session ID.
///
/// Uses the last path component if the ID looks like a path,
/// otherwise returns the ID itself.
fn extract_name(id: &str) -> String {
    id.rsplit('/')
        .next()
        .unwrap_or(id)
        .to_string()
}

/// Formats a [`Duration`] as a human-readable elapsed string.
///
/// - `< 60s` : `Xs`
/// - `< 1h`  : `Xm Ys`
/// - `>= 1h` : `Xh Ym`
pub fn format_duration(d: Duration) -> String {
    let total = d.as_secs();
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;

    if h > 0 {
        format!("{h}h {m}m")
    } else if m > 0 {
        format!("{m}m {s}s")
    } else {
        format!("{s}s")
    }
}

/// Returns the status color for a given session status.
fn status_color(status: crate::Status) -> Color {
    match status {
        crate::Status::Working => Color::Green,
        crate::Status::Attention => Color::Yellow,
        crate::Status::Question => Color::Blue,
        crate::Status::Closed => Color::Gray,
    }
}

/// Builds a styled [`Span`] for a session's status indicator.
fn status_span(entry: &SessionEntry) -> Span<'static> {
    let color = status_color(entry.status);
    match entry.status {
        crate::Status::Working => Span::styled("●".to_string(), Style::default().fg(color)),
        crate::Status::Attention => {
            Span::styled(format_duration(entry.elapsed), Style::default().fg(color))
        }
        crate::Status::Question => {
            Span::styled(format!("? {}", format_duration(entry.elapsed)), Style::default().fg(color))
        }
        crate::Status::Closed => Span::styled("×".to_string(), Style::default().fg(color)),
    }
}

/// Computes the full (untruncated) width of a single entry: `name: status`.
fn entry_full_width(name: &str, status_text: &str) -> usize {
    // "name: status"
    name.len() + 2 + status_text.len()
}

/// Truncates a name to fit within `max_chars`, keeping at least 3 chars
/// plus an ellipsis character.
fn truncate_name(name: &str, max_chars: usize) -> String {
    if name.len() <= max_chars {
        name.to_string()
    } else if max_chars <= 3 {
        name.chars().take(max_chars).collect()
    } else {
        let keep = max_chars - 1; // room for ellipsis
        let truncated: String = name.chars().take(keep).collect();
        format!("{truncated}\u{2026}")
    }
}

/// Builds the full horizontal [`Line`] from session entries, fitting
/// within the given `width`.
fn build_line(entries: &[SessionEntry], width: u16) -> Line<'static> {
    let w = width as usize;

    // Pre-compute status texts.
    let status_texts: Vec<String> = entries
        .iter()
        .map(|e| {
            let span = status_span(e);
            span.content.to_string()
        })
        .collect();

    // Separator overhead: " | " = 3 chars between each pair.
    let separator_total = if entries.len() > 1 {
        (entries.len() - 1) * 3
    } else {
        0
    };

    // Total width needed if names are not truncated.
    let full_widths: Vec<usize> = entries
        .iter()
        .zip(status_texts.iter())
        .map(|(e, st)| entry_full_width(&e.name, st))
        .collect();
    let total_full: usize = full_widths.iter().sum::<usize>() + separator_total;

    // Determine available space for names if we need to truncate.
    let names: Vec<String> = if total_full <= w {
        entries.iter().map(|e| e.name.clone()).collect()
    } else {
        // Fixed overhead per entry: ": " + status_text
        let fixed_per_entry: Vec<usize> = status_texts.iter().map(|st| 2 + st.len()).collect();
        let total_fixed: usize = fixed_per_entry.iter().sum::<usize>() + separator_total;
        let available_for_names = w.saturating_sub(total_fixed);
        let per_name = if entries.is_empty() {
            0
        } else {
            available_for_names / entries.len()
        };
        let max_name = per_name.max(3);
        entries
            .iter()
            .map(|e| truncate_name(&e.name, max_name))
            .collect()
    };

    // Build spans.
    let mut spans: Vec<Span<'static>> = Vec::with_capacity(entries.len() * 3);
    for (i, (entry, name)) in entries.iter().zip(names.iter()).enumerate() {
        if i > 0 {
            spans.push(Span::raw(" | "));
        }
        spans.push(Span::raw(format!("{name}: ")));
        spans.push(status_span(entry));
    }

    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentType, Session, Status};
    use std::path::PathBuf;
    use std::time::Instant;

    fn make_session(id: &str, status: Status) -> Session {
        let mut s = Session::new(
            id.to_string(),
            AgentType::ClaudeCode,
            PathBuf::from("/tmp/test"),
        );
        s.status = status;
        s
    }

    fn ctx_with_sessions(sessions: &[Session]) -> WidgetContext<'_> {
        WidgetContext {
            sessions,
            now: Instant::now(),
            selected_index: None,
            usage: None,
        }
    }

    // -- Widget trait basics --

    #[test]
    fn test_widget_id() {
        let w = SessionStatusWidget::new();
        assert_eq!(w.id(), "session-status");
    }

    #[test]
    fn test_widget_min_width() {
        let w = SessionStatusWidget::new();
        assert_eq!(w.min_width(), 20);
    }

    #[test]
    fn test_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<SessionStatusWidget>();
    }

    // -- Empty sessions --

    #[test]
    fn test_render_empty_sessions() {
        let sessions: Vec<Session> = vec![];
        let ctx = ctx_with_sessions(&sessions);
        let w = SessionStatusWidget::new();
        let line = w.render(80, &ctx);
        assert_eq!(line.to_string(), "(no sessions)");
    }

    // -- Single session --

    #[test]
    fn test_render_single_working_session() {
        let sessions = vec![make_session("proj-a", Status::Working)];
        let ctx = ctx_with_sessions(&sessions);
        let w = SessionStatusWidget::new();
        let line = w.render(80, &ctx);
        let text = line.to_string();
        assert!(text.contains("proj-a: "), "expected name prefix, got: {text}");
        assert!(text.contains('●'), "expected working symbol, got: {text}");
    }

    // -- Three sessions with separator --

    #[test]
    fn test_render_three_sessions_has_separators() {
        let sessions = vec![
            make_session("alpha", Status::Working),
            make_session("beta", Status::Closed),
            make_session("gamma", Status::Working),
        ];
        let ctx = ctx_with_sessions(&sessions);
        let w = SessionStatusWidget::new();
        let line = w.render(120, &ctx);
        let text = line.to_string();
        // Count separators: should be 2 for 3 sessions.
        let separator_count = text.matches(" | ").count();
        assert_eq!(separator_count, 2, "expected 2 separators, got: {text}");
    }

    // -- Status symbols and colors --

    #[test]
    fn test_working_status_symbol_and_color() {
        let entry = SessionEntry {
            name: "test".to_string(),
            status: Status::Working,
            elapsed: Duration::from_secs(0),
        };
        let span = status_span(&entry);
        assert_eq!(span.content.as_ref(), "●");
        assert_eq!(span.style.fg, Some(Color::Green));
    }

    #[test]
    fn test_attention_status_shows_elapsed_yellow() {
        let entry = SessionEntry {
            name: "test".to_string(),
            status: Status::Attention,
            elapsed: Duration::from_secs(154),
        };
        let span = status_span(&entry);
        assert_eq!(span.content.as_ref(), "2m 34s");
        assert_eq!(span.style.fg, Some(Color::Yellow));
    }

    #[test]
    fn test_question_status_shows_question_mark_blue() {
        let entry = SessionEntry {
            name: "test".to_string(),
            status: Status::Question,
            elapsed: Duration::from_secs(72),
        };
        let span = status_span(&entry);
        assert_eq!(span.content.as_ref(), "? 1m 12s");
        assert_eq!(span.style.fg, Some(Color::Blue));
    }

    #[test]
    fn test_closed_status_symbol_and_color() {
        let entry = SessionEntry {
            name: "test".to_string(),
            status: Status::Closed,
            elapsed: Duration::from_secs(0),
        };
        let span = status_span(&entry);
        assert_eq!(span.content.as_ref(), "×");
        assert_eq!(span.style.fg, Some(Color::Gray));
    }

    // -- Elapsed time formatting --

    #[test]
    fn test_format_duration_seconds_only() {
        assert_eq!(format_duration(Duration::from_secs(0)), "0s");
        assert_eq!(format_duration(Duration::from_secs(45)), "45s");
        assert_eq!(format_duration(Duration::from_secs(59)), "59s");
    }

    #[test]
    fn test_format_duration_minutes_seconds() {
        assert_eq!(format_duration(Duration::from_secs(60)), "1m 0s");
        assert_eq!(format_duration(Duration::from_secs(154)), "2m 34s");
        assert_eq!(format_duration(Duration::from_secs(3599)), "59m 59s");
    }

    #[test]
    fn test_format_duration_hours_minutes() {
        assert_eq!(format_duration(Duration::from_secs(3600)), "1h 0m");
        assert_eq!(format_duration(Duration::from_secs(3661)), "1h 1m");
        assert_eq!(format_duration(Duration::from_secs(7384)), "2h 3m");
    }

    // -- Name truncation --

    #[test]
    fn test_truncate_name_no_truncation() {
        assert_eq!(truncate_name("hello", 10), "hello");
        assert_eq!(truncate_name("abc", 3), "abc");
    }

    #[test]
    fn test_truncate_name_with_ellipsis() {
        let result = truncate_name("my-long-project", 6);
        // 5 chars + ellipsis
        assert_eq!(result, "my-lo\u{2026}");
        assert_eq!(result.chars().count(), 6);
    }

    #[test]
    fn test_truncate_name_very_short_max() {
        assert_eq!(truncate_name("hello", 2), "he");
        assert_eq!(truncate_name("hello", 3), "hel");
    }

    #[test]
    fn test_render_narrow_width_truncates_names() {
        let sessions = vec![
            make_session("very-long-project-name", Status::Working),
            make_session("another-long-name", Status::Closed),
        ];
        let ctx = ctx_with_sessions(&sessions);
        let w = SessionStatusWidget::new();
        // Very narrow: names must be truncated.
        let line = w.render(30, &ctx);
        let text = line.to_string();
        // Should still contain separator and not panic.
        assert!(text.contains(" | "), "expected separator in narrow: {text}");
    }

    // -- Integration: render with controlled time --

    #[test]
    fn test_render_attention_session_shows_elapsed() {
        let base = Instant::now();
        let mut session = make_session("proj-b", Status::Attention);
        // Simulate session started 154 seconds ago by manipulating `since`.
        // We set `since` in the past relative to `now` in the context.
        session.since = base;

        let ctx = WidgetContext {
            sessions: std::slice::from_ref(&session),
            now: base + Duration::from_secs(154),
            selected_index: None,
            usage: None,
        };
        let w = SessionStatusWidget::new();
        let line = w.render(80, &ctx);
        let text = line.to_string();
        assert!(text.contains("2m 34s"), "expected elapsed time, got: {text}");
    }

    // -- Default trait --

    #[test]
    fn test_default_trait() {
        let w = SessionStatusWidget::default();
        assert_eq!(w.id(), "session-status");
    }
}
