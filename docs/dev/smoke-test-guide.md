# Smoke Test Guide

Template for generating release-specific smoke test checklists. Each release,
generate a concrete checklist from this template covering recent changes and
regression-prone areas.

## Pre-Test Setup

### Build

- Build from source: `cargo build -p agent-console-dashboard`
- Verify version: `acd --version` (should match Cargo.toml)

### Clean State

- Uninstall existing hooks: `acd uninstall`
- Stop daemon if running: `acd daemon stop`
- Optionally remove config: back up and delete
  `$XDG_CONFIG_HOME/agent-console-dashboard/`

## Test Categories

### Install and Hooks

- `acd install` registers all expected hooks (count and event types)
- `acd uninstall` removes all hooks cleanly
- Reinstall after uninstall works correctly
- Verify hook list matches `acd_hook_definitions()` in main.rs

### Daemon Lifecycle

- `acd daemon start` starts the daemon (check process exists)
- `acd daemon stop` stops it cleanly
- Daemon auto-starts via hooks (start a Claude Code session without manual
  daemon start)
- Idle timeout shuts down daemon after configured period

### TUI Layout

- `acd tui` launches without error
- Header shows "Agent Console Dashboard" (no version in header)
- Footer left: keybinding hints
  `[j/k] Navigate  [Enter] Details  [r] Resurrect  [q] Quit`
- Footer right: version string `vX.Y.Z` in gray
- Column headers: Directory, Session ID, Status, Time Elapsed (all left-aligned)
- Highlight marker: `▶` (filled triangle) with consistent spacing for all rows
- `q` exits cleanly

### Session Tracking

- Start a Claude Code session in a project directory
- TUI shows new session with correct directory basename, session ID, and status
- Status transitions: working (green) → attention (yellow) when waiting for
  input
- Elapsed time counts up correctly
- Session closure shows "closed" status (gray)

### Navigation and Interaction

- `j`/`k` moves selection up/down with `▶` marker
- `Enter` opens inline detail panel below session list
- Detail panel shows: Status, Dir, ID, and status history
- `Esc` or clicking header closes detail panel / clears selection
- Scroll wheel navigates list (dashboard) or history (detail)

### Double-Click Behavior

- Double-click with NO hook configured: yellow message appears in footer for ~3
  seconds
- Double-click with hook configured: "Hook executed" message appears for ~2
  seconds
- Single click opens detail panel (not double-click behavior)

### Configuration

- `acd config init` creates default config file
- `acd config init` with existing file returns error
- `acd config init --force` backs up and recreates
- `acd config show` displays current configuration

### Debug Mode

- `AGENT_CONSOLE_DASHBOARD_DEBUG=1 acd tui` shows ruler row below column headers
- Ruler shows column width labels (dir:XX, id:40, stat:14, time:16)
- Normal mode (no env var) has no ruler

### Responsive Layout

- Narrow terminal (<40 cols): simplified layout, no column headers
- Standard terminal (40-80 cols): full column layout
- Wide terminal (>80 cols): wider directory column

## Regression Checklist

Areas that have historically broken. Always test these regardless of changes:

- Elapsed time resets correctly on status change
- Inactive sessions show "◌" symbol and "inactive" status
- Multiple sessions with same directory basename disambiguate with parent
- Config file parse errors show line and column
- Status message auto-expires (doesn't persist after timeout)

## Generating a Release Checklist

For each release:

1. Start with all items from Test Categories above
2. Add specific items for each feature/fix in the release
3. Add regression items for any area that was modified
4. Mark items as PASS/FAIL during testing
5. Record any issues found as beads issues
