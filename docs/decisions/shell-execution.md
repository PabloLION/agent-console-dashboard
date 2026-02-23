# Shell Execution of User Strings

When executing user-configured commands (EDITOR, hooks, etc.):

**Wrong**: `Command::new(&user_string)` -- `execve` treats the entire string as
a binary path. No word splitting. `EDITOR="code --wait"` fails.

**Right**:

```rust
Command::new("sh")
    .arg("-c")
    .arg(format!("{} \"$1\"", &editor))
    .arg("--")         // $0
    .arg(&file_path)   // $1
```

This lets the shell handle word splitting, same as git/crontab/visudo.

Applies to any user-configured command: EDITOR, VISUAL, hooks, custom scripts.

## Environment Variables Over String Substitution

When passing data to user-configured shell commands:

**Wrong**: String substitution `{working_dir}` -- shell injection risk if value
contains metacharacters (spaces, quotes, `$()`).

**Right**: Set environment variables on the child process and let users
reference them in their commands:

```rust
Command::new("sh")
    .arg("-c")
    .arg(&hook_cmd)
    .env("ACD_SESSION_ID", &session_id)
    .env("ACD_WORKING_DIR", &working_dir)
    .env("ACD_STATUS", &status)
```

Shell handles quoting naturally with `"$ACD_WORKING_DIR"`.

## References

- Issue: acd-ohr6 (shell execution), acd-ynba (env vars)
