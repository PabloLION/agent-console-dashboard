# Manual service setup

This guide covers installing the Agent Console Dashboard daemon as a system
service on macOS and Linux. For automated setup, use `acd service install`.

## Prerequisites

- The `acd` binary must be installed at `/usr/local/bin/acd`
- XDG state directory must exist for logs

### Installing the binary

#### cargo install (recommended)

```sh
cargo install --path crates/agent-console-dashboard
```

Then copy to the expected location:

```sh
sudo cp ~/.cargo/bin/acd /usr/local/bin/acd
```

#### Manual build

```sh
cargo build --release -p agent-console
sudo cp target/release/acd /usr/local/bin/acd
```

### Creating the log directory

macOS:

```sh
mkdir -p ~/.local/state/agent-console
```

Linux (XDG default):

```sh
mkdir -p "${XDG_STATE_HOME:-$HOME/.local/state}/agent-console"
```

## macOS (launchd)

### 1. Copy the plist

```sh
cp resources/com.agent-console.daemon.plist ~/Library/LaunchAgents/
```

### 2. Replace the USERNAME placeholder

```sh
sed -i '' "s/USERNAME/$(whoami)/g" \
  ~/Library/LaunchAgents/com.agent-console.daemon.plist
```

### 3. Load the service

```sh
launchctl load ~/Library/LaunchAgents/com.agent-console.daemon.plist
```

### 4. Verify

```sh
launchctl list | grep com.agent-console
```

A line with the label `com.agent-console.daemon` confirms the service is loaded.

### Unloading

```sh
launchctl bootout gui/$(id -u)/com.agent-console.daemon
rm ~/Library/LaunchAgents/com.agent-console.daemon.plist
```

## Linux (systemd)

### 1. Copy the unit file

```sh
mkdir -p ~/.config/systemd/user
cp resources/acd.service ~/.config/systemd/user/
```

### 2. Reload and enable

```sh
systemctl --user daemon-reload
systemctl --user enable acd.service
```

### 3. Start the service

```sh
systemctl --user start acd.service
```

### 4. Verify

```sh
systemctl --user is-enabled acd.service
systemctl --user status acd.service
```

### Disabling

```sh
systemctl --user stop acd
systemctl --user disable acd
rm ~/.config/systemd/user/acd.service
systemctl --user daemon-reload
```

## Troubleshooting

### Permission denied

Ensure the binary is executable:

```sh
chmod +x /usr/local/bin/acd
```

### Binary not found

The service expects `acd` at `/usr/local/bin/acd`. Verify:

```sh
ls -la /usr/local/bin/acd
/usr/local/bin/acd --version
```

### Service will not start

Check logs:

- **macOS**: `cat ~/.local/state/agent-console/acd.log`
- **Linux**: `journalctl --user -u acd.service`

Ensure no other instance is already running and the socket file is not stale:

```sh
rm -f /tmp/agent-console.sock
```

### Logs are empty or missing

Verify the log directory exists (see "Creating the log directory" above). On
macOS the plist `StandardErrorPath` must point to a writable location.

## Automated alternative

The CLI provides an automated installation command that performs all of the
above steps:

```sh
acd service install    # install and enable
acd service status     # check service state
acd service uninstall  # remove service
```
