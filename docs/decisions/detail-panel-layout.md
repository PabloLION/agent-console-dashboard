# Detail Panel Layout

**Decided:** 2026-02-24 **Status:** Implemented

## Decision

The detail panel is always visible as a 12-line fixed section below the session
list. It does not toggle. When no session is selected, it shows hint text
guiding the user to select a session.

## Rationale

Always-visible panel avoids layout jitter — toggling show/hide causes the
session list to resize every time the user selects or deselects a session.
Constant layout height makes the interface stable and predictable.

Hint text in the empty state serves new users who may not know how to activate
the panel.

## Alternatives Considered

- **Toggle on selection**: The panel appears when a session is selected and
  disappears when deselected. Rejected because it causes the session list to
  resize on every selection change, producing distracting layout shifts.
- **Always empty when nothing selected**: Show a blank area instead of hint
  text. Rejected because the blank space gives no guidance to new users.
