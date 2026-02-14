# Manual Test Results — v0.1.2

Tested: 2026-02-12

## Passed

- Version `v0.1.2` in bottom-right footer (gray)
- Header: plain "Agent Console Dashboard" (no version)
- Footer left: keybinding hints
- Column headers: left-aligned
- Cell content: left-aligned, trailing padding
- Highlight marker: `▶` with consistent spacing
- Debug ruler: shows with `AGENT_CONSOLE_DASHBOARD_DEBUG=1`, hidden without
- Session tracking: new sessions appear, status transitions work
- Double-click: no-hook feedback (yellow message ~3s), single-click opens detail
- Detail panel: show/hide works, displays status history
- Hooks: 7 installed (no PostToolUse per acd-ws6)
- Config show: displays current configuration
- Session closure: "closed" status shown correctly
- `q` exits cleanly

## Issues Found

- **acd-87o** (P2): Highlighted inactive session text unreadable on selection
- **acd-88r** (P3): Closed sessions don't get inactive visual style
- **acd-0ab** (P2): Session resume may not update TUI (needs investigation)
- **acd-bbm** (P3): Single click sometimes doesn't register
- **acd-4nd** (P3): Scroll doesn't show detail panel of focused session
- **acd-0hd** (P3): Narrow terminal layout not observed
- **acd-qga** (P2): Config init --force gives no feedback about backup path
- Yellow color too dim (deferred to V1 theming, acd-5sk)
- Green color not vivid enough (deferred to V1 theming, acd-5sk)

## Not Tested

- Env var rename (`AGENT_CONSOLE_DASHBOARD_LOG` vs old `ACD_LOG`)
- Multiple sessions with same directory basename (deferred to E2E test acd-0ci)
- Scroll wheel behavior (partially observed)
