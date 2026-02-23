//! Main rendering orchestration for the TUI dashboard.
//!
//! Provides the top-level `render_dashboard` function that composes
//! the header, session list, and footer into a cohesive layout.

use crate::tui::app::{App, LayoutMode, TWO_LINE_LAYOUT_HEIGHT_THRESHOLD};
use crate::tui::views::dashboard::render_session_list;
use crate::tui::views::detail::{render_detail_placeholder, render_inline_detail};
use crate::widgets::{api_usage::ApiUsageWidget, Widget, WidgetContext};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use std::time::Instant;

/// Header text displayed at the top of the dashboard.
const HEADER_TEXT: &str = "Agent Console Dashboard";

/// Footer text showing available keybindings.
const FOOTER_TEXT: &str = "[j/k] Navigate  [Enter] Hook  [s] Copy ID  [r] Resurrect  [q] Quit";

/// Version string shown in the header (right-aligned).
const VERSION_TEXT: &str = concat!("v", env!("CARGO_PKG_VERSION"));

/// Renders the full dashboard layout: header, session list, detail panel, and footer.
///
/// The detail panel is always visible below the session list. It shows information
/// about the currently focused session, or a hint message when no session is focused.
///
/// Layout modes:
/// - Large (height >= 5): Header, session list, detail panel, footer
/// - TwoLine (height < 5): Session chips (line 1), API usage (line 2)
///
/// When `app.layout_mode_override` is `Some(mode)`, that mode is used regardless of
/// terminal height. Otherwise, layout mode is auto-detected from terminal height.
///
/// Updates `app.session_list_inner_area` with the inner Rect of the session list
/// for accurate mouse click detection.
pub fn render_dashboard(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let now = Instant::now();

    // Store terminal width for mouse click detection
    app.terminal_width = area.width;

    // Use override if present, otherwise auto-detect from terminal height
    app.layout_mode = if let Some(override_mode) = app.layout_mode_override {
        override_mode
    } else if area.height < TWO_LINE_LAYOUT_HEIGHT_THRESHOLD {
        LayoutMode::TwoLine
    } else {
        LayoutMode::Large
    };

    match app.layout_mode {
        LayoutMode::Large => render_large_layout(frame, app, area, now),
        LayoutMode::TwoLine => render_two_line_layout(frame, app, area, now),
    }
}

/// Renders the Large layout mode: header, session list, detail panel, footer.
fn render_large_layout(
    frame: &mut Frame,
    app: &mut App,
    area: ratatui::prelude::Rect,
    now: Instant,
) {
    // Detail panel is always visible
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // header
            Constraint::Min(3),     // session list (minimum 3 rows)
            Constraint::Length(12), // detail panel (always visible)
            Constraint::Length(1),  // footer
        ])
        .split(area);

    // Header with title (left) and version (right-aligned)
    let header_width = chunks[0].width as usize;
    let title_len = HEADER_TEXT.len();
    let version_len = VERSION_TEXT.len();

    // Calculate padding to position version at the right
    // Format: "[title]...[version]"
    let available_space = header_width.saturating_sub(title_len);
    let padding_len = available_space.saturating_sub(version_len);

    let header = Paragraph::new(Line::from(vec![
        Span::styled(HEADER_TEXT, Style::default().fg(Color::Cyan)),
        Span::raw(" ".repeat(padding_len)),
        Span::styled(VERSION_TEXT, Style::default().fg(Color::DarkGray)),
    ]));
    frame.render_widget(header, chunks[0]);

    // Session list - capture inner area for mouse click detection
    let inner_area = render_session_list(
        frame,
        chunks[1],
        &app.sessions,
        app.selected_index,
        area.width,
    );
    app.session_list_inner_area = Some(inner_area);

    // Detail panel (always visible — shows focused session or placeholder)
    if let Some(selected_idx) = app.selected_index {
        if let Some(session) = app.sessions.get(selected_idx) {
            render_inline_detail(frame, session, chunks[2], app.history_scroll, now);
        } else {
            render_detail_placeholder(frame, chunks[2]);
        }
    } else {
        render_detail_placeholder(frame, chunks[2]);
    }

    // Footer (with optional status message overlay)
    // When status message is active, it overrides the entire footer
    let footer_text = if let Some((ref msg, expiry)) = app.status_message {
        if Instant::now() < expiry {
            Line::from(vec![Span::styled(
                msg.clone(),
                Style::default().fg(Color::Yellow),
            )])
        } else {
            render_footer_normal(&app.sessions, app.usage.as_ref(), chunks[3].width as usize)
        }
    } else {
        render_footer_normal(&app.sessions, app.usage.as_ref(), chunks[3].width as usize)
    };
    let footer = Paragraph::new(footer_text);
    frame.render_widget(footer, chunks[3]);
}

/// Renders the TwoLine layout mode: session chips (line 1), API usage (line 2).
fn render_two_line_layout(
    frame: &mut Frame,
    app: &mut App,
    area: ratatui::prelude::Rect,
    now: Instant,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // session chips
            Constraint::Length(1), // API usage
        ])
        .split(area);

    // Line 1: Session chips with horizontal pagination
    let session_line = render_compact_session_chips(
        &app.sessions,
        app.selected_index,
        app.compact_scroll_offset,
        chunks[0].width,
        now,
    );

    // Auto-scroll to keep selected chip visible
    let max_visible = calculate_max_visible_chips(chunks[0].width);
    app.ensure_selected_visible_compact(max_visible);

    let session_paragraph = Paragraph::new(session_line);
    frame.render_widget(session_paragraph, chunks[0]);

    // Line 2: Status message (if active) or API usage
    if let Some((ref msg, expiry)) = app.status_message {
        if now < expiry {
            // Show status message (yellow)
            let status_line = Line::from(vec![Span::styled(
                msg.clone(),
                Style::default().fg(Color::Yellow),
            )]);
            let status_paragraph = Paragraph::new(status_line);
            frame.render_widget(status_paragraph, chunks[1]);
        } else {
            // Status message expired, render API usage
            let mut ctx = WidgetContext::new(&app.sessions);
            ctx.now = now;
            if let Some(ref usage) = app.usage {
                ctx = ctx.with_usage(usage);
            }
            let api_widget = ApiUsageWidget::new();
            let api_line = api_widget.render(chunks[1].width, &ctx);
            let api_paragraph = Paragraph::new(api_line);
            frame.render_widget(api_paragraph, chunks[1]);
        }
    } else {
        // No status message, render API usage
        let mut ctx = WidgetContext::new(&app.sessions);
        ctx.now = now;
        if let Some(ref usage) = app.usage {
            ctx = ctx.with_usage(usage);
        }
        let api_widget = ApiUsageWidget::new();
        let api_line = api_widget.render(chunks[1].width, &ctx);
        let api_paragraph = Paragraph::new(api_line);
        frame.render_widget(api_paragraph, chunks[1]);
    }

    // Clear session_list_inner_area since there's no clickable list in TwoLine mode
    app.session_list_inner_area = None;
}

/// Renders the normal footer layout: keybinding hints left, API usage right.
///
/// The footer is split into two parts:
/// - LEFT: keybinding hints (DarkGray)
/// - RIGHT: API usage widget in SHORT format (width < 30 to force SHORT)
///
/// If the terminal is too narrow to fit both, only hints are shown.
fn render_footer_normal(
    sessions: &[crate::Session],
    usage: Option<&claude_usage::UsageData>,
    footer_width: usize,
) -> Line<'static> {
    let hints_text = FOOTER_TEXT;
    let hints_len = hints_text.len();

    // Create widget context (usage may be None, which shows "Quota: --")
    let mut ctx = WidgetContext::new(sessions);
    if let Some(u) = usage {
        ctx = ctx.with_usage(u);
    }
    let api_widget = ApiUsageWidget::new();

    // Render with width < 30 to force SHORT format
    // The SHORT format needs minimum 15 chars (widget.min_width())
    let api_usage_line = api_widget.render(25, &ctx);
    let api_usage_text = api_usage_line.to_string();
    let api_usage_len = api_usage_text.len();

    // Check if we have enough space for both hints and API usage
    // Need: hints_len + 2 (spacing) + api_usage_len
    let min_width = hints_len + 2 + api_usage_len;

    if footer_width < min_width {
        // Not enough space — only show hints
        return Line::from(vec![Span::styled(
            hints_text,
            Style::default().fg(Color::DarkGray),
        )]);
    }

    // Calculate padding to position API usage on the right
    let padding_len = footer_width
        .saturating_sub(hints_len)
        .saturating_sub(api_usage_len);

    // Build footer: hints (left) + padding + API usage (right)
    // Convert api_usage_line spans to owned Spans with cloned content
    let mut spans = vec![Span::styled(
        hints_text,
        Style::default().fg(Color::DarkGray),
    )];
    spans.push(Span::raw(" ".repeat(padding_len)));

    // Clone api_usage_line spans to owned Spans
    for span in api_usage_line.spans {
        spans.push(Span::styled(span.content.to_string(), span.style));
    }

    Line::from(spans)
}

/// Width reserved for each overflow indicator.
/// Format: `<- N+|` (left) or `|N+ ->` (right), where N can be 0-999.
/// With 0: `<- 0 |` (6 chars), with count: `<- N+|` (6 chars for N<10, 7 for N<100, 8 for N<1000).
/// We reserve 7 chars to handle counts up to 99 safely.
pub const OVERFLOW_INDICATOR_WIDTH: usize = 7;

/// Maximum width of a single session chip (symbol + folder name + spacing).
/// Actual chip widths are dynamic (sized to content), but capped at this max.
pub const MAX_CHIP_WIDTH: usize = 18;

/// Calculates the maximum count of session chips that fit in the available width.
///
/// With dynamic chip widths, this uses MAX_CHIP_WIDTH as an estimate for initial
/// viewport sizing. Actual visible count may vary based on content length.
fn calculate_max_visible_chips(available_width: u16) -> usize {
    let width = available_width as usize;
    // Reserve space for both overflow indicators
    let content_width = width.saturating_sub(OVERFLOW_INDICATOR_WIDTH * 2);
    // Divide by max chip width, minimum 1
    (content_width / MAX_CHIP_WIDTH).max(1)
}

/// Public wrapper for calculate_max_visible_chips (used by event handler).
pub fn calculate_max_visible_chips_public(available_width: u16) -> usize {
    calculate_max_visible_chips(available_width)
}

/// Helper: Truncates a folder name from the start, keeping the end.
/// E.g., "my-long-folder-name" → "...folder-name" (max 12 chars visible).
fn truncate_from_start(name: &str, max_len: usize) -> String {
    if name.len() <= max_len {
        name.to_string()
    } else if max_len <= 3 {
        name[name.len().saturating_sub(max_len)..].to_string()
    } else {
        format!("...{}", &name[name.len().saturating_sub(max_len - 3)..])
    }
}

/// Calculates the display width of a chip (accounts for symbol, name, brackets, separators).
///
/// # Chip anatomy
///
/// - Unselected: ` ` + symbol + ` ` + name (e.g., ` * my-proj`)
/// - Focused: `[` + symbol + ` ` + name + `]` (e.g., `[? src]`)
/// - Separator: ` | ` between chips, but NO extra space adjacent to brackets:
///   - unfocused | unfocused: ` | `
///   - unfocused |[focused: ` |[`
///   - focused]| unfocused: `]| `
///
/// The separator is NOT part of the chip width — it's rendered between chips.
fn chip_width(name: &str, is_focused: bool) -> usize {
    let name_len = name.len();
    if is_focused {
        // '[' + symbol + ' ' + name + ']' = 4 + name_len
        4 + name_len
    } else {
        // ' ' + symbol + ' ' + name = 3 + name_len
        3 + name_len
    }
}

/// Renders session chips with horizontal pagination and overflow indicators.
///
/// Shows a viewport window of visible sessions with overflow indicators in the new format:
/// - With overflow: `<- 3+| * agent-console | ! my-project |[? src]| . old-proj |5+ ->`
/// - No overflow: `<- 0 | * agent-console | ! my-project |[? src]| . old-proj | 0 ->`
///
/// Overflow indicators are ALWAYS shown, never hidden or grayed out. The count changes
/// format: N+ (no space) when N > 0, or 0 (with space) when N = 0.
///
/// # Arguments
///
/// * `sessions` - Full list of sessions to render
/// * `selected_index` - Index of selected session (if any)
/// * `scroll_offset` - Index of leftmost visible session
/// * `available_width` - Terminal width for this line
/// * `_now` - Current time for elapsed time calculations (unused for now)
fn render_compact_session_chips(
    sessions: &[crate::Session],
    selected_index: Option<usize>,
    scroll_offset: usize,
    available_width: u16,
    _now: Instant,
) -> Line<'static> {
    use crate::tui::views::dashboard::{get_directory_display_name, status_color, status_symbol};

    if sessions.is_empty() {
        return Line::raw("(no sessions)");
    }

    let width = available_width as usize;

    // Determine visible range by accumulating chip widths
    let start = scroll_offset.min(sessions.len().saturating_sub(1));

    // Calculate visible chips that fit in available width
    // Reserve: left_indicator (7) + right_indicator (7) = 14 chars
    let content_width = width.saturating_sub(OVERFLOW_INDICATOR_WIDTH * 2);

    let mut accumulated_width = 0;
    let mut end = start;

    for (offset, session) in sessions[start..].iter().enumerate() {
        let i = start + offset;
        let is_focused = selected_index == Some(i);

        // Get display name
        let display_name = get_directory_display_name(session);
        let label = if display_name == "<error>" {
            session.session_id.chars().take(8).collect()
        } else {
            truncate_from_start(&display_name, 12)
        };

        let this_chip_width = chip_width(&label, is_focused);

        // Add separator width (3 chars: " | ") except for first chip
        let separator_width = if offset > 0 { 3 } else { 0 };

        let total_needed = accumulated_width + separator_width + this_chip_width;

        if total_needed > content_width && offset > 0 {
            // Would overflow, stop here
            break;
        }

        accumulated_width = total_needed;
        end = i + 1;
    }

    let visible_sessions = &sessions[start..end];

    // Calculate overflow counts
    let overflow_left = start;
    let overflow_right = sessions.len().saturating_sub(end);

    let mut spans = vec![];

    // Left overflow indicator (always shown)
    if overflow_left > 0 {
        spans.push(Span::styled(
            format!("<- {}+|", overflow_left),
            Style::default().fg(Color::DarkGray),
        ));
    } else {
        spans.push(Span::styled(
            "<- 0 |".to_string(),
            Style::default().fg(Color::DarkGray),
        ));
    }

    // Visible chips
    for (index, session) in visible_sessions.iter().enumerate() {
        let global_index = start + index;
        let is_selected = selected_index == Some(global_index);

        let inactive = session.is_inactive(crate::INACTIVE_SESSION_THRESHOLD);
        let should_dim = inactive || session.status.should_dim();

        // Use dot symbol for inactive sessions, otherwise use status-specific symbol
        let (symbol, color) = if should_dim {
            if inactive {
                (".", Color::DarkGray)
            } else {
                (status_symbol(session.status), Color::DarkGray)
            }
        } else {
            (status_symbol(session.status), status_color(session.status))
        };

        // Display name: folder basename, or fallback to short session_id (first 8 chars)
        let display_name = get_directory_display_name(session);
        let label = if display_name == "<error>" {
            session.session_id.chars().take(8).collect()
        } else {
            truncate_from_start(&display_name, 12)
        };

        // Separator before this chip (except for first chip)
        if index > 0 {
            // Previous chip was focused: its ']' was already pushed in that
            // chip's own rendering, so the separator is just "|".
            // Otherwise: " |"
            let prev_was_focused = selected_index == Some(global_index - 1);
            if prev_was_focused {
                spans.push(Span::styled(
                    "|".to_string(),
                    Style::default().fg(Color::DarkGray),
                ));
            } else {
                spans.push(Span::styled(
                    " |".to_string(),
                    Style::default().fg(Color::DarkGray),
                ));
            }
        }

        // Current chip opening (space or bracket)
        if is_selected {
            spans.push(Span::styled(
                "[".to_string(),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::styled(" ".to_string(), Style::default().fg(color)));
        }

        // Chip content: symbol + space + label
        let chip_content = format!("{} {}", symbol, label);
        let style = if is_selected {
            Style::default().fg(color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(color)
        };
        spans.push(Span::styled(chip_content, style));

        // Close focused chip with ']' using the same style as the chip content.
        // This ensures the bracket color matches the chip text regardless of
        // whether this is the last visible chip or not.
        if is_selected {
            spans.push(Span::styled(
                "]".to_string(),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ));
        }
    }

    // Right overflow indicator (always shown)
    if overflow_right > 0 {
        // If last chip was selected, no space before pipe (already have ']')
        let last_selected = selected_index == Some(end - 1);
        if last_selected {
            spans.push(Span::styled(
                format!("|{}+ ->", overflow_right),
                Style::default().fg(Color::DarkGray),
            ));
        } else {
            spans.push(Span::styled(
                format!(" |{}+ ->", overflow_right),
                Style::default().fg(Color::DarkGray),
            ));
        }
    } else {
        let last_selected = selected_index == Some(end - 1);
        if last_selected {
            spans.push(Span::styled(
                "| 0 ->".to_string(),
                Style::default().fg(Color::DarkGray),
            ));
        } else {
            spans.push(Span::styled(
                " | 0 ->".to_string(),
                Style::default().fg(Color::DarkGray),
            ));
        }
    }

    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentType, Session, Status};
    use std::path::PathBuf;

    fn make_app() -> App {
        App::new(PathBuf::from("/tmp/test.sock"), None)
    }

    fn make_app_with_sessions(count: usize) -> App {
        let mut app = make_app();
        for i in 0..count {
            let mut session = Session::new(
                format!("session-{}", i),
                AgentType::ClaudeCode,
                Some(PathBuf::from(format!("/home/user/project-{}", i))),
            );
            if i % 4 == 1 {
                session.status = Status::Attention;
            } else if i % 4 == 2 {
                session.status = Status::Question;
            } else if i % 4 == 3 {
                session.status = Status::Closed;
            }
            app.sessions.push(session);
        }
        app
    }

    #[test]
    fn test_render_dashboard_empty_no_panic() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let mut app = make_app();
        terminal
            .draw(|frame| render_dashboard(frame, &mut app))
            .expect("draw should not fail");
    }

    #[test]
    fn test_render_dashboard_with_sessions_no_panic() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let mut app = make_app_with_sessions(5);
        app.selected_index = Some(2);
        terminal
            .draw(|frame| render_dashboard(frame, &mut app))
            .expect("draw should not fail");
    }

    #[test]
    fn test_render_dashboard_narrow_no_panic() {
        let backend = ratatui::backend::TestBackend::new(30, 10);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let mut app = make_app_with_sessions(3);
        terminal
            .draw(|frame| render_dashboard(frame, &mut app))
            .expect("draw should not fail");
    }

    #[test]
    fn test_render_dashboard_wide_no_panic() {
        let backend = ratatui::backend::TestBackend::new(200, 50);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let mut app = make_app_with_sessions(10);
        terminal
            .draw(|frame| render_dashboard(frame, &mut app))
            .expect("draw should not fail");
    }

    #[test]
    fn test_render_dashboard_minimal_height_no_panic() {
        let backend = ratatui::backend::TestBackend::new(80, 3);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let mut app = make_app_with_sessions(5);
        terminal
            .draw(|frame| render_dashboard(frame, &mut app))
            .expect("draw should not fail");
    }

    #[test]
    fn test_render_dashboard_single_row_no_panic() {
        let backend = ratatui::backend::TestBackend::new(80, 1);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let mut app = make_app();
        terminal
            .draw(|frame| render_dashboard(frame, &mut app))
            .expect("draw should not fail");
    }

    #[test]
    fn test_render_dashboard_many_sessions_no_panic() {
        let backend = ratatui::backend::TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let mut app = make_app_with_sessions(100);
        app.selected_index = Some(50);
        terminal
            .draw(|frame| render_dashboard(frame, &mut app))
            .expect("draw should not fail");
    }

    #[test]
    fn test_header_text_content() {
        assert_eq!(HEADER_TEXT, "Agent Console Dashboard");
    }

    #[test]
    fn test_version_text_content() {
        // Both sides use compile-time values — no hardcoded version to maintain
        assert_eq!(VERSION_TEXT, concat!("v", env!("CARGO_PKG_VERSION")));
    }

    #[test]
    fn test_version_shown_in_header_row() {
        let mut app = make_app_with_sessions(3);
        let buffer = render_dashboard_to_buffer(&mut app, 80, 24);
        // Header is row 0
        assert!(
            row_contains(&buffer, 0, VERSION_TEXT),
            "Header row should contain version string"
        );
    }

    #[test]
    fn test_version_not_in_footer_row() {
        let mut app = make_app_with_sessions(3);
        let buffer = render_dashboard_to_buffer(&mut app, 80, 24);
        let footer_row = buffer.area().height - 1;
        assert!(
            !row_contains(&buffer, footer_row, VERSION_TEXT),
            "Footer row should NOT contain version string (moved to header)"
        );
    }

    #[test]
    fn test_footer_text_content() {
        assert!(FOOTER_TEXT.contains("[j/k]"));
        assert!(FOOTER_TEXT.contains("[q] Quit"));
        assert!(FOOTER_TEXT.contains("[r] Resurrect"));
        assert!(FOOTER_TEXT.contains("[Enter] Hook"));
        assert!(FOOTER_TEXT.contains("[s] Copy ID"));
    }

    // --- Detail view (inline panel) tests ---

    #[test]
    fn test_render_dashboard_with_detail_view_no_panic() {
        let backend = ratatui::backend::TestBackend::new(80, 30);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let mut app = make_app_with_sessions(5);
        app.selected_index = Some(1);
        app.open_detail(1);
        terminal
            .draw(|frame| render_dashboard(frame, &mut app))
            .expect("draw should not fail with detail view active");
    }

    #[test]
    fn test_render_dashboard_detail_view_narrow_no_panic() {
        let backend = ratatui::backend::TestBackend::new(30, 20);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let mut app = make_app_with_sessions(3);
        app.open_detail(0);
        terminal
            .draw(|frame| render_dashboard(frame, &mut app))
            .expect("draw should not fail with detail view on narrow terminal");
    }

    #[test]
    fn test_render_dashboard_detail_view_minimal_height_no_panic() {
        let backend = ratatui::backend::TestBackend::new(80, 5);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let mut app = make_app_with_sessions(3);
        app.open_detail(0);
        terminal
            .draw(|frame| render_dashboard(frame, &mut app))
            .expect("draw should not fail with detail on minimal height");
    }

    #[test]
    fn test_render_dashboard_detail_view_out_of_bounds_session_no_panic() {
        let backend = ratatui::backend::TestBackend::new(80, 30);
        let mut terminal = ratatui::Terminal::new(backend).expect("failed to create test terminal");
        let mut app = make_app_with_sessions(2);
        // Set out-of-bounds selection to test placeholder path
        app.selected_index = Some(99);
        terminal
            .draw(|frame| render_dashboard(frame, &mut app))
            .expect("draw should not fail with out-of-bounds detail index");
    }

    // --- Full Dashboard Integration Tests (acd-211) ---

    use crate::tui::test_utils::{
        find_row_with_text, render_dashboard_to_buffer, row_contains, row_text,
    };

    #[test]
    fn test_full_dashboard_render_with_mixed_statuses() {
        let mut app = App::new(PathBuf::from("/tmp/test.sock"), None);

        // Add 4 sessions with different statuses
        let mut s1 = Session::new(
            "working-session".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/work")),
        );
        s1.status = Status::Working;

        let mut s2 = Session::new(
            "attention-session".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/attention")),
        );
        s2.status = Status::Attention;

        let mut s3 = Session::new(
            "question-session".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/question")),
        );
        s3.status = Status::Question;

        let mut s4 = Session::new(
            "closed-session".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/closed")),
        );
        s4.status = Status::Closed;

        app.sessions.extend([s1, s2, s3, s4]);
        app.init_selection();

        let buffer = render_dashboard_to_buffer(&mut app, 80, 30);

        // Verify all sessions appear in the buffer
        assert!(
            find_row_with_text(&buffer, "working-session").is_some(),
            "Dashboard should show working session"
        );
        assert!(
            find_row_with_text(&buffer, "attention-session").is_some(),
            "Dashboard should show attention session"
        );
        assert!(
            find_row_with_text(&buffer, "question-session").is_some(),
            "Dashboard should show question session"
        );
        assert!(
            find_row_with_text(&buffer, "closed-session").is_some(),
            "Dashboard should show closed session"
        );

        // Verify structural elements
        assert!(
            find_row_with_text(&buffer, "Agent Console Dashboard").is_some(),
            "Should show header"
        );
        assert!(
            row_contains(&buffer, buffer.area().height - 1, "[q] Quit"),
            "Should show footer"
        );
    }

    #[test]
    fn test_full_dashboard_render_with_detail_panel() {
        let mut app = make_app_with_sessions(3);
        app.init_selection();
        app.open_detail(1);

        let buffer = render_dashboard_to_buffer(&mut app, 80, 35);

        // Verify session list is visible
        assert!(
            find_row_with_text(&buffer, "session-1").is_some(),
            "Session list should be visible"
        );

        // Verify detail panel is visible
        assert!(
            find_row_with_text(&buffer, "Status:").is_some(),
            "Detail panel should be visible with Status label"
        );

        assert!(
            find_row_with_text(&buffer, "Dir:").is_some(),
            "Detail panel should be visible with Dir label"
        );

        assert!(
            find_row_with_text(&buffer, "ID:").is_some(),
            "Detail panel should be visible with ID label"
        );

        // Verify both header and footer are present
        assert!(
            find_row_with_text(&buffer, "Agent Console Dashboard").is_some(),
            "Header should be visible"
        );
        assert!(
            row_contains(&buffer, buffer.area().height - 1, "[q] Quit"),
            "Footer should be visible"
        );
    }

    #[test]
    fn test_full_dashboard_render_many_sessions_scrolling() {
        let mut app = App::new(PathBuf::from("/tmp/test.sock"), None);

        // Add 50 sessions
        for i in 0..50 {
            let mut session = Session::new(
                format!("session-{:02}", i),
                AgentType::ClaudeCode,
                Some(PathBuf::from(format!("/tmp/project-{}", i))),
            );
            if i % 4 == 1 {
                session.status = Status::Attention;
            } else if i % 4 == 2 {
                session.status = Status::Question;
            } else if i % 4 == 3 {
                session.status = Status::Closed;
            }
            app.sessions.push(session);
        }

        // Select session #25
        app.selected_index = Some(25);

        let buffer = render_dashboard_to_buffer(&mut app, 100, 40);

        // The selected session should appear in the buffer
        // (ratatui's List widget handles scrolling automatically to show selection)
        assert!(
            find_row_with_text(&buffer, "session-25").is_some(),
            "Selected session should be visible in scrolled view"
        );

        // Verify structural integrity with many sessions
        assert!(
            find_row_with_text(&buffer, "Agent Console Dashboard").is_some(),
            "Header should be visible with many sessions"
        );
        assert!(
            find_row_with_text(&buffer, "Sessions").is_some(),
            "Session border should be visible"
        );
        assert!(
            row_contains(&buffer, buffer.area().height - 1, "[q] Quit"),
            "Footer should be visible with many sessions"
        );
    }

    // --- Footer layout tests (acd-0i4i) ---

    #[test]
    fn test_footer_shows_api_usage_short_format() {
        use claude_usage::{UsageData, UsagePeriod};
        let mut app = make_app_with_sessions(3);
        app.usage = Some(UsageData {
            five_hour: UsagePeriod {
                utilization: 8.0,
                resets_at: None,
            },
            seven_day: UsagePeriod {
                utilization: 77.0,
                resets_at: None,
            },
            seven_day_sonnet: None,
            extra_usage: None,
        });

        // Width must be large enough for hints (67) + spacing (2) + usage (~15) = 84+
        let buffer = render_dashboard_to_buffer(&mut app, 100, 24);
        let footer_row = buffer.area().height - 1;

        // Verify SHORT format is present: [5h:8% 7d:77%]
        assert!(
            row_contains(&buffer, footer_row, "[5h:8%"),
            "Footer should contain SHORT format start"
        );
        assert!(
            row_contains(&buffer, footer_row, "7d:77%]"),
            "Footer should contain SHORT format end"
        );
    }

    #[test]
    fn test_footer_layout_hints_left_usage_right() {
        use claude_usage::{UsageData, UsagePeriod};
        let mut app = make_app_with_sessions(3);
        app.usage = Some(UsageData {
            five_hour: UsagePeriod {
                utilization: 8.0,
                resets_at: None,
            },
            seven_day: UsagePeriod {
                utilization: 77.0,
                resets_at: None,
            },
            seven_day_sonnet: None,
            extra_usage: None,
        });

        let buffer = render_dashboard_to_buffer(&mut app, 100, 24);
        let footer_row = buffer.area().height - 1;
        let footer_text = row_text(&buffer, footer_row);

        // Find positions of hints and usage
        let hints_pos = footer_text
            .find("[j/k]")
            .expect("hints should be in footer");
        let usage_pos = footer_text
            .find("[5h:8%")
            .expect("usage should be in footer");

        // Usage should be to the right of hints
        assert!(
            usage_pos > hints_pos,
            "API usage should be positioned right of hints"
        );
    }

    #[test]
    fn test_footer_no_usage_shows_placeholder() {
        let mut app = make_app_with_sessions(3);
        app.usage = None; // No usage data

        let buffer = render_dashboard_to_buffer(&mut app, 80, 24);
        let footer_row = buffer.area().height - 1;

        // When usage is None, widget should show "Quota: --"
        assert!(
            row_contains(&buffer, footer_row, "Quota: --"),
            "Footer should show quota placeholder when usage unavailable"
        );
    }

    #[test]
    fn test_footer_narrow_terminal_shows_only_hints() {
        use claude_usage::{UsageData, UsagePeriod};
        let mut app = make_app_with_sessions(3);
        app.usage = Some(UsageData {
            five_hour: UsagePeriod {
                utilization: 8.0,
                resets_at: None,
            },
            seven_day: UsagePeriod {
                utilization: 77.0,
                resets_at: None,
            },
            seven_day_sonnet: None,
            extra_usage: None,
        });

        // Very narrow terminal — not enough space for both hints and usage
        let buffer = render_dashboard_to_buffer(&mut app, 50, 24);
        let footer_row = buffer.area().height - 1;
        let footer_text = row_text(&buffer, footer_row);

        // Hints should still be there
        assert!(
            footer_text.contains("[j/k]"),
            "Footer should show hints on narrow terminal"
        );

        // Usage should NOT be there (not enough space)
        assert!(
            !footer_text.contains("[5h:8%"),
            "Footer should not show usage on narrow terminal"
        );
    }

    #[test]
    fn test_footer_status_message_overrides_entire_footer() {
        use claude_usage::{UsageData, UsagePeriod};
        let mut app = make_app_with_sessions(3);
        app.usage = Some(UsageData {
            five_hour: UsagePeriod {
                utilization: 8.0,
                resets_at: None,
            },
            seven_day: UsagePeriod {
                utilization: 77.0,
                resets_at: None,
            },
            seven_day_sonnet: None,
            extra_usage: None,
        });
        app.status_message = Some((
            "Test message".to_string(),
            Instant::now() + std::time::Duration::from_secs(10),
        ));

        let buffer = render_dashboard_to_buffer(&mut app, 80, 24);
        let footer_row = buffer.area().height - 1;
        let footer_text = row_text(&buffer, footer_row);

        // Status message should override everything
        assert!(
            footer_text.contains("Test message"),
            "Footer should show status message"
        );
        assert!(
            !footer_text.contains("[j/k]"),
            "Footer should not show hints when status message active"
        );
        assert!(
            !footer_text.contains("[5h:8%"),
            "Footer should not show usage when status message active"
        );
    }

    // --- Layout mode tests ---

    #[test]
    fn test_layout_mode_auto_detects_large_for_height_7() {
        let mut app = make_app_with_sessions(3);
        let buffer = render_dashboard_to_buffer(&mut app, 80, 7);
        // Height 7 (= threshold) should use Large mode (threshold is < 7)
        assert_eq!(app.layout_mode, crate::tui::app::LayoutMode::Large);
        // Should have header
        assert!(
            find_row_with_text(&buffer, "Agent Console Dashboard").is_some(),
            "Large mode should show header"
        );
    }

    #[test]
    fn test_layout_mode_auto_detects_two_line_for_height_6() {
        let mut app = make_app_with_sessions(3);
        let buffer = render_dashboard_to_buffer(&mut app, 80, 6);
        // Height 6 (< 7 threshold) should use TwoLine mode
        assert_eq!(app.layout_mode, crate::tui::app::LayoutMode::TwoLine);
        // Should NOT have header
        assert!(
            find_row_with_text(&buffer, "Agent Console Dashboard").is_none(),
            "TwoLine mode should not show header"
        );
    }

    #[test]
    fn test_two_line_layout_shows_session_chips() {
        let mut app = make_app_with_sessions(3);
        let buffer = render_dashboard_to_buffer(&mut app, 80, 2);
        // Line 0 should have session chips with folder names
        let line0 = row_text(&buffer, 0);
        assert!(
            line0.contains("project-"),
            "Line 0 should contain session chips with folder names: {}",
            line0
        );
    }

    #[test]
    fn test_two_line_layout_shows_api_usage() {
        use claude_usage::{UsageData, UsagePeriod};
        let mut app = make_app_with_sessions(3);
        app.usage = Some(UsageData {
            five_hour: UsagePeriod {
                utilization: 42.0,
                resets_at: None,
            },
            seven_day: UsagePeriod {
                utilization: 77.0,
                resets_at: None,
            },
            seven_day_sonnet: None,
            extra_usage: None,
        });
        let buffer = render_dashboard_to_buffer(&mut app, 80, 2);
        // Line 1 should have API usage
        let line1 = row_text(&buffer, 1);
        assert!(
            line1.contains("42%") || line1.contains("77%"),
            "Line 1 should contain API usage: {}",
            line1
        );
    }

    #[test]
    fn test_two_line_layout_no_panic_with_no_sessions() {
        let mut app = make_app();
        let buffer = render_dashboard_to_buffer(&mut app, 80, 2);
        assert_eq!(app.layout_mode, crate::tui::app::LayoutMode::TwoLine);
        // Should render without panic
        let line0 = row_text(&buffer, 0);
        assert!(
            line0.contains("no sessions") || line0.is_empty() || line0.trim().is_empty(),
            "Should handle empty sessions gracefully"
        );
    }

    #[test]
    fn test_layout_mode_threshold_is_7() {
        use crate::tui::app::TWO_LINE_LAYOUT_HEIGHT_THRESHOLD;
        assert_eq!(TWO_LINE_LAYOUT_HEIGHT_THRESHOLD, 7);
    }

    #[test]
    fn test_layout_mode_override_forces_two_line() {
        // Create app with TwoLine override
        let mut app = App::new(
            PathBuf::from("/tmp/test.sock"),
            Some(crate::tui::app::LayoutMode::TwoLine),
        );
        app.sessions.push(Session::new(
            "test-session".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp")),
        ));

        // Render with height 24 (normally would be Large mode)
        let buffer = render_dashboard_to_buffer(&mut app, 80, 24);

        // Should use TwoLine mode despite tall terminal
        assert_eq!(app.layout_mode, crate::tui::app::LayoutMode::TwoLine);

        // TwoLine mode should NOT have the "Agent Console Dashboard" header
        assert!(
            find_row_with_text(&buffer, "Agent Console Dashboard").is_none(),
            "TwoLine mode should not show header even with override"
        );
    }

    #[test]
    fn test_layout_mode_override_forces_large() {
        // Create app with Large override
        let mut app = App::new(
            PathBuf::from("/tmp/test.sock"),
            Some(crate::tui::app::LayoutMode::Large),
        );
        app.sessions.push(Session::new(
            "test-session".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp")),
        ));

        // Render with height 2 (normally would be TwoLine mode)
        let _buffer = render_dashboard_to_buffer(&mut app, 80, 2);

        // Should use Large mode despite short terminal
        assert_eq!(app.layout_mode, crate::tui::app::LayoutMode::Large);
    }

    #[test]
    fn test_layout_mode_no_override_auto_detects() {
        // Create app with None override (auto-detect)
        let mut app = App::new(PathBuf::from("/tmp/test.sock"), None);
        app.sessions.push(Session::new(
            "test-session".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp")),
        ));

        // Height 24 should auto-detect to Large
        let _buffer = render_dashboard_to_buffer(&mut app, 80, 24);
        assert_eq!(app.layout_mode, crate::tui::app::LayoutMode::Large);

        // Height 4 should auto-detect to TwoLine
        let _buffer = render_dashboard_to_buffer(&mut app, 80, 4);
        assert_eq!(app.layout_mode, crate::tui::app::LayoutMode::TwoLine);
    }

    // --- Compact layout pagination tests (acd-6wg6) ---

    #[test]
    fn test_calculate_max_visible_chips_wide_terminal() {
        // Wide terminal (80 chars): should fit multiple chips
        // 80 - (7*2 for indicators) = 66 / 18 = 3 chips
        assert_eq!(calculate_max_visible_chips(80), 3);
    }

    #[test]
    fn test_calculate_max_visible_chips_narrow_terminal() {
        // Narrow terminal (30 chars): should fit at least 1 chip
        // 30 - 14 = 16 / 18 = 0, but minimum is 1
        assert_eq!(calculate_max_visible_chips(30), 1);
    }

    #[test]
    fn test_calculate_max_visible_chips_minimum() {
        // Very narrow: should always return at least 1
        assert_eq!(calculate_max_visible_chips(10), 1);
        assert_eq!(calculate_max_visible_chips(1), 1);
    }

    #[test]
    fn test_render_compact_chips_empty_sessions() {
        use std::time::Instant;
        let sessions = vec![];
        let line = render_compact_session_chips(&sessions, None, 0, 80, Instant::now());
        assert_eq!(line.to_string(), "(no sessions)");
    }

    #[test]
    fn test_render_compact_chips_single_session() {
        use std::time::Instant;
        let mut session = Session::new(
            "test-session-id-1234".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/home/user/myproject")),
        );
        session.status = Status::Working;
        let sessions = vec![session];

        let line = render_compact_session_chips(&sessions, None, 0, 80, Instant::now());
        let text = line.to_string();

        // Should contain status symbol and folder name
        assert!(text.contains("*"), "should contain working symbol");
        assert!(text.contains("myproject"), "should contain folder name");
    }

    #[test]
    fn test_render_compact_chips_selected_session_has_brackets() {
        use std::time::Instant;
        let mut session = Session::new(
            "selected-session".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/home/user/myproject")),
        );
        session.status = Status::Attention;
        let sessions = vec![session];

        let line = render_compact_session_chips(&sessions, Some(0), 0, 80, Instant::now());
        let text = line.to_string();

        // Selected chip should have brackets with folder name
        assert!(
            text.contains("[! myproject]"),
            "selected chip should have brackets with folder name"
        );
    }

    #[test]
    fn test_render_compact_chips_overflow_left() {
        use std::time::Instant;
        let sessions: Vec<Session> = (0..10)
            .map(|i| {
                Session::new(
                    format!("session-{}", i),
                    AgentType::ClaudeCode,
                    Some(PathBuf::from("/tmp")),
                )
            })
            .collect();

        // Scroll to position 5 (5 sessions hidden to the left)
        let line = render_compact_session_chips(&sessions, None, 5, 80, Instant::now());
        let text = line.to_string();

        // Should show left overflow indicator with count
        assert!(text.contains("<- 5+"), "should show left overflow count");
    }

    #[test]
    fn test_render_compact_chips_overflow_right() {
        use std::time::Instant;
        let sessions: Vec<Session> = (0..10)
            .map(|i| {
                Session::new(
                    format!("session-{}", i),
                    AgentType::ClaudeCode,
                    Some(PathBuf::from("/tmp")),
                )
            })
            .collect();

        // At position 0, with 80 width fitting ~3 chips, should have 7 hidden on right
        let line = render_compact_session_chips(&sessions, None, 0, 80, Instant::now());
        let text = line.to_string();

        // Should show right overflow indicator
        assert!(
            text.contains("+ ->"),
            "should show right overflow indicator"
        );
    }

    #[test]
    fn test_render_compact_chips_fallback_to_session_id_when_no_working_dir() {
        use std::time::Instant;
        let mut session = Session::new(
            "fallback-session-id-12345".to_string(),
            AgentType::ClaudeCode,
            None, // No working_dir
        );
        session.status = Status::Working;
        let sessions = vec![session];

        let line = render_compact_session_chips(&sessions, None, 0, 80, Instant::now());
        let text = line.to_string();

        // Should fallback to first 8 chars of session_id
        assert!(text.contains("*"), "should contain working symbol");
        assert!(
            text.contains("fallback"),
            "should contain short session_id as fallback: {}",
            text
        );
    }

    #[test]
    fn test_render_compact_chips_truncates_long_folder_names() {
        use std::time::Instant;
        let mut session = Session::new(
            "test-session".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from(
                "/home/user/very-long-project-name-that-exceeds-limit",
            )),
        );
        session.status = Status::Working;
        let sessions = vec![session];

        let line = render_compact_session_chips(&sessions, None, 0, 80, Instant::now());
        let text = line.to_string();

        // Should truncate folder name from start, keeping end with ellipsis
        assert!(text.contains("*"), "should contain working symbol");
        assert!(
            text.contains("...eds-limit"),
            "should truncate long folder name from start with ellipsis: {}",
            text
        );
    }

    #[test]
    fn test_render_compact_chips_no_overflow_when_all_fit() {
        use std::time::Instant;
        let sessions: Vec<Session> = (0..3)
            .map(|i| {
                Session::new(
                    format!("session-{}", i),
                    AgentType::ClaudeCode,
                    Some(PathBuf::from("/tmp")),
                )
            })
            .collect();

        // Wide terminal (80 chars) should fit all 3 sessions
        let line = render_compact_session_chips(&sessions, None, 0, 80, Instant::now());
        let text = line.to_string();

        // Should NOT have overflow indicators with counts
        assert!(
            !text.contains("<- 1+"),
            "should not show left overflow when all fit"
        );
        assert!(
            !text.contains("1+ ->"),
            "should not show right overflow when all fit"
        );
    }

    #[test]
    fn test_app_scroll_compact_left() {
        let mut app = make_app_with_sessions(10);
        app.compact_scroll_offset = 5;

        app.scroll_compact_left();
        assert_eq!(app.compact_scroll_offset, 4);

        // Should clamp at 0
        app.compact_scroll_offset = 0;
        app.scroll_compact_left();
        assert_eq!(app.compact_scroll_offset, 0);
    }

    #[test]
    fn test_app_scroll_compact_right() {
        let mut app = make_app_with_sessions(10);
        app.compact_scroll_offset = 5;

        app.scroll_compact_right();
        assert_eq!(app.compact_scroll_offset, 6);

        // Should clamp at sessions.len() - 1
        app.compact_scroll_offset = 9;
        app.scroll_compact_right();
        assert_eq!(app.compact_scroll_offset, 9);
    }

    #[test]
    fn test_app_ensure_selected_visible_scrolls_left() {
        let mut app = make_app_with_sessions(10);
        app.compact_scroll_offset = 5;
        app.selected_index = Some(3); // Selected is before viewport

        app.ensure_selected_visible_compact(4); // max_visible = 4

        // Should scroll left to show selected session
        assert_eq!(app.compact_scroll_offset, 3);
    }

    #[test]
    fn test_app_ensure_selected_visible_scrolls_right() {
        let mut app = make_app_with_sessions(10);
        app.compact_scroll_offset = 0;
        app.selected_index = Some(8); // Selected is after viewport

        app.ensure_selected_visible_compact(4); // max_visible = 4

        // Should scroll right to show selected session
        // viewport should be [5, 6, 7, 8] so offset = 5
        assert_eq!(app.compact_scroll_offset, 5);
    }

    #[test]
    fn test_app_ensure_selected_visible_no_scroll_when_visible() {
        let mut app = make_app_with_sessions(10);
        app.compact_scroll_offset = 3;
        app.selected_index = Some(5); // Selected is within viewport [3, 4, 5, 6]

        app.ensure_selected_visible_compact(4); // max_visible = 4

        // Should not change offset
        assert_eq!(app.compact_scroll_offset, 3);
    }

    #[test]
    fn test_two_line_layout_with_pagination() {
        let mut app = make_app_with_sessions(10);
        app.selected_index = Some(0);

        // Render in TwoLine mode
        let buffer = render_dashboard_to_buffer(&mut app, 80, 2);

        assert_eq!(app.layout_mode, crate::tui::app::LayoutMode::TwoLine);

        // Line 0 should contain session chips
        let line0 = row_text(&buffer, 0);
        assert!(
            line0.contains("session-") || line0.contains("*") || line0.contains("!"),
            "Line 0 should contain session chips: {}",
            line0
        );
    }

    // --- Dynamic chip width and new style tests (acd-2d9a) ---

    #[test]
    fn test_dynamic_chip_width_short_names_not_padded() {
        use std::time::Instant;
        let mut s1 = Session::new(
            "session-1".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/src")),
        );
        s1.status = Status::Working;
        let sessions = vec![s1];

        let line = render_compact_session_chips(&sessions, None, 0, 80, Instant::now());
        let text = line.to_string();

        // Short name "src" should not be padded to 18 chars
        // Chip width: ' ' + '*' + ' ' + "src" = 6 chars (not 18)
        assert!(
            text.contains(" * src"),
            "should contain short name without padding: {}",
            text
        );
    }

    #[test]
    fn test_truncate_from_start_keeps_end() {
        use std::time::Instant;
        let mut session = Session::new(
            "test-session".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/my-very-long-project-name")),
        );
        session.status = Status::Working;
        let sessions = vec![session];

        let line = render_compact_session_chips(&sessions, None, 0, 80, Instant::now());
        let text = line.to_string();

        // Should keep end: "...ject-name" (12 chars max)
        assert!(
            text.contains("...ject-name"),
            "should truncate from start, keeping end: {}",
            text
        );
    }

    #[test]
    fn test_pipe_separator_style() {
        use std::time::Instant;
        let sessions: Vec<Session> = (0..3)
            .map(|i| {
                let mut s = Session::new(
                    format!("session-{}", i),
                    AgentType::ClaudeCode,
                    Some(PathBuf::from(format!("/tmp/proj{}", i))),
                );
                s.status = Status::Working;
                s
            })
            .collect();

        let line = render_compact_session_chips(&sessions, None, 0, 80, Instant::now());
        let text = line.to_string();

        // Should have " | " separators between chips
        assert!(
            text.contains(" |"),
            "should have pipe separators between chips: {}",
            text
        );
    }

    #[test]
    fn test_brackets_only_on_focused_chip() {
        use std::time::Instant;
        let sessions: Vec<Session> = (0..3)
            .map(|i| {
                let mut s = Session::new(
                    format!("session-{}", i),
                    AgentType::ClaudeCode,
                    Some(PathBuf::from(format!("/tmp/proj{}", i))),
                );
                s.status = Status::Working;
                s
            })
            .collect();

        // Select middle session
        let line = render_compact_session_chips(&sessions, Some(1), 0, 80, Instant::now());
        let text = line.to_string();

        // Should have brackets around focused chip only
        assert!(
            text.contains("[* proj1]"),
            "focused chip should have brackets: {}",
            text
        );
        // Unfocused chips should not have brackets
        assert!(
            text.contains(" * proj0"),
            "unfocused chip should not have brackets: {}",
            text
        );
    }

    #[test]
    fn test_focused_chip_bracket_style_matches_chip_content() {
        // Regression test for acd-uvq5: the '[' and ']' brackets must use
        // the same fg color and modifiers as the chip content text.
        use ratatui::style::Modifier;
        use std::time::Instant;

        let sessions: Vec<Session> = (0..3)
            .map(|i| {
                let mut s = Session::new(
                    format!("session-{}", i),
                    AgentType::ClaudeCode,
                    Some(PathBuf::from(format!("/tmp/proj{}", i))),
                );
                s.status = Status::Working;
                s
            })
            .collect();

        // Select the middle session so it is NOT the last visible chip.
        // This exercises the code path where ']' was previously rendered
        // with DarkGray inside the next chip's separator.
        let line = render_compact_session_chips(&sessions, Some(1), 0, 80, Instant::now());

        // Collect (text, style) pairs for all spans
        let span_pairs: Vec<(&str, Style)> = line
            .spans
            .iter()
            .map(|s| (s.content.as_ref(), s.style))
            .collect();

        // Find the '[' span for the focused chip
        let bracket_open = span_pairs
            .iter()
            .find(|(text, _)| *text == "[")
            .expect("should have a '[' span for focused chip");

        // Find the chip content span (contains the symbol and name)
        let chip_content = span_pairs
            .iter()
            .find(|(text, _)| text.contains("* proj1"))
            .expect("should have chip content span for focused chip");

        // Find the ']' span that closes the focused chip
        let bracket_close = span_pairs
            .iter()
            .find(|(text, _)| *text == "]")
            .expect("should have a ']' span for focused chip");

        // All three spans must share the same fg color
        assert_eq!(
            bracket_open.1.fg, chip_content.1.fg,
            "opening bracket fg color must match chip content fg color"
        );
        assert_eq!(
            bracket_close.1.fg, chip_content.1.fg,
            "closing bracket fg color must match chip content fg color"
        );

        // All three spans must have the BOLD modifier
        assert!(
            bracket_open.1.add_modifier.contains(Modifier::BOLD),
            "opening bracket must be BOLD"
        );
        assert!(
            chip_content.1.add_modifier.contains(Modifier::BOLD),
            "chip content must be BOLD when focused"
        );
        assert!(
            bracket_close.1.add_modifier.contains(Modifier::BOLD),
            "closing bracket must be BOLD"
        );
    }

    #[test]
    fn test_overflow_format_with_count() {
        use std::time::Instant;
        // Create many sessions to ensure overflow on both sides
        let sessions: Vec<Session> = (0..20)
            .map(|i| {
                Session::new(
                    format!("session-{}", i),
                    AgentType::ClaudeCode,
                    Some(PathBuf::from(format!("/tmp/project{}", i))),
                )
            })
            .collect();

        // Scroll to position 5 (5 hidden left, should have overflow on right too)
        let line = render_compact_session_chips(&sessions, None, 5, 80, Instant::now());
        let text = line.to_string();

        // Should show left overflow with format: "<- N+|" (no space before pipe)
        assert!(
            text.contains("<- 5+|"),
            "should show left overflow with N+ format: {}",
            text
        );
        // Should show right overflow with format: "|N+ ->"
        assert!(
            text.contains("+ ->"),
            "should show right overflow indicator: {}",
            text
        );
    }

    #[test]
    fn test_overflow_format_zero_count() {
        use std::time::Instant;
        let sessions: Vec<Session> = (0..3)
            .map(|i| {
                Session::new(
                    format!("session-{}", i),
                    AgentType::ClaudeCode,
                    Some(PathBuf::from(format!("/tmp/p{}", i))),
                )
            })
            .collect();

        // All sessions fit, no overflow
        let line = render_compact_session_chips(&sessions, None, 0, 80, Instant::now());
        let text = line.to_string();

        // Should show zero format: "<- 0 |" and "| 0 ->" (with space)
        assert!(
            text.contains("<- 0 |"),
            "should show left zero indicator with space: {}",
            text
        );
        assert!(
            text.contains("| 0 ->"),
            "should show right zero indicator with space: {}",
            text
        );
    }

    #[test]
    fn test_overflow_indicators_always_shown() {
        use std::time::Instant;
        let sessions: Vec<Session> = (0..2)
            .map(|i| {
                Session::new(
                    format!("session-{}", i),
                    AgentType::ClaudeCode,
                    Some(PathBuf::from(format!("/tmp/p{}", i))),
                )
            })
            .collect();

        let line = render_compact_session_chips(&sessions, None, 0, 80, Instant::now());
        let text = line.to_string();

        // Overflow indicators should always be present (never hidden)
        assert!(
            text.starts_with("<-"),
            "should always show left overflow indicator: {}",
            text
        );
        assert!(
            text.ends_with("->"),
            "should always show right overflow indicator: {}",
            text
        );
    }
}
