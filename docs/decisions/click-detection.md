# Click Detection: Rect-Based Hit Testing

Created: 20260214T220000Z Issue: acd-khj

## Problem

`calculate_clicked_session()` used a hardcoded offset of 2 to map mouse row
coordinates to session indices. This assumed sessions always start at row 2 (app
header + column header). It was wrong:

- **Normal mode**: sessions start at row 3 (+ block border). Off by 1.
- **Debug mode**: sessions start at row 4 (+ debug ruler). Off by 2.
- **Narrow mode** (width < 40): sessions start at row 2 (no column header).
  Accidentally correct.

Any future layout change (adding a status bar, removing the header) would
silently re-introduce the bug.

## Root Cause

The click handler duplicated layout knowledge with a magic number instead of
using the geometry computed during rendering.

## Solution

Store the session list's inner `Rect` on `App` during the render pass. The click
handler reads it instead of computing its own offset.

### Render side

`render_session_list()` returns the inner `Rect` (the area inside the List
widget's block borders, where session rows actually appear).
`render_dashboard()` stores it on `App.session_list_inner_area`.

### Click side

`calculate_clicked_session()` reads the stored Rect:

```rust
fn calculate_clicked_session(&self, row: u16) -> Option<usize> {
    let inner_area = self.session_list_inner_area?;
    if row < inner_area.y || row >= inner_area.y + inner_area.height {
        return None;
    }
    let list_row = (row - inner_area.y) as usize;
    if list_row < self.sessions.len() { Some(list_row) } else { None }
}
```

If no render has occurred yet (`None`), all clicks return `None`.

## Design Discussion

### Why `&mut App` in render is acceptable

Storing layout geometry during render means `render_dashboard` takes `&mut App`
instead of `&App`. This was debated:

- In retained-mode UI (HTML, React), render is pure — side effects are wrong.
- In immediate-mode UI (ratatui, Dear ImGui), render IS where layout is
  computed. Storing hit-test geometry during render is standard practice.

Since ratatui is immediate-mode, `&mut App` during render is idiomatic.

### Alternatives considered

1. **Compute offset dynamically** — a pure function derives the row offset from
   terminal dimensions and debug mode. Rejected: duplicates layout logic in a
   second place, which is exactly how the original bug happened.
2. **Event propagation** (HTML model) — click events route to sub-components,
   each handling its own area. Rejected: requires building a component system
   that ratatui doesn't have. Good long-term direction but premature today.
3. **Store Rect outside App** — a separate `RenderState` struct. Rejected: the
   data logically belongs to App (it IS the TUI state holder).

## Testing Lesson

The original agent fix called `render_dashboard_to_buffer()` in every click test
to populate the Rect. This was wrong:

- Rendering reads `AGENT_CONSOLE_DASHBOARD_DEBUG` env var to decide whether to
  show the debug ruler. A debug-mode test set this env var, leaking to parallel
  tests and causing flaky failures.
- Unit tests should test pure functions. Click detection is pure: given an inner
  area and sessions, map row to index.

Fix: set `session_list_inner_area` directly in tests via a
`make_clickable_app()` helper. No rendering, no env vars, no `#[serial]`.
Different layout modes (normal, debug, narrow) are tested by setting different
Rect offsets.

```rust
fn make_clickable_app(session_count: usize) -> App {
    let mut app = make_app_with_sessions(session_count);
    app.session_list_inner_area = Some(Rect::new(0, 3, 80, 20));
    app
}
```

## Files Changed

- `src/tui/app/mod.rs` — added `session_list_inner_area` field, rewrote
  `calculate_clicked_session()`
- `src/tui/views/dashboard/mod.rs` — `render_session_list()` returns inner Rect
- `src/tui/ui.rs` — `render_dashboard()` takes `&mut App`, stores Rect
- `src/tui/test_utils.rs` — updated `render_dashboard_to_buffer()` signature
- `src/tui/app/tests/interaction.rs` — rewrote as pure unit tests
