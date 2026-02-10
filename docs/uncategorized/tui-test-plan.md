# TUI Test Plan

TestBackend-based tests for the TUI dashboard. Tests are TDD — written first,
implementation follows. Uses ratatui 0.29 TestBackend.

## Infrastructure

### Shared Helpers (`src/tui/test_utils.rs`)

```rust
#[cfg(test)]
pub(crate) mod test_utils;
```

Helpers to create:

- `test_terminal(width, height)` — creates `Terminal<TestBackend>`
- `row_text(buffer, row)` — extracts row as String
- `row_contains(buffer, row, text)` — checks substring in row
- `find_row_with_text(buffer, text)` — searches all rows
- `assert_fg_color(buffer, col, row, color)` — foreground color check
- `assert_bg_color(buffer, col, row, color)` — background color check
- `assert_text_fg_in_row(buffer, row, text, color)` — span color check
- `make_session(id, status, working_dir)` — session factory
- `make_inactive_session(id, age_secs)` — inactive session factory
- `render_session_list_to_buffer(sessions, selected, width, height)` — render
  helper
- `render_dashboard_to_buffer(app, width, height)` — full dashboard render
  helper

### TestBackend API (ratatui 0.29)

```rust
let backend = TestBackend::new(80, 24);
let mut terminal = Terminal::new(backend).expect("test terminal");
terminal.draw(|frame| { /* render */ }).expect("draw");
let buffer = terminal.backend().buffer();
let cell = buffer.cell((col, row)); // Option<&Cell>
// cell.symbol(), cell.fg, cell.bg, cell.modifier
```

### Gotchas

- `Session.since = Instant::now()` — elapsed always "0s" in tests
- `highlight_symbol("▸ ")` adds 2-char offset on selected rows
- Block borders shift content by 1 column
- `assert_buffer_lines` checks text only, not styles

## Test Scenarios

### Column Layout (TDD — should fail before fix)

Tests targeting acd-0uz, acd-7dl, acd-k69, acd-czj, acd-csg:

```text
test_directory_is_first_data_column_standard
test_directory_is_first_data_column_wide
test_header_directory_is_first_column_standard
test_header_does_not_say_name
test_header_says_session_id_standard
test_header_says_session_id_wide
test_session_id_not_truncated_in_line
test_session_id_not_truncated_at_any_width
test_elapsed_column_fits_hours_format
test_elapsed_column_width_at_least_10
test_header_labels_left_aligned_standard
test_header_labels_left_aligned_wide
test_data_columns_left_aligned_standard
```

### Buffer Content (verify existing behavior)

```text
test_dashboard_buffer_contains_header_text       — row 0 has "Agent Console Dashboard"
test_dashboard_buffer_contains_footer_keybindings — last row has "[q] Quit"
test_dashboard_buffer_contains_session_border     — "Sessions" title in border
test_dashboard_buffer_shows_session_names         — session IDs appear in buffer
test_dashboard_empty_renders_without_session_text — no sessions → no data rows
test_dashboard_selected_session_has_highlight     — bg: DarkGray on selected
test_dashboard_selected_session_has_arrow_symbol  — highlight symbol on selected
test_narrow_mode_shows_only_symbol_and_name       — width < 40
```

### Status Colors

```text
test_working_status_renders_green_symbol    — fg: Green, "●"
test_attention_status_renders_yellow_symbol — fg: Yellow, "○"
test_question_status_renders_blue_symbol   — fg: Blue, "?"
test_closed_status_renders_gray_symbol     — fg: Gray, "x"
test_inactive_session_renders_dark_gray    — fg: DarkGray, Modifier::DIM
test_error_working_dir_renders_red         — "<error>" in Color::Red
```

### Responsive Layout

```text
test_standard_mode_shows_all_columns         — width 40-80, all 5 columns
test_wide_mode_shows_wider_directory         — width > 80, dir 30 chars
test_header_row_is_cyan_bold                 — header style check
test_header_row_absent_in_narrow_mode        — width < 40, no headers
```

### Detail Panel (TDD — acd-bbh)

```text
test_detail_renders_below_session_list_not_centered — bottom panel, not modal
test_detail_section_shows_session_status            — "Status:" + status text
test_detail_section_shows_working_directory          — "Dir:" + path
test_detail_section_shows_session_id                 — "ID:" + id
test_detail_shows_action_hints                       — "[ESC] Back", "[C]lose"
test_detail_closed_session_shows_resurrect           — "[R]esurrect" in yellow
test_detail_unknown_dir_shows_error_not_unknown      — "<error>" not "unknown" (acd-4sq)
test_detail_normal_dir_shows_path                    — valid path displayed
test_detail_no_history_shows_placeholder             — "(no transitions)"
test_detail_history_shows_transitions                — transition arrows
test_detail_history_scroll_shows_entry_count         — "[X/Y entries]"
```

### Mouse (mostly covered by existing unit tests)

```text
test_click_selects_and_renders_highlight — click + re-render, verify highlight
```

### Full Dashboard Integration

```text
test_full_dashboard_render_with_mixed_statuses     — 4 sessions, all statuses
test_full_dashboard_render_with_detail_panel        — sessions + detail visible
test_full_dashboard_render_many_sessions_scrolling  — 50 sessions, scroll to #25
```

## File Organization

```text
src/tui/test_utils.rs           — shared helpers (#[cfg(test)])
src/tui/views/dashboard.rs      — column layout + rendering tests (mod tests)
src/tui/views/detail.rs         — detail panel tests (mod tests)
src/tui/ui.rs                   — full dashboard layout tests (mod tests)
src/tui/app.rs                  — mouse interaction tests (existing + new)
```

## Test Count

```csv
Category,Count,TDD
Column layout,13,yes
Buffer content,8,no
Status colors,6,no
Responsive layout,4,no
Detail panel,11,yes
Mouse,1,partial
Full dashboard,3,partial
Total,46,24 TDD / 22 existing
```

## Source Agents

Research performed by two subagents (resumable for follow-up):

- Test Expert (ae91266): identified all 46 test scenarios
- UI Expert (abbc188): researched ratatui TestBackend API and patterns
