# TUI Polish Epic (acd-cx9)

Epic for polishing the TUI dashboard display and interaction. All stories in
this epic must be implemented sequentially due to shared file dependencies.

## Dependencies

All issues touch the same core files:

- `crates/agent-console-dashboard/src/tui/views/dashboard.rs` — session list
  rendering
- `crates/agent-console-dashboard/src/tui/ui.rs` — dashboard layout
- `crates/agent-console-dashboard/src/tui/app.rs` — app state and view
  management

Sequential implementation prevents merge conflicts and ensures each story builds
on the previous work.

## Story Sequence

### Story 1: Display Missing CWD as Red Error (acd-lht, P2)

#### Type, Priority, Dependency

Type: Bug fix Priority: P2 Dependency: None (first story)

#### Description

When the hook input is missing the `cwd` field, the TUI currently displays
"unknown" in gray text in the working directory column. This makes errors
indistinguishable from placeholder text. We need to display `<error>` in red to
clearly indicate a data integrity problem.

#### Current Behavior

In `format_session_line()` at lines 115 and 126 of `dashboard.rs`, the code
calls `truncate_path(&session.working_dir, N)` which always succeeds because
`working_dir` is a `PathBuf` (never None). When the daemon receives a SET
command without a working directory, it defaults to `PathBuf::from("unknown")`
(see `handlers.rs` line 44).

#### Acceptance Criteria

1. When `session.working_dir` equals `PathBuf::from("unknown")`, display
   `<error>` instead of "unknown"
2. The `<error>` text must be styled with `Color::Red` (use the existing
   `error_color()` function)
3. Normal working directories continue to display in their current color (dimmed
   style)
4. All three width breakpoints (narrow, standard, wide) handle the error case
   correctly
5. Unit tests verify the red error display for "unknown" working directory
6. Unit tests verify normal paths still display correctly

#### Implementation Hints

##### Files to modify

- `crates/agent-console-dashboard/src/tui/views/dashboard.rs`

##### Approach

1. In `format_session_line()`, before building the spans, check if
   `session.working_dir == PathBuf::from("unknown")`
2. If true, create a special span:
   `Span::styled("<error>", Style::default().fg(error_color()))`
3. If false, use the existing `truncate_path()` logic with dimmed style
4. Apply this conditional to all three width branches (lines 115, 126)
5. Add test case: `test_format_session_line_unknown_working_dir_shows_error()`
6. Add test case: `test_format_session_line_normal_path_unchanged()`

##### Pattern to follow

```rust
let work_dir_span = if session.working_dir == PathBuf::from("unknown") {
    Span::styled("<error>", Style::default().fg(error_color()))
} else {
    Span::styled(format!("{:<20} ", truncate_path(&session.working_dir, 20)), dim)
};
```

---

### Story 2: Right-Align Layout Columns (acd-r57, P3)

#### Type, Priority, Dependency

Type: Task Priority: P3 Dependency: Story 1 (acd-lht) must be complete

#### Description

The current TUI layout centers all content, making the dashboard hard to scan.
Implement flexbox-like column alignment: the session name column should expand
to fill available space, while the status, working directory, and elapsed time
columns should be right-aligned with fixed widths.

#### Current Behavior

All spans in `format_session_line()` use fixed-width left-padding (e.g.,
`{:<20}`) and are concatenated. Ratatui's `Line` widget has no built-in
alignment concept — it just flows spans left-to-right.

#### Acceptance Criteria

1. Session name (first column) expands to fill remaining space
2. Last three columns are right-aligned:
   - Status: fixed 10 chars
   - Working directory: fixed 20 chars (standard width) or 30 chars (wide width)
   - Elapsed time: variable width, right-aligned
3. Column alignment works at all three width breakpoints (narrow, standard,
   wide)
4. Columns remain readable when terminal is resized
5. Unit tests verify correct span count and formatting
6. Visual test: columns align vertically when viewing multiple sessions

#### Implementation Hints

##### Files to modify

- `crates/agent-console-dashboard/src/tui/views/dashboard.rs`

##### Approach

Ratatui doesn't have flex layout. To simulate right-aligned columns:

1. Calculate the remaining width after fixed columns:

   ```rust
   let fixed_width = symbol_width + status_width + wd_width + elapsed_width + padding;
   let name_width = width.saturating_sub(fixed_width);
   ```

2. Pad the session name to the calculated width:

   ```rust
   Span::styled(format!("{:<name_width$} ", name), dim)
   ```

3. Right-align status, working_dir, elapsed by using `{:>N}` instead of `{:<N}`:

   ```rust
   Span::styled(format!("{:>10} ", status_text), Style::default().fg(color))
   Span::styled(format!("{:>20} ", work_dir), dim)
   Span::styled(format!("{:>8}", elapsed), dim)  // no trailing space on last column
   ```

4. Update all three width branches (narrow, standard, wide)

5. Test cases:
   - `test_column_alignment_standard_width()`
   - `test_column_alignment_wide_width()`
   - `test_name_column_expands_with_terminal_width()`

##### Known edge cases

- Very narrow terminals (width < 40): narrow mode shows symbol + name only, no
  alignment needed
- Very wide terminals: name column should expand, not the fixed columns

---

### Story 3: Add Column Headers (acd-8uw, P3)

#### Type, Priority, Dependency

Type: Task Priority: P3 Dependency: Story 2 (acd-r57) must be complete

#### Description

The session list has no header row, making it unclear what each column
represents. Add a header row above the session list with column titles aligned
to their respective columns.

#### Current Behavior

The `render_session_list()` function creates a `List` widget with a block title
" Sessions " but no column headers. Users must infer column meanings from the
data.

#### Acceptance Criteria

1. Header row displays above the session list with column titles:
   - Narrow mode: no headers (symbol + name only)
   - Standard mode: "Name", "Status", "Working Directory", "Elapsed"
   - Wide mode: "Session ID", "Name", "Status", "Working Directory", "Elapsed"
2. Header text aligns with its column data (matches the alignment from Story 2)
3. Header styling: bold or distinct color (e.g., `Color::Cyan` with
   `Modifier::BOLD`)
4. Header does not scroll with the session list (fixed at top of list area)
5. Unit test verifies header rendering
6. Visual test: header remains visible when scrolling through sessions

#### Implementation Hints

##### Files to modify

- `crates/agent-console-dashboard/src/tui/views/dashboard.rs`

##### Approach

Ratatui's `List` widget doesn't support headers. Options:

###### Option A: Separate header widget (Recommended)

1. In `render_session_list()`, split the `area` into header (1 line) + list
   (remaining):

   ```rust
   let chunks = Layout::default()
       .direction(Direction::Vertical)
       .constraints([
           Constraint::Length(1),  // header
           Constraint::Min(1),     // list
       ])
       .split(area);
   ```

2. Create a header `Line` using the same width logic as `format_session_line()`:

   ```rust
   fn format_header_line(width: u16) -> Line<'static> {
       if width < NARROW_THRESHOLD {
           Line::from(vec![])  // no header in narrow mode
       } else if width <= WIDE_THRESHOLD {
           Line::from(vec![
               Span::styled("Name            ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
               Span::styled("    Status", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
               Span::styled("  Working Directory", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
               Span::styled("  Elapsed", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
           ])
       } else {
           // wide mode with Session ID column
       }
   }
   ```

3. Render the header as a `Paragraph` in `chunks[0]`

4. Render the list in `chunks[1]` (instead of full `area`)

###### Option B: Prepend header to List items

Simpler but header scrolls with content (less desirable).

###### Recommended

Option A for fixed header.

##### Test cases

- `test_header_alignment_matches_data()`
- `test_header_narrow_mode_no_header()`
- `test_header_wide_mode_includes_session_id()`

---

### Story 4: Compact 3-Line Layout (acd-hex, P3)

#### Type, Priority, Dependency

Type: Feature Priority: P3 Dependency: Story 3 (acd-8uw) must be complete

#### Description

Add a compact dashboard layout that fits in 3 terminal lines (header + 1
session + footer) for use in tmux status panes or limited-height terminals.
Auto-detect when terminal height ≤ 5 lines, or force with `--layout compact`
flag.

#### Current Behavior

The layout preset infrastructure exists (`layout_preset` field in `App`, key `2`
toggles preset), but the rendering code in `dashboard.rs` doesn't differentiate
between presets. All presets use the same full-height layout.

#### Acceptance Criteria

1. When `app.layout_preset == 2` (compact mode), render only the most important
   session (selected or most recent)
2. Compact mode layout:
   - Line 1: Header ("Agent Console Dashboard")
   - Line 2: Single session row (symbol + name + status + elapsed, no working
     directory)
   - Line 3: Footer (keybindings)
3. Auto-detect: if terminal height ≤ 5 lines, automatically switch to compact
   mode
4. `--layout compact` flag on `acd tui` command forces compact mode regardless
   of height
5. Compact mode respects all visual treatments (colors, inactive sessions, etc.)
6. Unit tests verify compact rendering
7. Manual test: works in tmux status pane (3 lines)

#### Implementation Hints

##### Files to modify

- `crates/agent-console-dashboard/src/tui/ui.rs`
- `crates/agent-console-dashboard/src/tui/views/dashboard.rs`
- `crates/agent-console-dashboard/src/main.rs` (add `--layout` flag)

##### Approach

###### Step 1: Add CLI flag

In `main.rs`, add to `TuiCommand` struct:

```rust
#[derive(Args, Debug)]
pub struct TuiCommand {
    /// Layout preset: default, compact
    #[arg(long, value_name = "NAME")]
    layout: Option<String>,
}
```

Parse and set `app.layout_preset`:

```rust
if let Some(layout_name) = &tui_cmd.layout {
    app.layout_preset = match layout_name.as_str() {
        "compact" => 2,
        "default" => 1,
        _ => {
            eprintln!("Unknown layout: {}", layout_name);
            std::process::exit(1);
        }
    };
}
```

###### Step 2: Auto-detect in event loop

In `app.rs`, inside `event_loop()` before rendering:

```rust
if terminal.size()?.height <= 5 && app.layout_preset == 1 {
    app.layout_preset = 2;  // auto-switch to compact
}
```

###### Step 3: Implement compact rendering

In `ui.rs`, modify `render_dashboard()`:

```rust
pub fn render_dashboard(frame: &mut Frame, app: &App) {
    if app.layout_preset == 2 {
        render_compact_dashboard(frame, app);
    } else {
        // existing full rendering
    }
}

fn render_compact_dashboard(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // header
            Constraint::Length(1),  // single session
            Constraint::Length(1),  // footer
        ])
        .split(area);

    // Render header
    let header = Paragraph::new(Line::from(vec![Span::styled(
        HEADER_TEXT,
        Style::default().fg(Color::Cyan),
    )]));
    frame.render_widget(header, chunks[0]);

    // Render single session (selected or first)
    let session = app.selected_session().or_else(|| app.sessions.first());
    if let Some(s) = session {
        let line = format_compact_session_line(s);
        let paragraph = Paragraph::new(line);
        frame.render_widget(paragraph, chunks[1]);
    }

    // Render footer
    let footer = Paragraph::new(Line::from(vec![Span::styled(
        FOOTER_TEXT,
        Style::default().fg(Color::DarkGray),
    )]));
    frame.render_widget(footer, chunks[2]);
}
```

In `dashboard.rs`, add:

```rust
fn format_compact_session_line(session: &Session) -> Line<'_> {
    // symbol + name (truncated to 15) + status + elapsed
    // No working directory in compact mode
}
```

###### Test cases

- `test_compact_mode_renders_single_session()`
- `test_compact_mode_auto_switch_on_small_height()`
- `test_compact_mode_cli_flag_forces_preset()`

---

### Story 5: Wire TUI Resurrect to Daemon (acd-bwa, P3)

#### Type, Priority, Dependency

Type: Task Priority: P3 Dependency: Story 4 (acd-hex) must be complete

#### Description

The `r` key in the TUI currently generates a `Resurrect` action but does nothing
with it (see `app.rs` line 249:
`// TODO: send RESURRECT IPC command to daemon`). Wire the keybinding to call
the daemon's existing `RESURRECT` command via IPC, then execute the returned
command to resume the session.

#### Current Behavior

When the user presses `r` on a closed session:

1. `handle_key_event()` returns `Action::Resurrect(session_id)` (event.rs
   line 127)
2. The action is logged but not handled (app.rs line 248)
3. Nothing happens

The daemon already implements `handle_resurrect_command()` (handlers.rs
line 281) which validates the session and returns JSON with `session_id`,
`working_dir`, and `command`.

#### Acceptance Criteria

1. Pressing `r` on a closed session sends `RESURRECT <session_id>` to the daemon
   via socket
2. Parse the daemon's JSON response to extract resurrection metadata
3. Execute the command in a new terminal pane/window using the existing
   `terminal::executor` module
4. Display a status message in the TUI: "Resurrecting session `<id>`..."
   (success) or "Failed to resurrect: `<reason>`" (error)
5. If the daemon returns ERR, display the error message
6. Unit tests verify IPC message construction
7. Integration test (manual): resurrect a closed Claude Code session in Zellij

#### Implementation Hints

##### Files to modify

- `crates/agent-console-dashboard/src/tui/app.rs`
- `crates/agent-console-dashboard/src/client.rs` (add `resurrect_session()`
  helper)

##### Approach

###### Step 1: Add client helper

In `client.rs`, add:

```rust
/// Sends RESURRECT command to daemon and returns resurrection metadata.
pub async fn resurrect_session(
    socket_path: &Path,
    session_id: &str,
) -> Result<ResurrectResponse, Box<dyn std::error::Error>> {
    let mut stream = UnixStream::connect(socket_path).await?;
    let command = format!("RESURRECT {}\n", session_id);
    stream.write_all(command.as_bytes()).await?;

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response).await?;

    if response.starts_with("OK") {
        let json_str = response.trim_start_matches("OK ").trim();
        let metadata: ResurrectResponse = serde_json::from_str(json_str)?;
        Ok(metadata)
    } else {
        Err(response.trim_start_matches("ERR ").trim().into())
    }
}

#[derive(serde::Deserialize)]
pub struct ResurrectResponse {
    pub session_id: String,
    pub working_dir: String,
    pub command: String,
}
```

###### Step 2: Handle Action::Resurrect

In `app.rs`, replace the TODO (line 247-250):

```rust
Action::Resurrect(id) => {
    tracing::debug!("resurrect session {id}");
    let socket_path = self.socket_path.clone();
    let session_id = id.clone();

    tokio::spawn(async move {
        match client::resurrect_session(&socket_path, &session_id).await {
            Ok(metadata) => {
                tracing::info!("Resurrecting session: {:?}", metadata);
                // TODO: execute metadata.command in terminal
                // Use terminal::executor::execute_in_terminal()
            }
            Err(e) => {
                tracing::error!("Failed to resurrect {}: {}", session_id, e);
                // TODO: display error in status line
            }
        }
    });
}
```

###### Step 3: Execute command

Import and use `terminal::executor::execute_in_terminal()`:

```rust
use crate::terminal::executor::{execute_in_terminal, ExecutionResult};

match execute_in_terminal(&metadata.command, &[], Some(Path::new(&metadata.working_dir))) {
    Ok(ExecutionResult::Executed) => {
        tracing::info!("Session {} resurrected", session_id);
    }
    Ok(ExecutionResult::DisplayCommand(cmd)) => {
        tracing::warn!("Manual resurrection required: {}", cmd);
    }
    Err(e) => {
        tracing::error!("Execution failed: {}", e);
    }
}
```

###### Step 4: Status message (optional enhancement)

Add a `status_message: Option<String>` field to `App` and display it in the
footer or as a temporary overlay.

##### Test cases

- `test_resurrect_sends_ipc_command()`
- `test_resurrect_parses_json_response()`
- `test_resurrect_handles_error_response()`

##### Integration test

1. Start daemon: `acd daemon start`
2. Create closed session: `acd set test-session closed /tmp/test`
3. Start TUI: `acd tui`
4. Navigate to "test-session" and press `r`
5. Verify: new pane opens with `claude --resume test-session`

---

### Story 6: Mouse and Cursor Interaction (acd-3cv, P4)

#### Type, Priority, Dependency

Type: Feature Priority: P4 Dependency: Story 5 (acd-bwa) must be complete

#### Description

Add mouse and cursor support to the TUI for clicking to select sessions,
scrolling through the list, and text selection. Ratatui supports mouse events
via crossterm — we need to enable them and handle the events.

#### Current Behavior

The TUI is keyboard-only. Mouse clicks, scrolling, and text selection are
ignored.

#### Acceptance Criteria

1. Click on a session row to select it (updates `selected_index`)
2. Double-click on a session to open detail view
3. Scroll wheel scrolls through session list
4. Mouse drag selects text (for copying session IDs, paths, etc.)
5. Mouse events work alongside existing keyboard navigation
6. Unit tests verify mouse event handling logic
7. Manual test: all mouse interactions work in terminal emulators that support
   mouse (iTerm2, Alacritty, etc.)

#### Implementation Hints

##### Files to modify

- `crates/agent-console-dashboard/src/tui/app.rs`
- `crates/agent-console-dashboard/src/tui/event.rs`

##### Approach

###### Step 1: Enable mouse capture

In `app.rs`, modify `setup_terminal()`:

```rust
use crossterm::event::{EnableMouseCapture, DisableMouseCapture};

fn setup_terminal() -> io::Result<()> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    Ok(())
}

fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}
```

###### Step 2: Handle mouse events

In `event.rs`, add `Event::Mouse` variant:

```rust
pub enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),  // NEW
    Resize(u16, u16),
    Tick,
}
```

Update `EventHandler::next()`:

```rust
match maybe_event {
    Some(Ok(CrosstermEvent::Key(key))) => return Ok(Event::Key(key)),
    Some(Ok(CrosstermEvent::Mouse(mouse))) => return Ok(Event::Mouse(mouse)),  // NEW
    Some(Ok(CrosstermEvent::Resize(w, h))) => return Ok(Event::Resize(w, h)),
    // ...
}
```

###### Step 3: Process mouse events

In `app.rs`, handle `Event::Mouse`:

```rust
Event::Mouse(mouse) => {
    match handle_mouse_event(self, mouse) {
        Action::OpenDetail(idx) => self.open_detail(idx),
        // ... handle other actions
        _ => {}
    }
}
```

Create `handle_mouse_event()`:

```rust
fn handle_mouse_event(app: &mut App, mouse: MouseEvent) -> Action {
    use crossterm::event::{MouseEventKind, MouseButton};

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // Calculate which row was clicked based on mouse.row
            // Account for header (1 line) and block borders
            let session_index = calculate_clicked_session(mouse.row, app);
            if let Some(idx) = session_index {
                app.selected_index = Some(idx);
                Action::None
            } else {
                Action::None
            }
        }
        MouseEventKind::DoubleClick(MouseButton::Left) => {
            let session_index = calculate_clicked_session(mouse.row, app);
            if let Some(idx) = session_index {
                Action::OpenDetail(idx)
            } else {
                Action::None
            }
        }
        MouseEventKind::ScrollDown => {
            app.select_next();
            Action::None
        }
        MouseEventKind::ScrollUp => {
            app.select_previous();
            Action::None
        }
        _ => Action::None,
    }
}

fn calculate_clicked_session(row: u16, app: &App) -> Option<usize> {
    // Header takes 1 line, block border takes 1 line
    // Session list starts at row 2
    if row < 2 {
        return None;
    }
    let list_row = (row - 2) as usize;
    if list_row < app.sessions.len() {
        Some(list_row)
    } else {
        None
    }
}
```

###### Step 4: Test mouse events

```rust
#[test]
fn test_mouse_click_selects_session() {
    let mut app = make_app_with_sessions(5);
    let mouse = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 0,
        row: 4,  // Click on 3rd session (header=1, border=1, session_0=2, session_1=3, session_2=4)
        modifiers: KeyModifiers::NONE,
    };
    handle_mouse_event(&mut app, mouse);
    assert_eq!(app.selected_index, Some(2));
}
```

##### Caveat

Mouse support varies by terminal emulator. Test on iTerm2, Alacritty, and tmux.

---

## Implementation Order

1. acd-lht (P2) — Red error display for missing CWD
2. acd-r57 (P3) — Right-align columns
3. acd-8uw (P3) — Add column headers
4. acd-hex (P3) — Compact 3-line layout
5. acd-bwa (P3) — Wire resurrect to daemon
6. acd-3cv (P4) — Mouse interaction

## Testing Strategy

Each story includes:

- Unit tests for rendering logic
- Integration tests where applicable (stories 5 and 6)
- Manual visual tests in terminal

Run tests with:

```bash
cargo test -p agent-console-dashboard --lib tui
```

## Notes

- All stories share the same files — implement in sequence to avoid conflicts
- The epic maintains backward compatibility — no breaking changes to daemon
  protocol
- Compact mode (story 4) is additive — default behavior unchanged
- Mouse support (story 6) is optional enhancement — keyboard navigation still
  primary
