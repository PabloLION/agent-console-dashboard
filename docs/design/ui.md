# UI Design

Widget-based TUI where each widget is a stateless renderer. The daemon owns all
state; the dashboard reads from a shared context and draws frames. Users
configure which widgets appear and in what order.

## Philosophy

- Widgets are the unit of display. Each widget occupies one or more lines.
- Widgets are stateless: they read from `WidgetContext`, render, and return. No
  widget holds data between frames.
- Color conveys meaning (semantic color scheme). No icons or symbols needed.
- Static display only, no animations. Color changes signal state transitions.
- Keyboard-first, mouse-supported. Vim-style bindings with arrow key fallback.

## Session Statuses

| Status    | Color   | Time display | Meaning              |
| --------- | ------- | ------------ | -------------------- |
| Working   | Blue    | mm:ss        | Active, processing   |
| Attention | Yellow  | mm:ss        | Needs user attention |
| Question  | Magenta | mm:ss        | Awaiting user input  |
| Idle      | Gray    | hidden       | No activity > 100m   |
| Closed    | Gray    | hidden       | Inactive             |

Idle detection threshold is 100 minutes (configurable). Idle sessions hide the
timer to save horizontal space for active sessions. See
[Q4](../archive/planning/6-open-questions.md),
[Q75](../archive/planning/6-open-questions.md),
[Q77](../archive/planning/6-open-questions.md).

## Widget Model

A widget is a self-contained renderer with a single purpose. It receives a
`WidgetContext` (read-only session data, terminal dimensions, selection state)
and returns styled lines.

Available widgets:

- `session-status` -- session names with status colors and timers
- `session-detail` -- expanded view of a selected session
- `api-usage` -- token quota percentages for 5h and 7d periods
- `state-history` -- status transition history of selected session
- `working-dir` -- current working directory
- `clock` -- current time
- `spacer` -- empty line for visual separation

## Layouts

Two layout options, differing by how much screen space the session widget
occupies.

### 2-line layout

```text
Line 1: <- 3+ | my-project 05:23 | [api-server] 03:15 | 5+ ->   (sessions)
Line 2: 5h: 42% / 50%  7d: 77% / 43%                            (usage)
```

When a session is selected, Line 2 swaps from usage to history:

```text
Line 1: <- 3+ | my-project 05:23 | [api-server] 03:15 | 5+ ->
Line 2: <- 2+ | working 10m | attention 2m | question 30s ->     (history)
```

### 3-line layout (default)

```text
Line 1: <- 3+ | my-project 05:23 | [api-server] 03:15 | 5+ ->   (sessions)
Line 2: <- 2+ | working 10m | attention 2m | question 30s ->     (history)
Line 3: 5h: 42% / 50%  7d: 77% / 43%                            (usage)
```

Line 2 content depends on Line 1 selection:

| Line 1 selection | Line 2 shows            |
| ---------------- | ----------------------- |
| Global item      | Global activity feed    |
| Session          | History of that session |

See [Q94](../archive/planning/6-open-questions.md),
[Q96](../archive/planning/6-open-questions.md),
[Q98](../archive/planning/6-open-questions.md).

### Full TUI (interactive)

```text
+-- Agent Console Dashboard ----------------------------------------+
|                                                                   |
|  Sessions:                                                        |
|  proj-a      Working      ~/projects/proj-a                       |
|  proj-b      Attention    ~/projects/proj-b      2m34s            |
|  proj-c      Question     ~/projects/proj-c                       |
|  old-proj    Closed       ~/old/project                           |
|                                                                   |
|  API Usage: 45.2k tokens today | ~$1.80                           |
|                                                                   |
|  [j/k] Navigate  [Enter] Details  [r] Resurrect  [q] Quit        |
+-------------------------------------------------------------------+
```

### Column layout

Full TUI columns: directory (flex), session_id (40), status (14), time elapsed
(16). Directory fills remaining width after fixed columns and highlight marker
(2 chars). Status width is 14 (even number: max label "attention" is 9 chars + 5
padding). Session ID is 40 (UUID 36 chars + 4 padding). Time Elapsed is 16
(HH:MM:SS 8 + 8 padding). Narrow mode (< 40 cols) shows only symbol + session
ID, no columnar layout.

Cell content alignment: all columns left-aligned, trailing padding. Highlight
marker (▶ + space, 2 chars) always reserved via `HighlightSpacing::Always`, even
when no item is selected.

### Responsive behavior

Sessions display horizontally by default (separated by `|`). Vertical layout is
available for wider terminals, with compact and full display modes.

| Width    | Behavior                                          |
| -------- | ------------------------------------------------- |
| < 40 col | Abbreviate session names, hide details            |
| 40-80    | Standard display                                  |
| > 80     | Show additional columns (session ID, working dir) |

Pagination handles overflow: hidden items show as `<- N+` and `M+ ->`.
Navigation shifts by one item at a time; the viewport auto-scrolls to keep the
selection visible. See [Q76](../archive/planning/6-open-questions.md),
[Q78](../archive/planning/6-open-questions.md).

## Navigation and Interaction

### Keyboard shortcuts

| Key    | Action                           |
| ------ | -------------------------------- |
| h / <- | Navigate left                    |
| l / -> | Navigate right                   |
| j / v  | Navigate down (Line 1 to Line 2) |
| k / ^  | Navigate up (Line 2 to Line 1)   |
| Enter  | Action on selected (Line 1 only) |
| Esc    | Deselect / clear focus           |
| q      | Quit dashboard                   |
| ?      | Toggle help overlay              |

### Mouse and focus

- Click to select an item. Hover to focus. Double-click for Enter action.
- Selected item renders with inverse background color.
- Terminal unfocus removes visual highlight but remembers position.
- Terminal refocus restores the previous selection.
- Initial state on app start: global item selected.
- Esc on Line 2 jumps focus to Line 1. Esc again deselects entirely.

See [Q79](../archive/planning/6-open-questions.md),
[Q80](../archive/planning/6-open-questions.md),
[Q102](../archive/planning/6-open-questions.md).

### Global item

The first position on Line 1 is a global item (`[G]`). Selecting it shows a
cross-session activity feed on Line 2. Pressing Enter on a feed entry jumps to
that session. See [Q100](../archive/planning/6-open-questions.md).

```text
Line 1: <- [G] | my-project 05:23 | api-server 03:15 | 5+ ->
Line 2: <- 2+ | my-project -> attention | api-server -> working ->
```

### Help overlay

Toggled with `?`. Any keypress dismisses it; the key is consumed and not
forwarded. See [Q83](../archive/planning/6-open-questions.md),
[Q101](../archive/planning/6-open-questions.md).

### Error display

Errors (daemon disconnect, API failure) replace session content temporarily.
Normal display resumes when the error resolves. See
[Q82](../archive/planning/6-open-questions.md).

### Name conflicts and long names

Same basename, different path: show parent folder (`work/my-app`). Same path,
multiple sessions: append session ID suffix (`my-app [abc1]`). Long names:
abbreviate non-distinguishing parents to first character, then truncate middle
as last resort (`base/p/w/my-app`). See
[Q41](../archive/planning/6-open-questions.md),
[Q42](../archive/planning/6-open-questions.md).

## Design Decisions

| Decision                  | Choice                                      | See                                             |
| ------------------------- | ------------------------------------------- | ----------------------------------------------- |
| Default layout            | 3-line                                      | [Q94](../archive/planning/6-open-questions.md)  |
| Color scheme              | Semantic (blue=working, yellow=attention)   | [Q75](../archive/planning/6-open-questions.md)  |
| Status indicators         | Color only, no icons                        | [Q77](../archive/planning/6-open-questions.md)  |
| Animations                | None (static display)                       | [Q81](../archive/planning/6-open-questions.md)  |
| Input model               | Vim-style + arrows + mouse                  | [Q79](../archive/planning/6-open-questions.md)  |
| Session overflow          | Pagination with hidden count                | [Q76](../archive/planning/6-open-questions.md)  |
| Pagination order          | Stable in v0, dynamic reorder in v1+        | [Q91](../archive/planning/6-open-questions.md)  |
| Enter on focused session  | Switch to session terminal tab              | [Q104](../archive/planning/6-open-questions.md) |
| Startup (no sessions)     | "No active sessions. See README."           | [Q28](../archive/planning/6-open-questions.md)  |
| Usage periods             | 5h and 7d, percentage of quota vs time      | [Q84](../archive/planning/6-open-questions.md)  |
| Usage refresh rate        | 180s default (configurable)                 | [Q86](../archive/planning/6-open-questions.md)  |
| Line 2 selectability      | Selectable, no action on Enter              | [Q95](../archive/planning/6-open-questions.md)  |
| 2-line history display    | Replaces usage when session selected        | [Q96](../archive/planning/6-open-questions.md)  |
| Focus on terminal refocus | Restore previous selection                  | [Q102](../archive/planning/6-open-questions.md) |
| Name conflicts            | Parent folder or session ID suffix          | [Q41](../archive/planning/6-open-questions.md)  |
| Long name truncation      | Abbreviate parents, truncate middle         | [Q42](../archive/planning/6-open-questions.md)  |
| Column widths (Full TUI)  | dir=flex, id=40, status=14, time elapsed=16 | —                                               |
| Cell content alignment    | Left-aligned, trailing padding              | —                                               |
| Highlight marker          | ▶ (filled triangle), always show spacing    | —                                               |
