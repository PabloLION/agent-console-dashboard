//! Session detail modal overlay view.
//!
//! Renders a centered modal showing comprehensive information about a single
//! session: status, working directory, session ID, API usage, and state
//! transition history. Supports scrolling through history entries.

use crate::{Session, Status};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use std::time::Instant;

/// Maximum history entries visible without scrolling.
const MAX_VISIBLE_HISTORY: usize = 5;

/// Renders the session detail modal overlay.
///
/// The modal is centered in the given `area` and displays session metadata,
/// API usage summary, state history (with scroll support), and action hints.
pub fn render_detail(
    frame: &mut Frame,
    session: &Session,
    area: Rect,
    history_scroll: usize,
    now: Instant,
) {
    let modal_width = 50u16.min(area.width.saturating_sub(4));
    let modal_height = 16u16.min(area.height.saturating_sub(2));

    if modal_width < 20 || modal_height < 8 {
        return; // Too small to render meaningfully
    }

    let x = area.x + (area.width.saturating_sub(modal_width)) / 2;
    let y = area.y + (area.height.saturating_sub(modal_height)) / 2;
    let modal_area = Rect::new(x, y, modal_width, modal_height);

    // Clear background
    frame.render_widget(Clear, modal_area);

    // Derive title from working directory basename or session ID
    let title = session
        .working_dir
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or(&session.session_id);

    let block = Block::default()
        .title(format!("── {} ──", title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let lines = build_detail_lines(session, inner.width, history_scroll, now, true);

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

/// Renders the session detail as an inline panel below the session list.
///
/// Unlike `render_detail`, this renders into the given `area` directly
/// without clearing background or centering. Used for the non-modal layout
/// where detail appears as a fixed section below the session list.
pub fn render_inline_detail(
    frame: &mut Frame,
    session: &Session,
    area: Rect,
    history_scroll: usize,
    now: Instant,
) {
    if area.height < 3 || area.width < 20 {
        return; // Too small to render meaningfully
    }

    let title = session
        .working_dir
        .as_ref()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or(&session.session_id);

    let block = Block::default()
        .title(format!("── {} ──", title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = build_detail_lines(session, inner.width, history_scroll, now, false);

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

/// Renders a placeholder message when no session is selected.
///
/// Shows a hint message with keybinding guidance to help users understand
/// how to navigate and interact with sessions.
pub fn render_detail_placeholder(frame: &mut Frame, area: Rect) {
    if area.height < 3 || area.width < 20 {
        return;
    }

    let block = Block::default()
        .title("── Detail ──")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Build hint text with keybinding guidance
    let lines = vec![
        Line::from(vec![Span::styled(
            "Select a session to see details",
            Style::default().fg(Color::DarkGray),
        )]),
        Line::from(vec![]),
        Line::from(vec![
            Span::styled("[j/k] ", Style::default().fg(Color::Cyan)),
            Span::styled("Navigate  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[Enter] ", Style::default().fg(Color::Cyan)),
            Span::styled("Hook  ", Style::default().fg(Color::DarkGray)),
            Span::styled("[q] ", Style::default().fg(Color::Cyan)),
            Span::styled("Quit", Style::default().fg(Color::DarkGray)),
        ]),
    ];

    let text = Paragraph::new(lines);
    frame.render_widget(text, inner);
}

/// Builds the content lines for a detail view (shared between modal and inline).
///
/// When `show_actions` is true, footer action hints are appended (modal mode).
/// For inline mode, actions are omitted since keybindings are shown in the
/// main footer.
fn build_detail_lines<'a>(
    session: &'a Session,
    panel_width: u16,
    history_scroll: usize,
    now: Instant,
    show_actions: bool,
) -> Vec<Line<'a>> {
    let mut lines: Vec<Line<'a>> = Vec::new();

    // Status line
    let elapsed = now.duration_since(session.since);
    let status_color = status_color(session.status);
    let elapsed_str = super::dashboard::format_duration_secs(elapsed.as_secs());
    lines.push(Line::from(vec![
        Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(
            format!("{}", session.status),
            Style::default().fg(status_color),
        ),
        Span::raw(format!(" ({})", elapsed_str)),
    ]));

    // Working directory
    let (wd, is_error) = match &session.working_dir {
        None => ("<error>".to_string(), true),
        Some(path) => (path.display().to_string(), false),
    };
    let max_wd_len = (panel_width as usize).saturating_sub(13);
    let wd_display = if wd.len() > max_wd_len {
        format!("…{}", &wd[wd.len().saturating_sub(max_wd_len - 1)..])
    } else {
        wd
    };
    let wd_style = if is_error {
        Style::default().fg(Color::Red)
    } else {
        Style::default()
    };
    lines.push(Line::from(vec![
        Span::styled("Dir: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(wd_display, wd_style),
    ]));

    // Session ID (truncated)
    let id_max = (panel_width as usize).saturating_sub(5);
    let id_display = if session.session_id.len() > id_max {
        format!("{}…", &session.session_id[..id_max.saturating_sub(1)])
    } else {
        session.session_id.clone()
    };
    lines.push(Line::from(vec![
        Span::styled("ID: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(id_display),
    ]));

    // API usage placeholder
    lines.push(Line::from(vec![
        Span::styled("Quota: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled("--", Style::default().fg(Color::DarkGray)),
    ]));

    // Blank separator
    lines.push(Line::raw(""));

    // History
    lines.push(Line::from(vec![Span::styled(
        "History:",
        Style::default().add_modifier(Modifier::BOLD),
    )]));

    if session.history.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "  (no transitions)",
            Style::default().fg(Color::DarkGray),
        )]));
    } else {
        let total = session.history.len();
        let start = history_scroll.min(total.saturating_sub(MAX_VISIBLE_HISTORY));
        let end = (start + MAX_VISIBLE_HISTORY).min(total);

        // Show most recent first (reverse order)
        let reversed: Vec<_> = session.history.iter().rev().collect();
        let visible = &reversed[start..end];

        for (idx, transition) in visible.iter().enumerate() {
            // Calculate duration in this state
            let duration_secs = if idx == 0 {
                // Most recent transition - duration from then until now (dynamic)
                now.duration_since(transition.timestamp).as_secs()
            } else {
                // Historical transition - use the duration stored in the StateTransition
                transition.duration.as_secs()
            };

            let duration_str = super::dashboard::format_duration_secs(duration_secs);
            lines.push(Line::from(vec![
                Span::raw(format!("  {}  ", duration_str)),
                Span::styled(
                    format!("{}", transition.from),
                    Style::default().fg(status_color_for(transition.from)),
                ),
                Span::raw(" → "),
                Span::styled(
                    format!("{}", transition.to),
                    Style::default().fg(status_color_for(transition.to)),
                ),
            ]));
        }

        if total > MAX_VISIBLE_HISTORY {
            let indicator = format!("  [{}/{} entries]", end - start, total);
            lines.push(Line::from(vec![Span::styled(
                indicator,
                Style::default().fg(Color::DarkGray),
            )]));
        }
    }

    if show_actions {
        // Footer actions (modal mode only)
        let mut actions = vec![Span::styled(
            "[ESC] Back",
            Style::default().fg(Color::DarkGray),
        )];
        if session.status == Status::Closed {
            actions.insert(
                0,
                Span::styled("[R]esurrect  ", Style::default().fg(Color::Yellow)),
            );
        }
        actions.insert(
            actions.len() - 1,
            Span::styled("[C]lose  ", Style::default().fg(Color::Red)),
        );
        lines.push(Line::from(actions));
    }

    lines
}

/// Returns the display color for a session status.
fn status_color(status: Status) -> Color {
    status_color_for(status)
}

/// Maps a status to its display color.
fn status_color_for(status: Status) -> Color {
    match status {
        Status::Working => Color::Green,
        Status::Attention => Color::Yellow,
        Status::Question => Color::Magenta,
        Status::Closed => Color::DarkGray,
    }
}


#[cfg(test)]
mod tests;
