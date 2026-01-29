# Epic: Zellij Integration

**Epic ID:** E010 **Status:** Draft **Priority:** Medium **Estimated Effort:** M

## Summary

Implement integration with Zellij terminal multiplexer to enable seamless
dashboard display and session resurrection within Zellij layouts. This epic
provides the tooling and scripts needed to embed the Agent Console Dashboard in
Zellij panes and execute session resurrection commands in the appropriate Zellij
context.

## Goals

- Enable dashboard display in dedicated Zellij panes via layout scripts
- Provide Zellij CLI integration for session resurrection (creating panes,
  sending commands)
- Support automatic dashboard startup within Zellij layouts
- Lay groundwork for future native Zellij plugin integration

## User Value

Users working in Zellij benefit from a persistent, always-visible dashboard pane
that shows Claude Code session status at a glance. When resurrecting sessions,
the integration can automatically create new panes in the appropriate location,
eliminating manual terminal management. This tight integration makes multi-agent
workflows feel native to the Zellij environment and reduces context switching
friction.

## Priority Rationale

Upgraded from Low to Medium. Zellij is the primary terminal multiplexer for this
project's target audience. Basic layout integration (S10.1) provides significant
value by making the dashboard a natural part of the Zellij workflow. Native
plugin (WASM) remains deferred to v2+ per
[Q8 decision](../plans/7-decisions.md#q8-zellij-plugin).

## Stories

| Story ID                                                  | Title                                    | Priority | Status |
| --------------------------------------------------------- | ---------------------------------------- | -------- | ------ |
| [S10.1](../stories/S10.1-zellij-layout-dashboard.md)      | Create zellij layout with dashboard pane | P1       | Draft  |
| [S10.2](../stories/S10.2-zellij-resurrection-workflow.md) | Document Zellij resurrection workflow    | P2       | Draft  |

## Dependencies

- [E001 - Daemon Core Infrastructure](./E001-daemon-core-infrastructure.md) -
  Daemon must be running for dashboard to display data
- [E004 - TUI Dashboard](./E004-tui-dashboard.md) - Dashboard TUI must exist to
  display in Zellij pane
- [E007 - Configuration System](./E007-configuration-system.md) - Zellij
  integration settings stored in config

## Acceptance Criteria

- [ ] Dashboard can run in a dedicated Zellij pane via layout script
- [ ] Layout script supports various pane sizes (1-3 lines)
- [ ] Session resurrection can create new Zellij pane programmatically
- [ ] Commands can be sent to existing Zellij panes for resurrection
- [ ] Integration works without Zellij (graceful fallback to current terminal)
- [ ] Configuration allows enabling/disabling Zellij integration
- [ ] Manual test plan for Zellij and non-Zellij environments per
      [testing strategy](../decisions/testing-strategy.md)

## Technical Notes

### Dashboard Pane Setup

The `zellij-claude-layout` script starts the dashboard in a dedicated pane:

```bash
# In layout script
zellij run -- agent-console tui --layout one-line
```

Recommended pane configuration:

- Small pane at top or bottom of layout
- 1-3 lines height depending on layout choice
- Auto-started with Zellij layout

### Session Resurrection in Zellij

When resurrecting a session within Zellij context:

1. Determine which Zellij pane/tab to use (or create new)
2. Run `claude --resume <session-id>` in that pane
3. Update dashboard to show session as Working

### Zellij CLI Commands

Potential approaches for pane management:

| Command                                   | Description                               |
| ----------------------------------------- | ----------------------------------------- |
| `zellij action new-pane`                  | Create a new pane for resurrected session |
| `zellij action write <pane-id> "command"` | Send command to existing pane             |
| `zellij action focus-pane`                | Focus a specific pane                     |

### Configuration

Zellij integration settings in config file:

```toml
[integrations.zellij]
enabled = true
# auto_pane = true  # Future: auto-create resurrection panes
```

### Zellij Detection

Before using Zellij commands, detect if running within Zellij:

```bash
if [ -n "$ZELLIJ" ]; then
    # Running inside Zellij, use zellij commands
else
    # Fallback to current terminal
fi
```

### Future: Native Zellij Plugin

A native Zellij plugin (written in WASM) could provide deeper integration:

- Automatic session detection without hooks
- Direct pane status indicators
- Resurrection without CLI command wrappers
- Built-in status bar widget

**Status:** Not planned for initial release. Evaluate after core features work.

### Testing Strategy

Manual testing workflow:

1. Start Zellij with layout that includes dashboard pane
2. Verify dashboard displays correctly at various sizes
3. Test pane creation for session resurrection
4. Verify command execution in Zellij panes
5. Test fallback behavior outside Zellij environment
