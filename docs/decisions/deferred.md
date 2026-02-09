# Decision: Deferred Work

**Status:** Deferred

Decisions about what the project chose NOT to do yet and why. These are
intentional deferrals, not forgotten items.

## Zellij Plugin (Q8)

**Deferred to:** v2+

A native Zellij plugin (WASM) would provide deeper integration without hooks.
Deferred because WASM adds complexity, creates maintenance burden, and is
Zellij-specific (not portable). The hook-based approach works for v0/v1.

## Tmux Plugin (Q9)

**Deferred to:** on request only

Same considerations as Zellij plugin but lower priority (user preference). Will
not implement unless users request it. Hook-based approach works without
plugins.

## Windows Support (Q23)

**Deferred to:** v2+

Windows would use Named Pipes (`\\.\pipe\acd`) instead of Unix sockets. Deferred
because v0/v1 scope is already large, and Windows adds platform-specific code
paths. Named Pipes chosen over TCP localhost because they are the Windows-native
equivalent with no network overhead.

| Platform | IPC Mechanism | Location                    |
| -------- | ------------- | --------------------------- |
| Linux    | Unix socket   | `$XDG_RUNTIME_DIR/acd.sock` |
| macOS    | Unix socket   | `$TMPDIR/acd.sock`          |
| Windows  | Named Pipe    | `\\.\pipe\acd`              |

## Man Pages (Q46)

**Deferred to:** v1+

Rely on `--help` for v0. Consider `clap_mangen` for man page generation later.

## Sound/Notification on Status Change (Q105)

**Deferred to:** v1+

Desktop notifications and sounds on status changes (e.g., session needs
attention). Deferred to focus on core functionality first.

## Parking Lot

Items deferred indefinitely:

- Multi-user support (probably never needed)
- Remote access (probably never needed)
- Plugin system for custom widgets
- Integration with other AI agents (after v1)

---

[All questions](../archive/planning/6-open-questions.md) |
[Original analysis](../archive/planning/7-decisions.md)
