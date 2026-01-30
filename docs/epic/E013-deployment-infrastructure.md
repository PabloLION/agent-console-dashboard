# Epic: Deployment Infrastructure

**Epic ID:** E013 **Status:** Draft **Priority:** Low **Estimated Effort:** S

## Summary

Provide deployment tooling for running the daemon as a managed system service on
macOS (launchd) and Linux (systemd). This ensures the daemon starts
automatically on login, restarts on crash, and integrates with OS-level service
management.

## Goals

- Provide launchd plist for macOS daemon management
- Provide systemd unit file for Linux daemon management
- Create install/uninstall commands for service registration
- Document manual service setup for users who prefer control

## User Value

Users who rely on the dashboard daily benefit from the daemon starting
automatically on login and recovering from crashes without manual intervention.
This transforms the daemon from a developer tool requiring manual startup into a
reliable background service. Per
[Q24 decision](../plans/7-decisions.md#q24-daemon-crash-recovery), auto-restart
was deferred from v0 — this epic delivers that capability.

## Stories

| Story ID                                                   | Title                           | Priority | Status |
| ---------------------------------------------------------- | ------------------------------- | -------- | ------ |
| [S013.01](../stories/S013.01-macos-launchd-plist.md)       | Create macOS launchd plist      | P1       | Draft  |
| [S013.02](../stories/S013.02-linux-systemd-unit-file.md)   | Create Linux systemd unit file  | P1       | Draft  |
| [S013.03](../stories/S013.03-install-uninstall-cli.md)     | Implement install/uninstall CLI | P2       | Draft  |
| [S013.04](../stories/S013.04-manual-service-setup-docs.md) | Document manual service setup   | P3       | Draft  |

## Dependencies

- [E001 - Daemon Core Infrastructure](./E001-daemon-core-infrastructure.md) -
  Daemon binary must exist
- [E012 - Logging and Diagnostics](./E012-logging-and-diagnostics.md) - Logging
  to file needed for background service operation

## Acceptance Criteria

- [ ] `acd service install` registers the daemon as a user-level service
- [ ] `acd service uninstall` removes the service registration
- [ ] Daemon auto-starts on user login (macOS and Linux)
- [ ] Daemon auto-restarts on crash with backoff delay
- [ ] Service files use correct paths for binary, socket, and log file
- [ ] Manual setup documented for users without install command
- [ ] Manual test plan for install/uninstall on both platforms per
      [testing strategy](../decisions/testing-strategy.md)

## Technical Notes

### macOS: launchd

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.agent-console.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/acd</string>
        <string>daemon</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardErrorPath</key>
    <string>/tmp/acd.log</string>
</dict>
</plist>
```

Install location: `~/Library/LaunchAgents/com.agent-console.daemon.plist`

### Linux: systemd

```ini
[Unit]
Description=Agent Console Dashboard Daemon
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/acd daemon
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
```

Install location: `~/.config/systemd/user/acd.service`

### CLI Commands

```bash
# Install as user service
acd service install

# Uninstall
acd service uninstall

# Check service status (delegates to launchctl/systemctl)
acd service status
```

### Platform Detection

```rust
#[cfg(target_os = "macos")]
fn install_service() { /* launchd */ }

#[cfg(target_os = "linux")]
fn install_service() { /* systemd */ }
```

### Interaction with Auto-Start (Q2)

The auto-start mechanism (E001 S001.04) and service management are
complementary:

- **Without service:** Auto-start on first hook/dashboard connection
- **With service:** Daemon always running, auto-start becomes a no-op

Users can choose either approach. Service management is optional.

## Out of Scope

- Windows service management — deferred with Windows support (Q23)
- Docker/container deployment
- Multi-user daemon (root-level service)
