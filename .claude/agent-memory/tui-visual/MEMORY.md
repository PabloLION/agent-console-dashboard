# TUI Visual Agent Memory

## Column Layout System

### Fixed Width Calculation Pattern

When adding/removing columns, update 3 locations in dashboard/mod.rs:

1. `format_session_line()` — data row rendering + fixed_width calc
2. `format_header_line()` — header rendering + fixed_width calc
3. `format_ruler_line()` — debug ruler + fixed_width calc

Fixed width = highlight marker (2) + sum of all fixed-width columns. Directory
column is flex, calculated as `width - fixed_width`.

### Column Order (acd-3q0z)

Order follows sort precedence: Directory (visual identifier) → Status → Priority
→ Time Elapsed → Session ID. Rationale: sort-key columns appear in sort order
for easy scanning.

Current columns:

- Directory: flex width
- Status: 14 chars (max label "attention" 9 + padding)
- Priority: 12 chars (u64 display + padding)
- Time Elapsed: 16 chars (HH:MM:SS format + padding)
- Session ID: 40 chars (UUID 36 + padding)

Total fixed width: 2 + 14 + 12 + 16 + 40 = 84

## Test Strategy

Tests use `render_session_list_to_buffer()` helper from `tui::test_utils`. Tests
check positioning via `find_row_with_text()` and string position comparison.
Column reordering will break tests that assume old order (Status/Session ID
positions).

### Span Index Pattern (acd-3q0z)

After adding Priority column:

- Data line: 5 spans (directory[0], status[1], priority[2], elapsed[3],
  session_id[4])
- Header line: 6 spans (padding[0], directory[1], status[2], priority[3],
  elapsed[4], session_id[5])
- Ruler line: 6 spans (padding[0], dir_label[1], stat_label[2], prio_label[3],
  time_label[4], id_label[5])

### Session ID Column Constraints (acd-3q0z)

Session ID column is fixed at 40 chars (`format!("{:<40}", name)`). IDs longer
than 40 chars are truncated at format level, not buffer level.

- Real session IDs are UUID v4 (36 chars), which fit comfortably
- Test session IDs must be ≤40 chars to avoid format truncation
- Minimum buffer width for all columns: 85 (directory 1 + fixed 84)

## Documentation Sync

Always update `docs/design/ui.md` when changing column layout:

- "Column layout" section (detailed explanation)
- Decision table at bottom (column widths row)

## Header and Footer Layout (acd-mq6y)

Version display moved from footer bottom-right to header right-aligned.

Header rendering pattern (ui.rs):

- Calculate padding: `header_width - title_len - version_len`
- Three-span Line: title (Cyan) + padding (spaces) + version (DarkGray)
- Version uses `env!("CARGO_PKG_VERSION")` at compile time
- Footer bottom-right now reserved for API usage (acd-0i4i, future work)

Test pattern:

- `test_version_shown_in_header_row()` checks row 0
- `test_version_not_in_footer_row()` verifies absence in footer
- Use `row_contains(&buffer, row_index, text)` from test_utils
