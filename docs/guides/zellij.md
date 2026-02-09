# Zellij Integration

Agent Console Dashboard integrates seamlessly with Zellij terminal multiplexer,
providing persistent session monitoring in a dedicated pane.

## Prerequisites

- **Zellij installed** (version 0.39.0+ recommended)
  - Installation guide: <https://zellij.dev/documentation/installation>
- **agent-console binary in PATH**
  - Build: `cargo build --release`
  - Install: `cargo install --path .`

## Quick Start

Use the provided launcher script to start Zellij with the dashboard:

```sh
# Default layout (2-line dashboard + vertical split)
./scripts/zellij/zellij-claude.sh

# Minimal layout (1-line dashboard)
./scripts/zellij/zellij-claude.sh minimal

# Detailed layout (multi-line dashboard)
./scripts/zellij/zellij-claude.sh detailed
```

The script automatically:

- Checks for Zellij installation
- Validates Zellij version (warns if < 0.39.0)
- Prevents nested sessions
- Launches the appropriate layout

## Layout Descriptions

Three pre-configured layouts are available:

| Layout       | Pane Height | Content                    | Use Case                      |
| ------------ | ----------- | -------------------------- | ----------------------------- |
| **minimal**  | 2 rows      | Session names + status     | Maximum screen space for work |
| **default**  | 3 rows      | Status + working directory | Balanced monitoring and space |
| **detailed** | 5 rows      | Full session details       | Deep session inspection       |

### Minimal Layout

Displays session count and basic status in a compact single-line format.

```kdl
layout {
    pane size=2 {
        command "agent-console"
        args "tui" "--layout" "one-line"
    }
    pane
}
```

### Default Layout

Two-line dashboard with vertical split for dual terminal panes.

```kdl
layout {
    pane size=3 {
        command "agent-console"
        args "tui" "--layout" "two-line"
    }
    pane split_direction="vertical" {
        pane
        pane
    }
}
```

### Detailed Layout

Multi-line dashboard showing full session information including timestamps,
directories, and detailed status.

```kdl
layout {
    pane size=5 {
        command "agent-console"
        args "tui"
    }
    pane
}
```

## Installation to Zellij Config

For persistent access, copy layouts to your Zellij configuration directory:

```sh
# Create layouts directory if it doesn't exist
mkdir -p ~/.config/zellij/layouts

# Copy all layouts
cp scripts/zellij/layouts/claude-*.kdl ~/.config/zellij/layouts/

# Launch directly via Zellij
zellij --layout claude-default
```

After installation, layouts are available system-wide:

```sh
zellij --layout claude-minimal
zellij --layout claude-default
zellij --layout claude-detailed
```

## Usage Patterns

### Launching from Outside Zellij

```sh
# Using launcher script (recommended)
./scripts/zellij/zellij-claude.sh default

# Using Zellij directly (if layouts installed)
zellij --layout claude-default
```

### Adding Dashboard to Existing Session

If you're already inside a Zellij session, the launcher script will exit with a
helpful message. To add the dashboard to your current session manually:

```sh
# Open Zellij command mode (Ctrl+o by default)
# Then run:
zellij action new-pane --command agent-console tui
```

### Switching Layouts Mid-Session

To change the dashboard layout while Zellij is running:

1. Close the current dashboard pane (Ctrl+p x)
2. Open a new pane with the desired layout command
3. Or restart Zellij with a different layout

## Daemon Auto-Start

The dashboard automatically triggers daemon auto-start when launched. No manual
daemon management is required - the TUI client handles connection and startup
transparently.

If the daemon is not running:

1. Dashboard launches and detects no daemon
2. Auto-start initiates daemon process
3. Dashboard connects and displays session data

## Troubleshooting

### Zellij Not Installed

**Error:**

```text
Error: Zellij not installed. See https://zellij.dev/documentation/installation
```

**Solution:** Install Zellij via your package manager or from source.

```sh
# macOS
brew install zellij

# Linux (cargo)
cargo install zellij
```

### Nested Session Warning

**Error:**

```text
Already inside Zellij session.
Use 'zellij action new-pane' to add dashboard to current session.
```

**Solution:** Either exit the current Zellij session first, or manually add a
dashboard pane to the existing session.

### Version Compatibility Warning

**Warning:**

```text
Warning: Zellij 0.38.2 detected. Version 0.39.0+ recommended.
```

**Impact:** Older Zellij versions may have different KDL syntax or command
support. The layouts should still work, but update Zellij if issues occur.

**Solution:**

```sh
# Update via package manager
brew upgrade zellij  # macOS
cargo install --force zellij  # From source
```

### Layout File Not Found

**Error:**

```text
Error: Layout file not found: /path/to/layout.kdl
```

**Solution:** Ensure you're running the launcher script from the project root or
that the layout files exist in `scripts/zellij/layouts/`.

### Agent Console Not Found

**Error:**

```text
command not found: agent-console
```

**Solution:** Build and install the binary:

```sh
# Build release version
cargo build --release

# Install to cargo bin directory (adds to PATH)
cargo install --path .
```

Verify installation:

```sh
which agent-console
agent-console --version
```

### Dashboard Pane Shows No Data

**Possible causes:**

1. **Daemon not running:** Dashboard should auto-start it, but check manually:

   ```sh
   agent-console daemon status
   ```

2. **No active sessions:** The dashboard displays nothing if no Claude Code
   sessions exist. Start a session first.

3. **Permission issues:** Check daemon logs for errors:

   ```sh
   agent-console daemon logs
   ```

## Customization

### Creating Custom Layouts

Copy an existing layout and modify the pane size or arguments:

```kdl
layout {
    pane size=4 {
        command "agent-console"
        args "tui" "--layout" "two-line"
    }
    pane split_direction="horizontal" {
        pane
        pane
        pane
    }
}
```

Save to `~/.config/zellij/layouts/custom.kdl` and launch:

```sh
zellij --layout custom
```

### Adjusting Pane Sizes

The `size=N` parameter controls pane height in rows. Experiment with different
values:

- `size=1` - Ultra-compact (may truncate content)
- `size=2` - Minimal (one-line layout)
- `size=3` - Default (two-line layout)
- `size=5` - Detailed (full session info)
- `size=10` - Extra detailed (for debugging)

### Dashboard Placement

By default, the dashboard appears at the top. To place it at the bottom, reverse
the pane order:

```kdl
layout {
    pane
    pane size=3 {
        command "agent-console"
        args "tui" "--layout" "two-line"
    }
}
```

## Platform Support

Tested on:

- **macOS** (Sonoma 14.x+)
- **Linux** (Ubuntu 22.04+, Arch, Fedora)

Windows support depends on Zellij Windows compatibility (WSL recommended).

## See Also

- [Zellij documentation](https://zellij.dev/documentation/)
- [KDL layout syntax](https://zellij.dev/documentation/creating-a-layout.html)
- Story S010.01: Create Zellij Layout with Dashboard Pane
- Epic E010: Zellij Integration
