# Horizontal Pagination Research for Compact Two-Line Layout

**Issue**: acd-4398
**Agent**: tui-rendering
**Date**: 2026-02-18
**Ratatui Version**: 0.29.0

## Executive Summary

Research into horizontal pagination for rendering N session "chips" in a single
ratatui Line with overflow indicators (`<- N+` left, `N+ ->` right). No
dedicated ratatui widget exists for horizontal scrollable chip layouts.
Recommend custom implementation using existing primitives: `Line`, `Span`, and
manual pagination state.

## Research Findings

### 1. Ratatui Built-in Widgets

#### Tabs Widget

Ratatui provides a `Tabs` widget (`ratatui::widgets::Tabs`) that renders
horizontal tab bars. However, it has critical limitations for our use case:

**Features:**
- Renders multiple titles horizontally
- Supports highlight styling for selected tab
- Automatic spacing with dividers

**Limitations:**
- **No pagination/scrolling support** — renders all tabs or truncates
- **No overflow indicators** — cannot show `<- N+` / `N+ ->` markers
- **Fixed layout** — cannot customize per-tab rendering beyond simple strings
- **No click target tracking** — doesn't expose tab positions for mouse handling

**Verdict:** Not suitable. Would need complete reimplementation to add pagination.

#### List Widget

The `List` widget (currently used in ACD) supports vertical scrolling but has no
horizontal equivalent. It cannot be rotated or configured for horizontal layout.

**Verdict:** Not applicable.

#### Custom Widget Approach

Ratatui's widget system allows custom implementations via the `Widget` trait.
However, for a simple horizontal list of chips, direct `Line`/`Span` rendering
is more appropriate than a full custom widget.

### 2. Community Solutions

#### Search Results

Searched crates.io and GitHub for:
- "ratatui horizontal pagination"
- "ratatui horizontal scroll"
- "ratatui tab bar pagination"
- "ratatui chip layout"

**No existing crates found** that implement horizontal scrollable layouts with
overflow indicators for ratatui 0.29.

#### Relevant Examples

The [ratatui examples repository](https://github.com/ratatui-org/ratatui/tree/main/examples)
includes `tabs.rs` showing basic `Tabs` widget usage, but no pagination examples.

### 3. Design Pattern Analysis

#### Common TUI Patterns

Horizontal pagination in TUI applications typically follows one of these patterns:

**Pattern A: Viewport Window (Recommended)**
```
<- 3+ [chip1] [chip2] [chip3] [chip4] [chip5] 7+ ->
      ^----- visible window (5 chips) ------^
```

State:
- Total items: N
- Viewport width: available terminal width
- Current offset: which chip is leftmost in viewport
- Overflow left: count of hidden items to the left
- Overflow right: count of hidden items to the right

**Pattern B: Page-Based**
```
<- Pg1 [chip6] [chip7] [chip8] [chip9] [chip10] Pg3 ->
       ^----- page 2 (5 chips per page) -------^
```

State:
- Total items: N
- Items per page: fixed count
- Current page: 1-indexed page number

**Pattern C: Centered Selection**
```
<- 5+ [chip4] [chip5] >chip6< [chip7] [chip8] 9+ ->
                       ^----- selected ------^
```

State:
- Same as Pattern A but viewport centers on selected item

#### Recommendation

**Use Pattern A (Viewport Window)** for ACD compact layout:

Reasons:
1. **Flexible sizing** — adapts to terminal width without fixed page boundaries
2. **Smooth scrolling** — left/right arrow keys move by one chip
3. **Clear overflow counts** — `N+` format shows exact hidden count
4. **Simple state** — single offset integer, no page arithmetic

### 4. Implementation Strategy

#### Required State (in `App` struct)

```rust
/// Horizontal scroll offset for compact layout (which session is leftmost).
pub compact_scroll_offset: usize,
```

Initialize to 0. When user presses left/right arrow or scrolls, increment/decrement.

#### Rendering Algorithm

Pseudocode for rendering N session chips into available width:

```rust
fn render_compact_session_chips(
    sessions: &[Session],
    selected_index: Option<usize>,
    scroll_offset: usize,
    available_width: u16,
) -> Line<'static> {
    const CHIP_WIDTH: usize = 20;  // "[Working] session-1  "
    const OVERFLOW_WIDTH: usize = 8;  // "<- N+  " or "  N+ ->"

    // Calculate how many chips fit
    let content_width = (available_width as usize).saturating_sub(OVERFLOW_WIDTH * 2);
    let max_visible = content_width / CHIP_WIDTH;

    // Determine visible range
    let start = scroll_offset;
    let end = (start + max_visible).min(sessions.len());
    let visible_sessions = &sessions[start..end];

    // Calculate overflow counts
    let overflow_left = start;
    let overflow_right = sessions.len().saturating_sub(end);

    // Build spans
    let mut spans = vec![];

    // Left overflow indicator
    if overflow_left > 0 {
        spans.push(Span::styled(
            format!("<- {}+ ", overflow_left),
            Style::default().fg(Color::DarkGray),
        ));
    } else {
        spans.push(Span::raw("       ")); // padding for alignment
    }

    // Visible chips
    for (index, session) in visible_sessions.iter().enumerate() {
        let global_index = start + index;
        let is_selected = selected_index == Some(global_index);
        spans.push(format_session_chip(session, is_selected));
    }

    // Right overflow indicator
    if overflow_right > 0 {
        spans.push(Span::styled(
            format!(" {}+ ->", overflow_right),
            Style::default().fg(Color::DarkGray),
        ));
    }

    Line::from(spans)
}

fn format_session_chip(session: &Session, is_selected: bool) -> Span<'static> {
    let status_symbol = status_symbol(session.status);
    let status_color = status_color(session.status);

    let text = if is_selected {
        format!("[{} {}]", status_symbol, session.session_id_short())
    } else {
        format!(" {} {} ", status_symbol, session.session_id_short())
    };

    Span::styled(
        text,
        if is_selected {
            Style::default().fg(status_color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(status_color)
        },
    )
}
```

#### Chip Format Design

Each chip should be compact and scannable:

**Option 1: Status + Short ID**
```
[● abc123]  [○ def456]  [? ghi789]
```
Width: ~12-15 chars per chip

**Option 2: Status + Directory Basename**
```
[● myrepo]  [○ other]  [? third]
```
Width: variable, needs truncation

**Option 3: Status Symbol Only (Ultra-Compact)**
```
● ○ ? ● ○ ×
```
Width: 2 chars per chip (dense but lacks context)

**Recommendation:** Start with Option 1 (status + short ID). Short ID = first
8 chars of session_id. Provides balance between density and readability.

#### Mouse Interaction

Mouse clicks on chips require coordinate-to-chip mapping:

```rust
fn calculate_clicked_chip(
    mouse_col: u16,
    scroll_offset: usize,
    sessions: &[Session],
    available_width: u16,
) -> Option<usize> {
    const CHIP_WIDTH: usize = 20;
    const OVERFLOW_WIDTH: usize = 8;

    // Subtract left overflow indicator width
    let content_start = OVERFLOW_WIDTH;
    if (mouse_col as usize) < content_start {
        return None; // Click on left indicator
    }

    let relative_col = (mouse_col as usize) - content_start;
    let chip_index = relative_col / CHIP_WIDTH;
    let global_index = scroll_offset + chip_index;

    if global_index < sessions.len() {
        Some(global_index)
    } else {
        None
    }
}
```

#### Scroll Navigation

Keyboard/mouse scroll updates `compact_scroll_offset`:

```rust
// Left arrow or scroll up
if compact_scroll_offset > 0 {
    compact_scroll_offset -= 1;
}

// Right arrow or scroll down
let max_offset = sessions.len().saturating_sub(max_visible);
if compact_scroll_offset < max_offset {
    compact_scroll_offset += 1;
}
```

#### Auto-Scroll to Selection

When selected session changes (j/k keys, mouse click), auto-scroll viewport to
ensure selected chip is visible:

```rust
fn ensure_selected_visible(
    selected_index: Option<usize>,
    scroll_offset: &mut usize,
    max_visible: usize,
    total_sessions: usize,
) {
    if let Some(idx) = selected_index {
        // If selected is before viewport, scroll left
        if idx < *scroll_offset {
            *scroll_offset = idx;
        }
        // If selected is after viewport, scroll right
        else if idx >= *scroll_offset + max_visible {
            *scroll_offset = (idx + 1).saturating_sub(max_visible);
        }
    }
}
```

### 5. Layout Integration

The compact two-line layout (from acd-hex scope) should be:

```
Line 1: [● abc123] [○ def456] [? ghi789] <-- horizontal session chips
Line 2: API Usage: 1.2M / 5.0M tokens      <-- usage widget
```

Both lines are independent widgets rendered in a vertical Layout:

```rust
let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(1), // session chips line
        Constraint::Length(1), // API usage line
    ])
    .split(area);

let session_line = render_compact_session_chips(...);
frame.render_widget(Paragraph::new(session_line), chunks[0]);

let usage_line = render_api_usage_line(...);
frame.render_widget(Paragraph::new(usage_line), chunks[1]);
```

### 6. Testing Strategy

Unit tests needed:

1. **Pagination bounds** — verify overflow counts with 0, 1, many sessions
2. **Chip formatting** — verify status symbol, color, selection brackets
3. **Mouse click mapping** — verify coordinate-to-index translation
4. **Auto-scroll** — verify viewport follows selection
5. **Truncation** — verify behavior when terminal width < 1 chip width

Integration tests:

1. **Render to buffer** — snapshot test for expected visual output
2. **Interaction sequence** — scroll left/right, verify viewport state

### 7. Performance Considerations

**Concern:** Rendering N chips every frame for 100 sessions.

**Mitigation:**
- Only visible chips are rendered (typically 5-10)
- `Line::from(vec![Span])` is cheap (stack allocation)
- Status color/symbol lookups are O(1) match statements

**Expected cost:** ~10-20 Span allocations per frame = negligible.

## Recommendations

### Immediate Next Steps (acd-6wg6)

1. Add `compact_scroll_offset: usize` to `App` struct
2. Implement `render_compact_session_chips()` in new module
   `./src/tui/views/compact/chips.rs`
3. Add keyboard handlers for left/right arrow (update scroll offset)
4. Add auto-scroll logic to `select_next()` / `select_previous()`
5. Write unit tests for pagination bounds and chip formatting

### Deferred to P4

- **Three-line layout** (acd-bxfc) — adds history line below session chips
- **Global activity feed** (acd-qgv5) — unified event stream across sessions
- **Mouse scroll wheel horizontal navigation** — requires crossterm feature detection

## References

- [Ratatui Tabs Widget Docs](https://docs.rs/ratatui/0.29.0/ratatui/widgets/struct.Tabs.html)
- [Ratatui Widget Trait](https://docs.rs/ratatui/0.29.0/ratatui/widgets/trait.Widget.html)
- [Crossterm Mouse Events](https://docs.rs/crossterm/0.28/crossterm/event/struct.MouseEvent.html)

## Open Questions

1. **Chip width calculation** — Should chip width be fixed (20 chars) or
   dynamic based on session_id length? Fixed is simpler but may waste space.

2. **Overflow indicator interactivity** — Should clicking `<- N+` jump to
   previous page (scroll by max_visible)? Or is arrow key navigation sufficient?

3. **Session ID shortening** — Use first 8 chars, last 8 chars, or hash-based
   short ID? Need to ensure uniqueness.

4. **Color scheme** — Should inactive sessions use DarkGray in compact mode
   (matching full TUI) or stay colored for density?

## Conclusion

No existing ratatui widget or crate provides horizontal pagination with overflow
indicators. Recommend custom implementation using viewport window pattern with
`Line`/`Span` primitives. Implementation is straightforward with ~200 lines of
code and clear test boundaries. Ready for implementation phase (acd-6wg6).
