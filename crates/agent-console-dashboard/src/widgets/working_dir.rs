//! Working directory widget for the dashboard status bar.
//!
//! Displays the working directory of the currently selected session,
//! collapsing the home directory prefix to `~` and truncating long
//! paths from the left to fit the available width.
//!
//! # Path Formatting
//!
//! The path is formatted through several stages:
//! 1. Home directory prefix is replaced with `~/`
//! 2. If the result fits the available width, it is used as-is
//! 3. If the final component name alone is too wide, it is truncated
//! 4. Otherwise, the path is truncated from the left with a `…/` prefix
//!
//! # Styling
//!
//! - Active sessions: Cyan text
//! - Closed sessions: Dimmed text
//!
//! # Example
//!
//! ```
//! use agent_console_dashboard::widgets::working_dir::WorkingDirWidget;
//! use agent_console_dashboard::widgets::{Widget, WidgetContext};
//! use agent_console_dashboard::Session;
//!
//! let widget = WorkingDirWidget;
//! assert_eq!(widget.id(), "working-dir");
//! assert_eq!(widget.min_width(), 15);
//! ```

use super::{Widget, WidgetContext};
use ratatui::{
    style::{Color, Modifier, Style},
    text::Line,
};
use std::path::Path;

/// Widget that displays the working directory of the selected session.
///
/// Shows `(no sessions)` when the session list is empty, or
/// `(no session selected)` when no session is selected and the list
/// is non-empty (falls back to first session in that case).
pub struct WorkingDirWidget;

impl WorkingDirWidget {
    /// Factory function for the widget registry.
    pub fn create() -> Box<dyn Widget> {
        Box::new(Self)
    }
}

impl Widget for WorkingDirWidget {
    fn render(&self, width: u16, context: &WidgetContext) -> Line<'_> {
        let w = width as usize;

        if context.sessions.is_empty() {
            return Line::styled(
                "(no sessions)",
                Style::default().add_modifier(Modifier::DIM),
            );
        }

        // Use selected session, or default to first session.
        let session = context
            .selected_session()
            .or_else(|| context.sessions.first())
            .expect("sessions is non-empty, first() must return Some");

        let text = match &session.working_dir {
            None => "<none>".to_string(),
            Some(path) => format_path(path, w),
        };

        let style = if session.closed {
            Style::default().add_modifier(Modifier::DIM)
        } else {
            Style::default().fg(Color::Cyan)
        };

        Line::from(ratatui::text::Span::styled(text, style))
    }

    fn id(&self) -> &'static str {
        "working-dir"
    }

    fn min_width(&self) -> u16 {
        15
    }
}

/// Collapse the home directory prefix to `~`.
///
/// If the path starts with the user's home directory, it is replaced
/// with `~`. Otherwise the path is returned as-is.
///
/// # Arguments
///
/// * `path` - The path to collapse.
///
/// # Returns
///
/// A string representation with home collapsed to `~`.
fn collapse_home(path: &Path) -> String {
    if let Some(home) = dirs::home_dir() {
        if let Ok(stripped) = path.strip_prefix(&home) {
            let rest = stripped.to_string_lossy();
            if rest.is_empty() {
                return "~".to_string();
            }
            return format!("~/{rest}");
        }
    }
    path.to_string_lossy().into_owned()
}

/// Format a path to fit within `max_width` columns.
///
/// Applies home directory collapsing, then truncates from the left
/// if the result is too long.
///
/// # Algorithm
///
/// 1. Collapse home directory to `~`
/// 2. If the result fits, return it
/// 3. If the final directory name plus `…/` prefix is already too wide,
///    truncate the final name itself
/// 4. Otherwise, progressively remove leading path components and
///    prepend `…/`
///
/// # Arguments
///
/// * `path` - The filesystem path to format.
/// * `max_width` - Maximum display width in columns.
///
/// # Returns
///
/// A string that fits within `max_width` columns.
fn format_path(path: &Path, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }

    let collapsed = collapse_home(path);

    // If it fits, return as-is.
    if collapsed.len() <= max_width {
        return collapsed;
    }

    // Get the final component name.
    let final_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| collapsed.clone());

    // If even "…/" + final_name doesn't fit, just truncate the final name.
    let prefix = "…/";
    let prefixed = format!("{prefix}{final_name}");

    if prefixed.len() > max_width {
        // Just show as much of final_name as fits.
        if max_width <= prefix.len() {
            // Very narrow: just truncate the final name directly.
            return final_name.chars().take(max_width).collect();
        }
        return format!(
            "{prefix}{}",
            &final_name[..max_width.saturating_sub(prefix.len()).min(final_name.len())]
        );
    }

    // Try progressively shorter suffixes of the path.
    // Split the collapsed path and try from the right.
    let parts: Vec<&str> = collapsed.split('/').collect();

    // Build from the right, adding components until we exceed max_width.
    let mut suffix = String::new();
    for &part in parts.iter().rev() {
        let candidate = if suffix.is_empty() {
            part.to_string()
        } else {
            format!("{part}/{suffix}")
        };

        let with_ellipsis = format!("{prefix}{candidate}");
        if with_ellipsis.len() > max_width {
            break;
        }
        suffix = candidate;
    }

    // Check if the full suffix equals the collapsed path (no truncation needed).
    if suffix == collapsed {
        return collapsed;
    }

    format!("{prefix}{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentType, Session};
    use std::path::PathBuf;

    // -- format_path tests --

    #[test]
    fn test_format_path_home_collapse() {
        if let Some(home) = dirs::home_dir() {
            let path = home.join("projects/myapp");
            let result = format_path(&path, 80);
            assert_eq!(result, "~/projects/myapp");
        }
    }

    #[test]
    fn test_format_path_fits_width() {
        let path = PathBuf::from("/usr/local/bin");
        let result = format_path(&path, 80);
        assert_eq!(result, "/usr/local/bin");
    }

    #[test]
    fn test_format_path_truncate_left() {
        let path = PathBuf::from("/very/long/deeply/nested/directory/structure");
        let result = format_path(&path, 25);
        assert!(
            result.starts_with("…/"),
            "expected …/ prefix, got: {result}"
        );
        assert!(
            result.len() <= 25,
            "expected <= 25 chars, got {} for: {result}",
            result.len()
        );
        assert!(
            result.contains("structure"),
            "expected final dir name in: {result}"
        );
    }

    #[test]
    fn test_format_path_very_narrow_final_name() {
        let path = PathBuf::from("/some/path/to/very_long_directory_name");
        let result = format_path(&path, 5);
        assert!(
            result.len() <= 5,
            "expected <= 5 chars, got {} for: {result}",
            result.len()
        );
    }

    #[test]
    fn test_format_path_root() {
        let path = PathBuf::from("/");
        let result = format_path(&path, 80);
        assert_eq!(result, "/");
    }

    #[test]
    fn test_format_path_zero_width() {
        let path = PathBuf::from("/some/path");
        let result = format_path(&path, 0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_format_path_exact_fit() {
        let path = PathBuf::from("/usr/local");
        let result = format_path(&path, 10);
        assert_eq!(result, "/usr/local");
    }

    #[test]
    fn test_format_path_one_over() {
        let path = PathBuf::from("/usr/local");
        let result = format_path(&path, 9);
        assert!(
            result.starts_with("…/"),
            "expected …/ prefix, got: {result}"
        );
        assert!(result.len() <= 9, "expected <= 9 chars, got: {result}");
    }

    #[test]
    fn test_collapse_home_with_home_path() {
        if let Some(home) = dirs::home_dir() {
            let result = collapse_home(&home);
            assert_eq!(result, "~");
        }
    }

    #[test]
    fn test_collapse_home_non_home_path() {
        let result = collapse_home(Path::new("/etc/config"));
        assert_eq!(result, "/etc/config");
    }

    // -- Widget trait tests --

    #[test]
    fn test_widget_id() {
        let widget = WorkingDirWidget;
        assert_eq!(widget.id(), "working-dir");
    }

    #[test]
    fn test_widget_min_width() {
        let widget = WorkingDirWidget;
        assert_eq!(widget.min_width(), 15);
    }

    #[test]
    fn test_widget_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<WorkingDirWidget>();
    }

    // -- Render tests --

    #[test]
    fn test_render_no_sessions() {
        let sessions: Vec<Session> = vec![];
        let ctx = WidgetContext::new(&sessions);
        let widget = WorkingDirWidget;
        let line = widget.render(40, &ctx);
        assert_eq!(line.to_string(), "(no sessions)");
    }

    #[test]
    fn test_render_no_selected_defaults_to_first() {
        let sessions = vec![Session::new(
            "s1".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/project")),
        )];
        let ctx = WidgetContext::new(&sessions);
        let widget = WorkingDirWidget;
        let line = widget.render(40, &ctx);
        assert_eq!(line.to_string(), "/tmp/project");
    }

    #[test]
    fn test_render_selected_session() {
        let sessions = vec![
            Session::new(
                "s1".to_string(),
                AgentType::ClaudeCode,
                Some(PathBuf::from("/tmp/a")),
            ),
            Session::new(
                "s2".to_string(),
                AgentType::ClaudeCode,
                Some(PathBuf::from("/tmp/b")),
            ),
        ];
        let ctx = WidgetContext::new(&sessions).with_selected(1);
        let widget = WorkingDirWidget;
        let line = widget.render(40, &ctx);
        assert_eq!(line.to_string(), "/tmp/b");
    }

    #[test]
    fn test_render_closed_session_dimmed() {
        let mut session = Session::new(
            "s1".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/closed")),
        );
        session.closed = true;
        let sessions = vec![session];
        let ctx = WidgetContext::new(&sessions).with_selected(0);
        let widget = WorkingDirWidget;
        let line = widget.render(40, &ctx);
        assert_eq!(line.to_string(), "/tmp/closed");
        // Verify dimmed style.
        let span = &line.spans[0];
        assert!(
            span.style.add_modifier.contains(Modifier::DIM),
            "closed session should have DIM modifier"
        );
    }

    #[test]
    fn test_render_active_session_cyan() {
        let sessions = vec![Session::new(
            "s1".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/active")),
        )];
        let ctx = WidgetContext::new(&sessions).with_selected(0);
        let widget = WorkingDirWidget;
        let line = widget.render(40, &ctx);
        let span = &line.spans[0];
        assert_eq!(
            span.style.fg,
            Some(Color::Cyan),
            "active session should have Cyan foreground"
        );
    }

    #[test]
    fn test_factory_create() {
        let widget = WorkingDirWidget::create();
        assert_eq!(widget.id(), "working-dir");
        assert_eq!(widget.min_width(), 15);
    }
}
