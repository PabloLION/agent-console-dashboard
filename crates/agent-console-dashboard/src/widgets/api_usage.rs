//! API usage widget for displaying Claude quota utilization.
//!
//! Renders 5-hour and 7-day utilization percentages with color-coded
//! thresholds and optional reset time. This widget is a **stateless
//! renderer** that only reads from [`WidgetContext::usage`] and never
//! calls the usage-fetching function directly.
//!
//! # Display Formats
//!
//! - **Full** (width >= 30): `Quota: 5h 8% | 7d 77% | resets 2h 15m`
//! - **Compact** (width < 30): `[5h:8% 7d:77%]`
//! - **Unavailable**: `Quota: --` in dark gray
//!
//! # Color Thresholds
//!
//! | Utilization | Color  |
//! |-------------|--------|
//! | < 80%       | Green  |
//! | 80%-95%     | Yellow |
//! | > 95%       | Red    |

use chrono::{DateTime, Utc};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

use super::{Widget, WidgetContext};

/// Widget displaying Claude API usage quotas.
///
/// Reads usage data from [`WidgetContext::usage`] and renders
/// utilization percentages with color-coded severity. Never fetches
/// usage data on its own.
pub struct ApiUsageWidget;

impl ApiUsageWidget {
    /// Create a new `ApiUsageWidget`.
    pub fn new() -> Self {
        Self
    }
}

impl Default for ApiUsageWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for ApiUsageWidget {
    fn render(&self, width: u16, context: &WidgetContext) -> Line<'_> {
        let usage = match context.usage {
            Some(u) => u,
            None => {
                return Line::from(vec![Span::styled(
                    "Quota: --",
                    Style::default().fg(Color::DarkGray),
                )]);
            }
        };

        let five_h_pct = usage.five_hour.utilization;
        let seven_d_pct = usage.seven_day.utilization;

        if width >= 30 {
            render_full(five_h_pct, seven_d_pct, usage.five_hour.resets_at)
        } else {
            render_compact(five_h_pct, seven_d_pct)
        }
    }

    fn id(&self) -> &'static str {
        "api-usage"
    }

    fn min_width(&self) -> u16 {
        15
    }
}

/// Render full format: `Quota: 5h 8% | 7d 77% | resets 2h 15m`
fn render_full(
    five_h_pct: f64,
    seven_d_pct: f64,
    resets_at: Option<DateTime<Utc>>,
) -> Line<'static> {
    let mut spans = vec![
        Span::raw("Quota: 5h "),
        Span::styled(
            format!("{:.0}%", five_h_pct.floor()),
            Style::default().fg(utilization_color(five_h_pct)),
        ),
        Span::raw(" | 7d "),
        Span::styled(
            format!("{:.0}%", seven_d_pct.floor()),
            Style::default().fg(utilization_color(seven_d_pct)),
        ),
    ];

    if let Some(reset) = resets_at {
        spans.push(Span::raw(" | resets "));
        spans.push(Span::raw(format_reset_time(reset)));
    }

    Line::from(spans)
}

/// Render compact format: `[5h:8% 7d:77%]`
fn render_compact(five_h_pct: f64, seven_d_pct: f64) -> Line<'static> {
    Line::from(vec![
        Span::raw("[5h:"),
        Span::styled(
            format!("{:.0}%", five_h_pct.floor()),
            Style::default().fg(utilization_color(five_h_pct)),
        ),
        Span::raw(" 7d:"),
        Span::styled(
            format!("{:.0}%", seven_d_pct.floor()),
            Style::default().fg(utilization_color(seven_d_pct)),
        ),
        Span::raw("]"),
    ])
}

/// Map utilization percentage to a color.
///
/// - < 80%: Green (normal usage)
/// - 80%-95%: Yellow (elevated usage)
/// - > 95%: Red (critical usage)
fn utilization_color(pct: f64) -> Color {
    if pct > 95.0 {
        Color::Red
    } else if pct > 80.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

/// Format a reset time as a human-readable relative duration.
///
/// Produces strings like `2h 15m` or `45m` relative to now.
fn format_reset_time(resets_at: DateTime<Utc>) -> String {
    let duration = resets_at - Utc::now();
    let hours = duration.num_hours();
    let minutes = (duration.num_minutes() % 60).abs();
    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

/// Factory function for [`WidgetRegistry`](super::WidgetRegistry).
pub fn create() -> Box<dyn Widget> {
    Box::new(ApiUsageWidget::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Session;
    use claude_usage::{UsageData, UsagePeriod};

    fn make_usage(five_h: f64, seven_d: f64, resets_at: Option<DateTime<Utc>>) -> UsageData {
        UsageData {
            five_hour: UsagePeriod {
                utilization: five_h,
                resets_at,
            },
            seven_day: UsagePeriod {
                utilization: seven_d,
                resets_at: None,
            },
            seven_day_sonnet: None,
            extra_usage: None,
        }
    }

    // --- Widget metadata ---

    #[test]
    fn test_widget_id() {
        let w = ApiUsageWidget::new();
        assert_eq!(w.id(), "api-usage");
    }

    #[test]
    fn test_widget_min_width() {
        let w = ApiUsageWidget::new();
        assert_eq!(w.min_width(), 15);
    }

    #[test]
    fn test_widget_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ApiUsageWidget>();
    }

    // --- Unavailable usage ---

    #[test]
    fn test_unavailable_usage_shows_placeholder() {
        let sessions: Vec<Session> = vec![];
        let ctx = WidgetContext::new(&sessions);
        let w = ApiUsageWidget::new();
        let line = w.render(40, &ctx);
        assert_eq!(line.to_string(), "Quota: --");
    }

    #[test]
    fn test_unavailable_usage_is_dark_gray() {
        let sessions: Vec<Session> = vec![];
        let ctx = WidgetContext::new(&sessions);
        let w = ApiUsageWidget::new();
        let line = w.render(40, &ctx);
        let span = &line.spans[0];
        assert_eq!(span.style.fg, Some(Color::DarkGray));
    }

    // --- Full format ---

    #[test]
    fn test_full_format_with_usage() {
        let usage = make_usage(8.0, 77.0, None);
        let sessions: Vec<Session> = vec![];
        let ctx = WidgetContext::new(&sessions).with_usage(&usage);
        let w = ApiUsageWidget::new();
        let line = w.render(40, &ctx);
        let text = line.to_string();
        assert!(text.contains("Quota:"), "expected 'Quota:' in '{}'", text);
        assert!(text.contains("5h"), "expected '5h' in '{}'", text);
        assert!(text.contains("8%"), "expected '8%' in '{}'", text);
        assert!(text.contains("7d"), "expected '7d' in '{}'", text);
        assert!(text.contains("77%"), "expected '77%' in '{}'", text);
    }

    #[test]
    fn test_full_format_with_reset_time() {
        let reset = Utc::now() + chrono::Duration::hours(2) + chrono::Duration::minutes(15);
        let usage = make_usage(8.0, 77.0, Some(reset));
        let sessions: Vec<Session> = vec![];
        let ctx = WidgetContext::new(&sessions).with_usage(&usage);
        let w = ApiUsageWidget::new();
        let line = w.render(40, &ctx);
        let text = line.to_string();
        assert!(text.contains("resets"), "expected 'resets' in '{}'", text);
        assert!(text.contains("2h"), "expected '2h' in '{}'", text);
    }

    // --- Compact format ---

    #[test]
    fn test_compact_format_with_usage() {
        let usage = make_usage(8.0, 77.0, None);
        let sessions: Vec<Session> = vec![];
        let ctx = WidgetContext::new(&sessions).with_usage(&usage);
        let w = ApiUsageWidget::new();
        let line = w.render(25, &ctx);
        let text = line.to_string();
        assert!(text.starts_with('['), "expected '[' start in '{}'", text);
        assert!(text.ends_with(']'), "expected ']' end in '{}'", text);
        assert!(text.contains("5h:8%"), "expected '5h:8%' in '{}'", text);
        assert!(text.contains("7d:77%"), "expected '7d:77%' in '{}'", text);
    }

    // --- Width threshold ---

    #[test]
    fn test_width_29_selects_compact() {
        let usage = make_usage(50.0, 50.0, None);
        let sessions: Vec<Session> = vec![];
        let ctx = WidgetContext::new(&sessions).with_usage(&usage);
        let w = ApiUsageWidget::new();
        let line = w.render(29, &ctx);
        let text = line.to_string();
        assert!(
            text.starts_with('['),
            "width 29 should use compact: '{}'",
            text
        );
    }

    #[test]
    fn test_width_30_selects_full() {
        let usage = make_usage(50.0, 50.0, None);
        let sessions: Vec<Session> = vec![];
        let ctx = WidgetContext::new(&sessions).with_usage(&usage);
        let w = ApiUsageWidget::new();
        let line = w.render(30, &ctx);
        let text = line.to_string();
        assert!(
            text.contains("Quota:"),
            "width 30 should use full: '{}'",
            text
        );
    }

    // --- Color thresholds ---

    #[test]
    fn test_color_green_at_50() {
        assert_eq!(utilization_color(50.0), Color::Green);
    }

    #[test]
    fn test_color_yellow_at_85() {
        assert_eq!(utilization_color(85.0), Color::Yellow);
    }

    #[test]
    fn test_color_red_at_96() {
        assert_eq!(utilization_color(96.0), Color::Red);
    }

    #[test]
    fn test_color_boundary_80_is_green() {
        assert_eq!(utilization_color(80.0), Color::Green);
    }

    #[test]
    fn test_color_boundary_95_is_yellow() {
        assert_eq!(utilization_color(95.0), Color::Yellow);
    }

    // --- Independent coloring ---

    #[test]
    fn test_five_h_and_seven_d_colored_independently() {
        let usage = make_usage(50.0, 96.0, None);
        let sessions: Vec<Session> = vec![];
        let ctx = WidgetContext::new(&sessions).with_usage(&usage);
        let w = ApiUsageWidget::new();
        let line = w.render(40, &ctx);
        // spans: "Quota: 5h ", "8%", " | 7d ", "77%"
        // Index 1 = 5h value (green), Index 3 = 7d value (red)
        let five_h_span = &line.spans[1];
        let seven_d_span = &line.spans[3];
        assert_eq!(five_h_span.style.fg, Some(Color::Green));
        assert_eq!(seven_d_span.style.fg, Some(Color::Red));
    }

    // --- Reset time formatting ---

    #[test]
    fn test_format_reset_time_hours_and_minutes() {
        let future = Utc::now() + chrono::Duration::hours(3) + chrono::Duration::minutes(42);
        let formatted = format_reset_time(future);
        assert!(formatted.contains("3h"), "expected '3h' in '{}'", formatted);
        assert!(
            formatted.contains("42m") || formatted.contains("41m"),
            "expected ~42m in '{}'",
            formatted
        );
    }

    #[test]
    fn test_format_reset_time_minutes_only() {
        let future = Utc::now() + chrono::Duration::minutes(45);
        let formatted = format_reset_time(future);
        assert!(
            !formatted.contains('h'),
            "should not contain 'h': '{}'",
            formatted
        );
        assert!(
            formatted.contains('m'),
            "should contain 'm': '{}'",
            formatted
        );
    }

    // --- Edge cases ---

    #[test]
    fn test_zero_utilization() {
        let usage = make_usage(0.0, 0.0, None);
        let sessions: Vec<Session> = vec![];
        let ctx = WidgetContext::new(&sessions).with_usage(&usage);
        let w = ApiUsageWidget::new();
        let line = w.render(40, &ctx);
        let text = line.to_string();
        assert!(text.contains("0%"), "expected '0%' in '{}'", text);
    }

    #[test]
    fn test_100_utilization() {
        let usage = make_usage(100.0, 100.0, None);
        let sessions: Vec<Session> = vec![];
        let ctx = WidgetContext::new(&sessions).with_usage(&usage);
        let w = ApiUsageWidget::new();
        let line = w.render(40, &ctx);
        let text = line.to_string();
        assert!(text.contains("100%"), "expected '100%' in '{}'", text);
        // 100 > 95 => Red
        let five_h_span = &line.spans[1];
        assert_eq!(five_h_span.style.fg, Some(Color::Red));
    }

    // --- Floor rounding ---

    #[test]
    fn test_floor_rounding_displays_lower_integer() {
        // 79.7% should display as "79%" (floor), not "80%" (round)
        let usage = make_usage(79.7, 94.9, None);
        let sessions: Vec<Session> = vec![];
        let ctx = WidgetContext::new(&sessions).with_usage(&usage);
        let w = ApiUsageWidget::new();
        let line = w.render(40, &ctx);
        let text = line.to_string();
        assert!(text.contains("79%"), "expected '79%' in '{}'", text);
        assert!(text.contains("94%"), "expected '94%' in '{}'", text);
        assert!(
            !text.contains("80%"),
            "should NOT contain '80%' in '{}'",
            text
        );
        assert!(
            !text.contains("95%"),
            "should NOT contain '95%' in '{}'",
            text
        );
    }

    #[test]
    fn test_floor_rounding_compact_format() {
        // Same floor behavior in compact format
        let usage = make_usage(79.7, 94.9, None);
        let sessions: Vec<Session> = vec![];
        let ctx = WidgetContext::new(&sessions).with_usage(&usage);
        let w = ApiUsageWidget::new();
        let line = w.render(25, &ctx);
        let text = line.to_string();
        assert!(text.contains("79%"), "expected '79%' in '{}'", text);
        assert!(text.contains("94%"), "expected '94%' in '{}'", text);
    }

    // --- Structural: no get_usage import ---

    #[test]
    fn test_no_fetch_function_import() {
        // Check that non-test code doesn't reference the fetch function.
        // We split the needle to avoid matching this test's own source.
        let needle = ["get", "_", "usage"].concat();
        let source = include_str!("api_usage.rs");
        // Only check lines outside of #[cfg(test)]
        let mut in_test = false;
        for line in source.lines() {
            if line.contains("#[cfg(test)]") {
                in_test = true;
            }
            if !in_test {
                assert!(
                    !line.contains(&needle),
                    "production code must not reference the fetch function: {line}"
                );
            }
        }
    }

    // --- Factory ---

    #[test]
    fn test_factory_creates_correct_widget() {
        let w = create();
        assert_eq!(w.id(), "api-usage");
        assert_eq!(w.min_width(), 15);
    }
}
