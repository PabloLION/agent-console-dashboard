# XDG Directory Selection

Use the correct XDG directory for each data type:

| Data type         | XDG variable       | Default            | Example          |
| ----------------- | ------------------ | ------------------ | ---------------- |
| Config            | XDG_CONFIG_HOME    | ~/.config          | config.toml      |
| Persistent data   | XDG_DATA_HOME      | ~/.local/share     | session database |
| **Logs, history** | **XDG_STATE_HOME** | **~/.local/state** | daemon.log       |
| Cache (deletable) | XDG_CACHE_HOME     | ~/.cache           | compiled assets  |

XDG_STATE_HOME (spec v0.8, May 2021) is specifically for volatile state that
persists across restarts but isn't worth backing up.

## macOS Caveat

`dirs::state_dir()` returns `None` on macOS (no macOS equivalent). Use fallback
chain: `state_dir().or_else(data_dir)`.

On macOS, this means logs end up in `~/Library/Application Support/` (the
data_dir fallback).

## References

- [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir-spec/latest/)
- Issue: acd-hzub
