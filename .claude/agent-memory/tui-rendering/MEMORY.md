# TUI Rendering Agent Memory

## Focus Interaction Model (acd-zpg, acd-bbm, acd-4nd)

**Detail panel is always visible** — it occupies a fixed 12-line section below
the session list. When no session is focused, it shows hint text with keybinding
guidance. When a session is focused, it shows session details.

### Key Behaviors

1. **Always-visible panel** — layout always allocates 4 sections: header (1),
   session list (min 3, flex), detail (12), footer (1)
2. **Focus changes update detail** — any action that changes `selected_index`
   updates the detail panel content
3. **Enter fires hook** — Enter key executes the double-click hook (same
   behavior as double-click), not OpenDetail
4. **Scroll never steals focus** — mouse scroll wheel always navigates sessions,
   never scrolls history in detail panel
5. **Esc clears selection** — pressing Esc or clicking header sets
   `selected_index = None` (defocus)

### State Management

- `App.view` is now deprecated (always `View::Dashboard`)
- `App.history_scroll` tracks detail panel scroll offset (separate field, not in
  View enum)
- `App.selected_index` determines which session's detail to show
- History scroll resets to 0 when selection changes (j/k, click, scroll wheel)

### Rendering Logic

- `render_dashboard` always calls either `render_inline_detail` (when session
  focused) or `render_detail_placeholder` (when no selection)
- `render_detail_placeholder` shows hint text with keybindings: "[j/k] Navigate
  [Enter] Hook [q] Quit"
- Footer text updated: "[j/k] Navigate [Enter] Hook [r] Resurrect [q] Quit"

### Test Updates

Tests updated to reflect new behavior:

- `open_detail()` is now a no-op (detail always visible)
- `close_detail()` clears selection and resets history scroll
- Enter key tests check for hook execution, not OpenDetail action
- Mouse scroll tests verify session navigation, not history scrolling
- Esc tests verify selection clearing

## Mouse Click Detection (acd-khj)

**Rect-based approach** replaces hardcoded offsets. Session list inner area
(excluding block borders) is captured during render and stored in
`App.session_list_inner_area`.

Key changes:
- `render_session_list()` returns inner Rect (calculated via `block.inner()`)
- `render_dashboard()` takes `&mut App` to store the inner area
- `calculate_clicked_session()` uses stored Rect for click detection
- Works correctly in all modes: normal, debug (with ruler), narrow (no header)
- Tests must call `render_dashboard_to_buffer()` before testing clicks to
  populate `session_list_inner_area`

Layout structure (normal mode, 80x24):
- Row 0: main header "Agent Console Dashboard"
- Row 1: session list area starts (column header)
- Row 2: top border of List block
- Row 3+: sessions (inner area starts here)

## File Structure

Key files for detail panel behavior:

- `src/tui/ui.rs` — main render orchestration, always-visible layout
- `src/tui/app/mod.rs` — App struct with `history_scroll` and
  `session_list_inner_area` fields
- `src/tui/views/dashboard/mod.rs` — session list rendering, returns inner Rect
- `src/tui/views/detail/mod.rs` — detail panel rendering, placeholder with hints
- `src/tui/event/mod.rs` — Enter key fires hook, Esc clears selection
- `src/tui/app/tests/basic.rs` — App state management tests
- `src/tui/app/tests/interaction.rs` — mouse interaction tests
- `src/tui/event/tests.rs` — keyboard event tests
