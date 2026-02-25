# Activation Gestures

**Decided:** 2026-02-24 **Status:** Implemented

## Decision

Enter key and double-click are equivalent activation gestures. Both fire the
configured hook for the selected session:

- Non-closed session → `tui.activate_hooks`
- Closed session → `tui.reopen_hooks`

Enter does NOT open or expand a detail view. The detail panel is always visible
(see `detail-panel-layout.md`); Enter is reserved for hook activation.

Deselection gestures:

- **Esc**: clears selection (`selected_index = None`)
- **Header click**: clears selection

Scroll wheel navigates the session list regardless of cursor position. It never
routes to the detail panel, even when the detail panel is focused.

## Rationale

Unifying Enter and double-click into a single activation model keeps the
interaction consistent. Users who prefer keyboard and users who prefer mouse
reach the same outcome with the natural gesture for each input mode.

Reserving Enter for hook activation (not detail expansion) follows the pattern
of most terminal list UIs: Enter = "act on this item", not "expand this item".
The detail panel is already visible, so there is nothing to expand.

Routing scroll exclusively to session navigation avoids confusion about which
pane receives scroll events. The session list is the primary navigation target;
the detail panel does not need independent scrolling in the current design.

## Alternatives Considered

- **Enter expands detail, separate key for hook**: Requires two keys for a
  common action. Rejected because the detail panel is always visible and needs
  no explicit open gesture.
- **Scroll routes to detail panel when focused**: Allows scrolling long history.
  Deferred — the detail panel currently fits its content without scrolling.
- **Esc closes the TUI**: Standard terminal convention, but would surprise users
  who use Esc to deselect. Deselection is the less destructive action and
  matches most TUI list conventions.
