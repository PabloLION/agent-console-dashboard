# Stdin, Pipes, and File Descriptors

Learning document explaining how Claude Code passes data to hook commands, and
why ACD uses Rust's `serde_json` instead of shell scripts with `jq`.

## File Descriptors

Every Unix process starts with three open file descriptors:

| fd | Name   | Default target | Purpose            |
| -- | ------ | -------------- | ------------------ |
| 0  | stdin  | Terminal input  | Read input data    |
| 1  | stdout | Terminal output | Write normal output |
| 2  | stderr | Terminal output | Write error output  |

These are inherited from the parent process. When Claude Code spawns a hook
command, it connects fd 0 (stdin) to a pipe carrying JSON data.

## Pipes

A pipe connects the stdout of one process to the stdin of another:

```text
Claude Code                    Hook command (acd claude-hook)
┌─────────────┐               ┌─────────────────────────┐
│ writes JSON  │──── pipe ────│ reads JSON from fd 0     │
│ to pipe      │   (fd 0)     │ parses with serde_json   │
└─────────────┘               │ writes response to fd 1  │
                              └─────────────────────────┘
```

The hook command reads all of stdin until EOF, parses the JSON, and writes its
response to stdout. Claude Code reads the hook's stdout for the response.

## `set -euo pipefail` (Shell Scripts)

The old shell hook scripts used `set -euo pipefail`:

| Flag        | Effect                                              |
| ----------- | --------------------------------------------------- |
| `-e`        | Exit on any command failure (non-zero exit code)     |
| `-u`        | Treat unset variables as errors                      |
| `-o pipefail` | Pipeline fails if any command in the pipe fails   |

This was necessary because shell scripts are fragile — a typo in a variable
name or a missing `jq` binary could silently succeed. Rust doesn't need these
guards because the compiler catches these errors at build time.

## Shell vs Rust: Reading Hook Data

### Shell (old approach)

```sh
INPUT=$(cat)                                        # Read all of stdin
SESSION_ID=$(echo "$INPUT" | jq -r '.session_id')   # Parse with jq
acd set "$SESSION_ID" attention || true              # Forward to daemon
```

Dependencies: bash, jq, cat, echo.

Failure modes: `jq` not installed, malformed JSON silently produces empty
string, variable expansion bugs.

### Rust (current approach)

```rust
let input: HookInput = serde_json::from_reader(std::io::stdin())?;
// input.session_id is now a String, type-checked at compile time
```

Dependencies: none (serde_json is compiled into the binary).

Failure modes: compile-time type errors, explicit `Result` handling at runtime.

## How Claude Code Passes Data to Hooks

Claude Code serializes a JSON object and writes it to the hook command's stdin.
The exact fields depend on the hook event:

```json
{
  "session_id": "abc-123",
  "cwd": "/home/user/project",
  "transcript_path": "/home/user/.claude/sessions/abc-123.jsonl",
  "permission_mode": "default",
  "hook_event_name": "Stop"
}
```

ACD's `HookInput` struct only declares `session_id`. Serde's default behavior
ignores unknown fields, so future Claude Code versions can add fields without
breaking ACD. This is deliberate forward-compatibility — we parse only what we
need.

## Exit Codes

Claude Code interprets the hook command's exit code:

| Code | Meaning        | Claude Code behavior                    |
| ---- | -------------- | --------------------------------------- |
| 0    | Success        | Continues normally, reads stdout JSON   |
| 2    | Blocking error | Shows error to user, may retry          |

ACD uses exit 0 even when the daemon is unreachable, to avoid blocking Claude
Code. The `systemMessage` field in the stdout JSON surfaces the condition
without interrupting the user's workflow.
