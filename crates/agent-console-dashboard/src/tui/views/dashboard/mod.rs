//! Dashboard view rendering for session list display.
//!
//! Provides session list rendering with responsive column layouts
//! and status-based color coding.

use crate::{Session, Status, INACTIVE_SESSION_THRESHOLD};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::time::Instant;

/// Returns the status symbol for a given session status.
pub fn status_symbol(status: Status) -> &'static str {
    match status {
        Status::Working => "●",
        Status::Attention => "○",
        Status::Question => "?",
        Status::Closed => "×",
    }
}

/// Returns the display color for a given session status.
pub fn status_color(status: Status) -> Color {
    match status {
        Status::Working => Color::Green,
        Status::Attention => Color::Yellow,
        Status::Question => Color::Blue,
        Status::Closed => Color::Gray,
    }
}

/// Returns the color for an error status (used for sessions with errors).
pub fn error_color() -> Color {
    Color::Red
}

/// Formats a duration in seconds as a human-readable string.
///
/// Returns "Xh Ym Zs" for durations >= 1 hour, "Xm Ys" for >= 1 minute, or "Xs" for < 1 minute.
pub fn format_duration_secs(total_seconds: u64) -> String {
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

/// Formats the elapsed time since the given instant as a human-readable string.
pub fn format_elapsed(since: Instant) -> String {
    format_duration_secs(since.elapsed().as_secs())
}

/// Formats the elapsed time from a raw seconds value (for testing without Instant).
pub fn format_elapsed_seconds(total_seconds: u64) -> String {
    format_duration_secs(total_seconds)
}

/// Responsive layout breakpoint threshold.
const NARROW_THRESHOLD: u16 = 40;

/// Computes display names for session directories with basename disambiguation.
///
/// Returns a map from session_id to display name. If multiple sessions share
/// the same basename, includes parent folders for disambiguation (up to 3 levels).
pub(crate) fn compute_directory_display_names(
    sessions: &[Session],
) -> std::collections::HashMap<String, String> {
    use std::collections::HashMap;
    use std::path::Path;

    // Build O(1) lookup map for sessions
    let session_map: HashMap<&str, &Session> = sessions
        .iter()
        .map(|s| (s.session_id.as_str(), s))
        .collect();

    // Helper: extract components as strings from a path
    fn path_components(path: &Path) -> Vec<String> {
        path.components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect()
    }

    // Initial display names (basename only)
    let mut display_names = HashMap::new();
    for session in sessions {
        let name = match &session.working_dir {
            None => "<error>".to_string(),
            Some(path) => path
                .file_name()
                .and_then(|n| n.to_str())
                .map(String::from)
                .unwrap_or_else(|| "<error>".to_string()),
        };
        display_names.insert(session.session_id.clone(), name);
    }

    // Iteratively add parent levels until no duplicates or max depth reached
    for depth in 1..=3 {
        let mut collision_groups: HashMap<String, Vec<String>> = HashMap::new();
        for (session_id, name) in &display_names {
            collision_groups
                .entry(name.clone())
                .or_default()
                .push(session_id.clone());
        }

        let mut changed = false;
        for (_colliding_name, session_ids) in collision_groups {
            if session_ids.len() <= 1 {
                continue; // No collision
            }

            // Try to disambiguate by adding one more parent level
            for session_id in &session_ids {
                let session = session_map
                    .get(session_id.as_str())
                    .expect("session must exist in map");
                if let Some(path) = &session.working_dir {
                    let components = path_components(path);
                    if components.len() > depth {
                        // Build display name with `depth+1` levels
                        let start = components.len().saturating_sub(depth + 1);
                        let new_name = components[start..].join("/");
                        display_names.insert(session_id.clone(), new_name);
                        changed = true;
                    }
                }
            }
        }

        if !changed {
            break; // No more improvements possible
        }
    }

    // Final pass: if still ambiguous, fall back to full path
    let mut final_collision_groups: HashMap<String, Vec<String>> = HashMap::new();
    for (session_id, name) in &display_names {
        final_collision_groups
            .entry(name.clone())
            .or_default()
            .push(session_id.clone());
    }
    for (_, session_ids) in final_collision_groups {
        if session_ids.len() > 1 {
            for session_id in &session_ids {
                let session = session_map
                    .get(session_id.as_str())
                    .expect("session must exist in map");
                if let Some(path) = &session.working_dir {
                    display_names.insert(session_id.clone(), path.display().to_string());
                }
            }
        }
    }

    display_names
}

/// Formats a single session line based on available terminal width.
///
/// Responsive breakpoints:
/// - `<40` cols: symbol + session ID only
/// - `40-80` cols: symbol + directory (flex) + session ID (40) + status (14) + elapsed (16)
/// - `>80` cols: symbol + directory (flex) + session ID (40) + status (14) + elapsed (16)
///
/// If `is_highlighted` is true and the session is inactive or closed, uses black text for readability
/// against the dark gray highlight background.
pub fn format_session_line<'a>(
    session: &Session,
    width: u16,
    dir_display: &str,
    is_highlighted: bool,
) -> Line<'a> {
    let inactive = session.is_inactive(INACTIVE_SESSION_THRESHOLD);
    let should_dim = inactive || session.status.should_dim();
    let (color, symbol, dim, status_text) = if should_dim {
        // Use black text when highlighted for readability against dark gray background
        let text_color = if is_highlighted {
            Color::Black
        } else {
            Color::DarkGray
        };
        let display_status = if inactive {
            "inactive".to_string()
        } else {
            session.status.to_string()
        };
        (
            Color::DarkGray,
            "◌",
            Style::default().fg(text_color).add_modifier(Modifier::DIM),
            display_status,
        )
    } else {
        (
            status_color(session.status),
            status_symbol(session.status),
            Style::default(),
            session.status.to_string(),
        )
    };
    let elapsed = format_elapsed(session.since);
    let name = session.session_id.clone();

    if width < NARROW_THRESHOLD {
        // Narrow: symbol + session ID only (no column alignment)
        Line::from(vec![
            Span::styled(format!("{} ", symbol), Style::default().fg(color)),
            Span::styled(name, dim),
        ])
    } else {
        // Standard/Wide: directory (flex) + session ID (40) + status (14) + time elapsed (16)
        // Highlight marker (▶ + space, 2 chars) is reserved by HighlightSpacing::Always.
        // Fixed = highlight (2) + session_id (40) + status (14) + time_elapsed (16) = 72
        let fixed_width = 2 + 40 + 14 + 16;
        let dir_width = (width as usize).saturating_sub(fixed_width).max(1);

        let work_dir_text = truncate_string(dir_display, dir_width);
        let is_error = dir_display == "<error>";

        let work_dir_span = if is_error {
            Span::styled(
                format!("{:<dir_width$}", work_dir_text),
                Style::default().fg(error_color()),
            )
        } else {
            Span::styled(format!("{:<dir_width$}", work_dir_text), dim)
        };

        Line::from(vec![
            work_dir_span,
            Span::styled(format!("{:<40}", name), dim),
            Span::styled(format!("{:<14}", status_text), Style::default().fg(color)),
            Span::styled(format!("{:<16}", elapsed), dim),
        ])
    }
}

/// Formats a header line matching the column widths from format_session_line.
///
/// Returns a header row with column titles aligned to their respective columns.
/// Narrow mode has no headers. Standard and wide modes share the same column
/// structure (directory, session ID, status, elapsed) with wider directory in
/// wide mode.
pub fn format_header_line(width: u16) -> Line<'static> {
    let header_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);

    if width < NARROW_THRESHOLD {
        // Narrow: no headers
        Line::from(vec![])
    } else {
        // Standard/Wide: 2 (highlight space) + Directory (flex) + Session ID (40) + Status (14) + Time Elapsed (16)
        let fixed_width = 2 + 40 + 14 + 16;
        let dir_width = (width as usize).saturating_sub(fixed_width).max(1);

        Line::from(vec![
            Span::styled("  ", header_style), // Aligns with highlight marker space
            Span::styled(format!("{:<dir_width$}", "Directory"), header_style),
            Span::styled(format!("{:<40}", "Session ID"), header_style),
            Span::styled(format!("{:<14}", "Status"), header_style),
            Span::styled(format!("{:<16}", "Time Elapsed"), header_style),
        ])
    }
}

/// Formats a debug ruler line showing column boundaries.
///
/// Only displayed when AGENT_CONSOLE_DASHBOARD_DEBUG=1.
pub(crate) fn format_ruler_line(width: u16) -> Line<'static> {
    let style = Style::default().fg(Color::DarkGray);

    if width < NARROW_THRESHOLD {
        return Line::from(vec![]);
    }

    let fixed_width: usize = 2 + 40 + 14 + 16;
    let dir_width = (width as usize).saturating_sub(fixed_width).max(1);

    // Show column widths as labels: "dir:XX | id:40 | stat:14 | time:16"
    let dir_label = format!("{:<dir_width$}", format!("dir:{dir_width}"));
    let id_label = format!("{:<40}", "id:40");
    let status_label = format!("{:<14}", "stat:14");
    let elapsed_label = format!("{:<16}", "time:16");

    Line::from(vec![
        Span::styled("  ", style),
        Span::styled(dir_label, style),
        Span::styled(id_label, style),
        Span::styled(status_label, style),
        Span::styled(elapsed_label, style),
    ])
}

/// Returns true if the debug ruler should be displayed.
pub(crate) fn debug_ruler_enabled() -> bool {
    std::env::var("AGENT_CONSOLE_DASHBOARD_DEBUG")
        .map(|v| v == "1")
        .unwrap_or(false)
}

/// Renders the session list into the given area.
///
/// Returns the inner Rect of the List widget (excluding block borders),
/// used for accurate mouse click detection.
pub fn render_session_list(
    frame: &mut Frame,
    area: Rect,
    sessions: &[Session],
    selected_index: Option<usize>,
    width: u16,
) -> Rect {
    // Split area into header (1 line) + optional ruler (1 line) + list (remaining) if not narrow mode
    let show_ruler = debug_ruler_enabled();

    let (header_area, ruler_area, list_area) = if width >= NARROW_THRESHOLD {
        if show_ruler {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // header
                    Constraint::Length(1), // debug ruler
                    Constraint::Min(1),    // list
                ])
                .split(area);
            (Some(chunks[0]), Some(chunks[1]), chunks[2])
        } else {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // header
                    Constraint::Min(1),    // list
                ])
                .split(area);
            (Some(chunks[0]), None, chunks[1])
        }
    } else {
        (None, None, area)
    };

    // Render header if not narrow mode
    if let Some(header_rect) = header_area {
        let header_line = format_header_line(width);
        let header = Paragraph::new(header_line);
        frame.render_widget(header, header_rect);
    }

    // Render debug ruler if enabled
    if let Some(ruler_rect) = ruler_area {
        let ruler_line = format_ruler_line(width);
        let ruler = Paragraph::new(ruler_line);
        frame.render_widget(ruler, ruler_rect);
    }

    // Compute directory display names with disambiguation
    let dir_display_names = compute_directory_display_names(sessions);

    // Render session list
    let items: Vec<ListItem> = sessions
        .iter()
        .enumerate()
        .map(|(index, session)| {
            let dir_display = dir_display_names
                .get(&session.session_id)
                .map(|s| s.as_str())
                .unwrap_or("<error>");
            let is_highlighted = selected_index == Some(index);
            ListItem::new(format_session_line(
                session,
                width,
                dir_display,
                is_highlighted,
            ))
        })
        .collect();

    let block = Block::default()
        .borders(Borders::TOP | Borders::BOTTOM)
        .title(" Sessions ");

    // Calculate inner area (excluding block borders) for mouse click detection
    let inner_area = block.inner(list_area);

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("▶ ")
        .highlight_spacing(HighlightSpacing::Always);

    let mut state = ListState::default();
    state.select(selected_index);

    frame.render_stateful_widget(list, list_area, &mut state);

    inner_area
}

/// Truncates a string to the given max length, appending "..." if truncated.
pub(crate) fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        s[..max_len].to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

#[cfg(test)]
mod tests;
