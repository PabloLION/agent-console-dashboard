# UI Design

Widget-based terminal UI system.

---

## Design Philosophy

The UI is **widget-based**. Each line in the display is a widget. Users configure which widgets to show and in what order.

This same concept applies to a separate project: **Claude Code status line** (not part of this project, but shares the widget pattern).

---

## Widget System

### What is a Widget?

A widget is a single line of output with a specific purpose.

```text
┌─────────────────────────────────────────────────────────────┐
│ Widget 1: working-dir                                       │
│ ~/projects/my-app                                           │
├─────────────────────────────────────────────────────────────┤
│ Widget 2: session-status                                    │
│ proj-a: - | proj-b: 2m34s | proj-c: ?                       │
├─────────────────────────────────────────────────────────────┤
│ Widget 3: api-usage                                         │
│ Tokens: 12.3k in / 8.1k out | $0.42 est                     │
└─────────────────────────────────────────────────────────────┘
```

### Available Widgets

| Widget ID        | Content                                   |
| ---------------- | ----------------------------------------- |
| `working-dir`    | Current working directory                 |
| `session-status` | All sessions with status and elapsed time |
| `session-detail` | Expanded view of selected session         |
| `api-usage`      | Token counts and cost estimate            |
| `state-history`  | Recent state transitions                  |
| `clock`          | Current time (simple utility widget)      |
| `spacer`         | Empty line for visual separation          |

### Widget Configuration

In config file:

```toml
[ui]
widgets = ["session-status", "api-usage"]
```

Or specify per-layout:

```toml
[ui.layouts.one-line]
widgets = ["session-status"]

[ui.layouts.two-line]
widgets = ["working-dir", "session-status"]

[ui.layouts.detailed]
widgets = ["working-dir", "session-status", "api-usage", "state-history"]
```

---

## Named Layouts

Predefined widget collections users can switch between.

### Built-in Layouts

| Layout     | Widgets                                      | Use Case            |
| ---------- | -------------------------------------------- | ------------------- |
| `one-line` | `session-status`                             | Minimal, current v1 |
| `two-line` | `working-dir`, `session-status`              | Standard            |
| `detailed` | `working-dir`, `session-status`, `api-usage` | Full monitoring     |
| `history`  | `session-status`, `state-history`            | Debug/analysis      |

### Custom Layouts

Users can define custom layouts:

```toml
[ui.layouts.my-layout]
widgets = ["clock", "session-status", "spacer", "api-usage"]
```

### Switching Layouts

```bash
# Via CLI
agent-console tui --layout detailed

# Via keyboard (in TUI)
# 1, 2, 3, 4 = switch to layout 1, 2, 3, 4
# or use menu
```

---

## Session Status Widget

The core widget showing all tracked sessions.

### One-Line Format (v1 compatible)

```text
proj-a: - | proj-b: 2m34s | proj-c: ?
```

| Symbol | Meaning                    |
| ------ | -------------------------- |
| `-`    | Working (no attention)     |
| `Xm`   | Attention, X minutes       |
| `?`    | Question (AskUserQuestion) |
| `×`    | Closed (can resurrect)     |

### Expanded Format

When user selects a session for detail:

```text
┌── proj-b ──────────────────────────────────┐
│ Status: Attention (2m34s)                  │
│ Working Dir: ~/projects/proj-b             │
│ Session ID: abc123...                      │
│ API Usage: 5.2k tokens                     │
│                                            │
│ History:                                   │
│   14:32:01  Working → Attention            │
│   14:30:45  Attention → Working            │
│   14:28:12  Working → Attention            │
│                                            │
│ [R]esurrect  [C]lose  [ESC] Back           │
└────────────────────────────────────────────┘
```

---

## Color Scheme

| Status    | Color  |
| --------- | ------ |
| Working   | Green  |
| Attention | Yellow |
| Question  | Blue   |
| Closed    | Gray   |
| Error     | Red    |

Configurable in config file:

```toml
[ui.colors]
working = "green"
attention = "yellow"
question = "blue"
closed = "gray"
```

---

## Keyboard Shortcuts

| Key     | Action                   |
| ------- | ------------------------ |
| `j/k`   | Navigate sessions        |
| `Enter` | Expand session detail    |
| `r`     | Resurrect closed session |
| `d`     | Remove session from list |
| `1-4`   | Switch layout            |
| `q`     | Quit                     |
| `?`     | Help                     |

---

## Responsive Design

The TUI should adapt to terminal width:

| Width    | Behavior                                   |
| -------- | ------------------------------------------ |
| <40 cols | Abbreviate session names, hide details     |
| 40-80    | Standard display                           |
| >80      | Show additional columns (session ID, etc.) |

---

## Mock-ups

### One-Line Layout

```text
proj-a: - | proj-b: 2m34s | proj-c: ?
```

### Two-Line Layout

```text
~/projects/proj-b
proj-a: - | proj-b: 2m34s | proj-c: ?
```

### Detailed Layout

```text
~/projects/proj-b
proj-a: - | proj-b: 2m34s | proj-c: ?
Tokens: 12.3k in / 8.1k out | $0.42 est
```

### Full TUI (Interactive)

```text
┌─ Agent Console Dashboard ──────────────────────────────────┐
│                                                            │
│  Sessions:                                                 │
│  ● proj-a      Working      ~/projects/proj-a              │
│  ○ proj-b      Attention    ~/projects/proj-b      2m34s   │
│  ? proj-c      Question     ~/projects/proj-c              │
│  × old-proj    Closed       ~/old/project                  │
│                                                            │
│  API Usage: 45.2k tokens today | ~$1.80                    │
│                                                            │
│  [j/k] Navigate  [Enter] Details  [r] Resurrect  [q] Quit  │
└────────────────────────────────────────────────────────────┘
```

---

## Layout Orientations

### Horizontal Layout (Default)

Sessions displayed inline, separated by `|`:

```text
proj-a: - | proj-b: 2m34s | proj-c: ?
```

### Vertical Layout

Each session on its own line:

```text
proj-a    working     -
proj-b    attention   2m34s
proj-c    question    -
```

**Vertical layout display modes:**

| Mode    | Status Display | Time Display |
| ------- | -------------- | ------------ |
| Compact | `-`            | `2m`         |
| Full    | `working`      | `waited 2m`  |

**Compact mode:**

```text
proj-a    -           -
proj-b    attention   2m
proj-c    ?           -
```

**Full mode:**

```text
proj-a    working     -
proj-b    attention   waited 2m
proj-c    question    -
```

### Configuration

```toml
[ui]
orientation = "vertical"  # or "horizontal"
display_mode = "full"     # or "compact"
```

---

## Positioning in Ratatui

Ratatui provides terminal dimensions via `frame.area()`. Widgets can be positioned anywhere:

```rust
fn render(frame: &mut Frame) {
    let area = frame.area();

    // Bottom-right positioning
    let widget_width = 20;
    let widget_height = 3;
    let x = area.width.saturating_sub(widget_width);
    let y = area.height.saturating_sub(widget_height);

    let bottom_right = Rect::new(x, y, widget_width, widget_height);
    frame.render_widget(my_widget, bottom_right);
}
```

Zellij/tmux pane resizes trigger terminal resize events. Ratatui's crossterm backend detects this automatically.
