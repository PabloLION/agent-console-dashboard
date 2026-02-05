# claude-hooks

Programmatic management of [Claude Code](https://claude.ai/code) hooks.

## Features

- **install** — Add hooks to `~/.claude/settings.json` with atomic writes
- **uninstall** — Remove only hooks installed by this crate (ownership tracking)
- **list** — Show all hooks with managed/unmanaged status

## Usage

```rust
use claude_hooks::{HookEvent, HookHandler, install, uninstall, list};

// Install a hook
let handler = HookHandler {
    r#type: "command".to_string(),
    command: "/path/to/hook.sh $SESSION_ID".to_string(),
    matcher: String::new(),
    timeout: Some(600),
    r#async: None,
};
install(HookEvent::Stop, handler, "my-app")?;

// List all hooks
for entry in list()? {
    println!("{:?}: {} (managed: {})",
        entry.event,
        entry.handler.command,
        entry.managed
    );
}

// Uninstall (only works for hooks we installed)
uninstall(HookEvent::Stop, "/path/to/hook.sh $SESSION_ID")?;
```

## Design

- **Atomic writes**: Uses temp-file-then-rename to prevent corruption
- **Ownership tracking**: Local registry in XDG data dir tracks which hooks we installed
- **Non-destructive**: Never modifies hooks installed by other tools or manually

## Hook Events

Supports all Claude Code hook events:

- `Start`, `Stop`
- `BeforePrompt`, `AfterPrompt`
- `BeforeToolUse`, `AfterToolUse`
- `BeforeEdit`, `AfterEdit`
- `BeforeRevert`, `AfterRevert`
- `BeforeRun`, `AfterRun`

## License

MIT OR Apache-2.0
