# Integrations

How Agent Console Dashboard connects to external systems.

---

## Claude Code Hooks

Primary integration method for Claude Code sessions.

### Hook Types

| Hook               | When Fired                | Dashboard Action      |
| ------------------ | ------------------------- | --------------------- |
| `Stop`             | Session stops/completes   | Set status: Attention |
| `Notification`     | Claude sends notification | Set status: Attention |
| `UserPromptSubmit` | User sends message        | Set status: Working   |
| `AskQuestion`      | Claude asks question (?)  | Set status: Question  |

### Hook Scripts

Location: `~/.claude/hooks/` (or configured path)

Example `stop.sh`:

```bash
#!/bin/bash
PROJECT=$(basename "$PWD")
agent-console set "$PROJECT" attention
```

Example `user-prompt-submit.sh`:

```bash
#!/bin/bash
PROJECT=$(basename "$PWD")
agent-console set "$PROJECT" working
```

### Hook Registration

In `~/.claude/settings.json`:

```json
{
  "hooks": {
    "Stop": ["~/.claude/hooks/stop.sh"],
    "UserPromptSubmit": ["~/.claude/hooks/user-prompt-submit.sh"],
    "Notification": ["~/.claude/hooks/notification.sh"]
  }
}
```

### AskQuestion Hook Issue

**Current Status:** Not working / not available

**Investigation needed:**

1. Check if Claude Code has an AskQuestion hook
2. If not, consider requesting feature from Anthropic
3. Alternative: Poll Claude Code state somehow?

---

## Zellij Integration

Primary terminal multiplexer integration.

### Current Usage

The `zellij-claude-layout` script starts the dashboard in a dedicated pane:

```bash
# In layout script
zellij run -- agent-console tui --layout one-line
```

### Dashboard Pane

Recommended setup:

- Small pane at top or bottom of layout
- 1-3 lines height depending on layout
- Auto-started with Zellij layout

### Session Resurrection in Zellij

When resurrecting a session, need to:

1. Determine which Zellij pane/tab to use (or create new)
2. Run `claude --resume <session-id>` in that pane
3. Update dashboard to show session as Working

**Potential approaches:**

- Use Zellij CLI to create pane: `zellij action new-pane`
- Send command to existing pane: `zellij action write <pane-id> "command"`
- Open in focused pane (simplest, user navigates first)

### Future: Zellij Plugin

A native Zellij plugin could provide deeper integration:

- Automatic session detection (no hooks needed)
- Direct pane status indicators
- Resurrection without CLI commands

**Status:** Not planned for initial release. Evaluate after core features work.

---

## Tmux Integration (Future)

Similar to Zellij but using tmux commands.

### Status Bar Integration

Tmux status bar could show dashboard output:

```bash
# In .tmux.conf
set -g status-right '#(agent-console list --format tmux)'
```

### Tmux Plugin (Future)

Native tmux plugin for deeper integration.

**Status:** Not planned for initial release.

---

## Other Multiplexers (Future)

Potential support for:

- Wezterm (native multiplexing)
- Screen
- Byobu

Architecture should be multiplexer-agnostic where possible.

---

## API Integration

### Claude Code API Metrics

If Claude Code exposes API usage, integrate it.

**Investigation needed:**

1. Does Claude Code report token usage via hooks?
2. Is there a file/socket we can read?
3. Can we parse Claude Code logs?

**Fallback:** Track metrics from hooks (count messages, estimate tokens).

### Future: Anthropic API Direct

If user provides API key, could query Anthropic API for usage stats.

**Considerations:**

- Security: Storing API keys
- Rate limits
- Scope: Might be overkill for this tool

---

## Configuration

All integrations configured in one file:

```toml
# ~/.config/agent-console/config.toml

[integrations.claude-code]
enabled = true
hooks_path = "~/.claude/hooks"

[integrations.zellij]
enabled = true
# auto_pane = true  # Future: auto-create resurrection panes

[integrations.tmux]
enabled = false

[integrations.api]
enabled = false
# api_key = "sk-..."  # If direct API access
```

---

## Integration Testing

### Manual Testing Steps

1. Start daemon: `agent-console daemon`
2. Start dashboard: `agent-console tui`
3. In another terminal, simulate hooks (see commands below)
4. Verify dashboard updates in real-time

```bash
agent-console set test-project working
agent-console set test-project attention
agent-console set test-project question
agent-console rm test-project
```

### With Real Claude Code

1. Configure hooks in `~/.claude/settings.json`
2. Start Claude Code session
3. Send messages, observe dashboard updates
4. Test Stop hook (let Claude finish)
5. Test resurrection (if implemented)
