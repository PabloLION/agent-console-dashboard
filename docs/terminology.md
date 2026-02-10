# Terminology

Project-specific terms used in code and documentation.

## Lazy-start

The daemon starts on demand when a hook or TUI first needs it, rather than
running persistently as a system service (launchd/systemd).

When a client needs the daemon:

1. Attempt socket connection
2. If not running, spawn `acd daemon --daemonize` in the background
3. Retry connection with exponential backoff (10ms initial, 500ms max, 10
   retries)
4. Socket binding acts as mutex — concurrent clients won't spawn duplicates

Implementation: `connect_with_lazy_start()` in
`crates/agent-console-dashboard/src/client/connection.rs`

## Wire format

The format data takes when transmitted between systems over IPC. In ACD, this is
the JSON Lines encoding of `SessionSnapshot`, `IpcCommand`, `IpcResponse`, and
`IpcNotification` structs sent over the Unix socket.

"Wire" comes from telecom/networking where data traveled over physical wires.
The term persists even for non-wire transports (sockets, pipes, memory).

See [IPC protocol decision](decisions/ipc-protocol.md) for the full wire format
specification.
