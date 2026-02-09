# Story Alignment Concerns

> Generated 2026-01-31 from 5 parallel review agents covering all 13 epics.
> Total: 112 raw concerns, deduplicated into clusters below.

## How to Use This File

- Work through clusters top-down (critical first)
- Check off items as resolved
- Each concern has file references for direct editing

---

## Cluster 1: Protocol & IPC Inconsistencies

**Severity:** Critical **Affects:** E001, E003, E004, E005, E009

### C01 — Protocol Format Contradiction (JSON Lines vs Text)

- **Files:** S003.01 lines 44-58, E003 lines 63-77
- **Issue:** Stories use JSON Lines format but E003 epic still shows old
  text-based protocol (`SET <session> <status> [metadata_json]`, `RM <session>`,
  `RESURRECT <session>`)
- **Fix:** Update E003 lines 63-77 to match JSON Lines format
- [x] Resolved

### C02 — Missing Story for RM Command

- **Files:** E003 line 66
- **Issue:** E003 lists `RM <session>` as a command but no S003.XX story
  implements it
- **Fix:** Create S003.XX or clarify deferred
- [x] Resolved

### C03 — RESURRECT Command Not in S003.01 Enum

- **Files:** E003 line 69, S003.01 Command enum
- **Issue:** E003 lists `RESURRECT <session>` in protocol. Implementation is in
  E008, but S003.01 doesn't mention RESURRECT in Command enum
- **Fix:** Add RESURRECT to S003.01's Command enum with note that execution is
  in E008
- [x] Resolved

### C04 — UsageUpdate Message Type Not Defined in Protocol

- **Files:** S004.02 lines 199-202, E003, S003.04
- **Issue:** S004.02 references `Message::UsageUpdate(usage)` but E003 doesn't
  define this message type and S003.04 doesn't mention it
- **Fix:** Add UsageUpdate to E003 protocol
- [x] Resolved

### C05 — Bounded Channel Size vs Max Clients Confusion

- **Files:** concurrency.md line 99, S001.02 line 127
- **Issue:** `mpsc::channel(100)` per subscriber vs "Max concurrent clients |
  100+". These are independent values but read as related
- **Fix:** Clarify 100 is per-subscriber buffer, independent of max clients
- [x] Resolved

### C06 — Subscriber Removal Mechanism Unclear

- **Files:** S003.04 line 82, S001.02
- **Issue:** S003.04 says failed send removes subscriber, but S001.02 doesn't
  explain subscriber management at all
- **Fix:** Add subscriber management to S001.02 or create dependency note
- [x] Resolved

---

## Cluster 2: Architecture Contradictions

**Severity:** Critical **Affects:** E009, E011

### C07 — E009 "Cut" Status Contradicts Story Body (CRITICAL)

- **Files:** E009 epic lines 7-9, S009.01 lines 6-11
- **Issue:** E009 epic says "daemon fetches account-level quota data". S009.01
  header says "This story was cut... TUI call claude_usage::get_usage()
  directly" but the body IMPLEMENTS daemon-centralized approach
- **Fix:** Remove "Status: Cut" from S009.01; header contradicts body
- [x] Resolved

### C08 — S009.01 File Name vs Title Mismatch

- **Files:** S009.01 filename: `S009.01-api-usage-data-model.md`, title:
  "Integrate claude-usage Crate"
- **Issue:** Filename does not match story title
- **Fix:** Rename file to match title
- [x] Resolved

### C09 — E011 Epic "Done" but Integration Unclear

- **Files:** E011 line 3
- **Issue:** E011 Status: Done, but daemon integration not verified
- **Fix:** Verify or change status
- [x] Resolved

### C10 — S011.06 Unchecked Checklist but Status "Done"

- **Files:** S011.06 lines 84-93
- **Issue:** All checklist items unchecked but status is Done
- **Fix:** Check items or change status
- [x] Resolved

### C11 — S011.08 Claims "Done" but Shows Unimplemented Code

- **Files:** S011.08 lines 96-108
- **Issue:** Shows daemon code examples, status: Done
- **Fix:** Clarify scope or change status
- [x] Resolved

### C12 — S011.05 Copy-Paste Bug

- **Files:** S011.05 `seven_day_on_pace()`
- **Issue:** `seven_day_on_pace()` calls `self.five_hour.is_on_pace()` instead
  of `self.seven_day`
- **Fix:** Fix the bug
- [x] Resolved

---

## Cluster 3: Session & Status Management Gaps

**Severity:** High **Affects:** E002, E003, E008

### C13 — Session Auto-Creation Metadata Undefined

- **Files:** S001.03 line 221, S002.01 line 151, S002.04 line 76
- **Issue:** All mention auto-creation but no story defines WHAT metadata is
  captured
- **Fix:** Add "Session Auto-Creation" section to S002.04
- [x] Resolved

### C14 — Same-Status Transition Behavior Missing from S002.03

- **Files:** S002.02 line 72, S002.03
- **Issue:** S002.02 says same-status updates timestamp without history entry,
  but S002.03 doesn't mention this at all
- **Fix:** Add same-status behavior to S002.03
- [x] Resolved

### C15 — Closed Session Cleanup Strategy Unclear

- **Files:** S002.04 lines 210-217
- **Issue:** Mentions "Time-based cleanup (default 30 minutes)" and "cleanup as
  part of auto-stop interval check" but no clear statement of WHEN closed
  sessions are removed
- **Fix:** Add cleanup mechanism to S002.04 acceptance criteria
- [x] Resolved

### C16 — Resurrection Metadata Requirements Vague

- **Files:** S002.04 line 52, S010.03 lines 41-45
- **Issue:** S002.04 says "Handle session metadata preservation for
  resurrection" but no story lists ALL metadata needed. S010.03 depends on
  S008.01 but doesn't specify expected schema
- **Fix:** Add "Resurrection Metadata" section to S002.04, reference specific
  E008 data structures
- [x] Resolved

### C17 — Resurrection TTL Config Missing

- **Files:** Q5 decision, E007, E008
- **Issue:** Q5 decides `[sessions] resurrection_ttl = "24h"` but E007 has no
  `[sessions]` section and E008 has no TTL
- **Fix:** Clarify if replaced by max_closed_sessions
- [x] Resolved

### C18 — No Session ID Format Validation

- **Files:** Stories reference session_id broadly
- **Issue:** No format spec for session IDs anywhere
- **Fix:** Document expected format
- [x] Resolved

### C19 — Display Name Derivation Edge Cases

- **Files:** S002.01 lines 159-175
- **Issue:** basename derivation with fallback "unknown" but no handling for "/"
  or non-UTF8 characters
- **Fix:** Add edge case handling to AC
- [x] Resolved

---

## Cluster 4: Elapsed Time & Timestamp Inconsistencies

**Severity:** High **Affects:** E003, E005, E012

### C20 — Elapsed Time Field Semantic Conflict

- **Files:** S003.03 line 119, S003.04 line 50
- **Issue:** S003.03: `"elapsed": 45` means "seconds in current status".
  S003.04: `"elapsed": 45` means "how long in previous status"
- **Fix:** Rename to "current_elapsed" in LIST and "previous_elapsed" in UPDATE
- [x] Resolved

### C21 — StateTransition Timestamp Type Inconsistency

- **Files:** S002.01 line 121, S002.01 line 180, S002.03 line 73
- **Issue:** Internal type is `Instant` which cannot be serialized as ISO 8601
  without conversion, but S002.03 says "timestamp field serialized as ISO 8601
  or Unix timestamp"
- **Fix:** Clarify internal=Instant, JSON=Unix timestamp
- [x] Resolved

### C22 — Elapsed Time Formatting: Seconds Dropped for Hour+

- **Files:** S005.02 lines 207-219
- **Issue:** Shows "2h5m" not "2h5m0s" — intentional?
- **Fix:** Confirm intentional, document behavior
- [x] Resolved

### C23 — Timestamp Timezone Ambiguity

- **Files:** S012.03 line 50, S012.03 line 122
- **Issue:** Shows "2026-01-31T10:00:00Z" but `pub created_at: String` with no
  timezone handling specified
- **Fix:** Specify always UTC
- [x] Resolved

---

## Cluster 5: Configuration Schema Fragmentation

**Severity:** High **Affects:** E007, E008, E013

### C24 — Socket Path Missing from E007 Schema

- **Files:** E007 epic line 88, S007.04 line 159
- **Issue:** E007 `[daemon]` has no `socket_path` but S007.04 shows
  `socket_path = "agent-console.sock"`
- **Fix:** Add socket_path to E007 schema
- [x] Resolved

### C25 — max_closed_sessions Missing from E007

- **Files:** S008.01 lines 206-209, E007 lines 73-91
- **Issue:** S008.01 defines `[daemon] max_closed_sessions = 20` but E007 schema
  doesn't include it
- **Fix:** Add to E007
- [x] Resolved

### C26 — color_scheme Missing from E007

- **Files:** S007.04 lines 120-124, E007 lines 73-77, E007 line 139
- **Issue:** S007.04 shows `color_scheme = "dark"`, E007 hot-reload lists
  "Colors" but no field defined
- **Fix:** Add to E007
- [x] Resolved

### C27 — Config File Integration Scattered Across Epics

- **Files:** E005, S005.05, E004, discussion-decisions.md
- **Issue:** Config snippets in multiple locations, no single story owns full
  config structure
- **Fix:** E007 should own this, other stories reference
- [x] Resolved

### C28 — E007 idle_timeout Comment Not Self-Contained

- **Files:** E007 line 87
- **Issue:** References "Q25 amendment" without explaining what idle_timeout
  controls
- **Fix:** Expand comment
- [x] Resolved

### C29 — History Depth Configuration Location Unclear

- **Files:** S002.03 line 49, S002.03 line 120, S002.03 line 136
- **Issue:** Config in `src/config.rs`, TOML under `[sessions]`, and
  `self.history_depth_limit` on Session struct — unclear where the value lives
- **Fix:** Clarify global config value, Session references it
- [x] Resolved

### C30 — Hot-Reload Scope Documented in 3 Places

- **Files:** S007.01, S007.02, E007
- **Issue:** All three document hot-reload behavior; consistent now but
  maintenance risk
- **Fix:** Consider single canonical location
- [x] Resolved

---

## Cluster 6: Widget System Gaps

**Severity:** High **Affects:** E004, E005, E007

### C31 — State History Widget Missing Story

- **Files:** E005 lines 78-87, E005 lines 103-109
- **Issue:** E005 lists `state-history` widget and uses it in `history` layout,
  but no story implements it
- **Fix:** Create S005.06 or mark future enhancement
- [x] Resolved

### C32 — Clock and Spacer Widgets Missing Stories

- **Files:** E005 lines 78-87, S005.01 lines 195-201
- **Issue:** E005 lists `clock` and `spacer` widgets, registry registers them,
  but no stories implement them
- **Fix:** Add stories or mark future
- [x] Resolved

### C33 — Widget Name Inconsistency: "session-status" vs "status"

- **Files:** S007.04 line 119, E007 line 76, S007.01 line 243
- **Issue:** S007.04 says `"session-status"`, E007 and S007.01 say `"status"`
- **Fix:** Standardize
- [x] Resolved

### C34 — S007.04 References Undefined Widgets

- **Files:** S007.04 line 119
- **Issue:** Lists "state-history", "clock", "spacer" — none have implementation
  stories
- **Fix:** Remove or mark as planned
- [x] Resolved

### C35 — Layout Preset Count Mismatch

- **Files:** S007.04 lines 105-109, S007.01/E007
- **Issue:** S007.04 shows 4 presets (one-line, two-line, detailed, history) vs
  3 in S007.01/E007 (one-line, two-line, custom)
- **Fix:** Align
- [x] Resolved

### C36 — Session Status Color/Symbol Mismatch

- **Files:** E004 lines 100-107, S005.02 lines 111-119
- **Issue:** E004: Working = `●` (filled circle). S005.02: Working = `-` (dash)
- **Fix:** Standardize on one symbol
- [x] Resolved

### C37 — UsageData Type Location Creates Circular Dependency

- **Files:** S005.01 lines 100-110, S005.01 line 56
- **Issue:** `WidgetContext` has `pub usage: &'a UsageData`, depends on E009 for
  type. Circular if UsageData in E009 but WidgetContext in E005
- **Fix:** Clarify UsageData location, recommend shared types
- [x] Resolved

### C38 — UsageData Default State Before First Broadcast

- **Files:** WidgetContext.usage is `&UsageData` (not Optional)
- **Issue:** No story specifies initial value before daemon broadcast
- **Fix:** Document `UsageData::default()` with `DataStatus::Unavailable`
- [x] Resolved

### C39 — Responsive Width Breakpoints vs Widget min_width

- **Files:** E004 lines 109-118
- **Issue:** Unclear interaction between layout breakpoints and widget min_width
- **Fix:** Clarify layout manager hides widgets below min_width
- [x] Resolved

---

## Cluster 7: Hook & Status Detection Inconsistencies

**Severity:** High **Affects:** E002, E006

### C40 — Hook Trigger: Notification → Attention Not Explained

- **Files:** S002.02 lines 99-105
- **Issue:** "Stop hook, Notification hook" trigger Attention status but no
  story explains WHY Notification hook triggers Attention
- **Fix:** Add note explaining semantic meaning
- [x] Resolved

### C41 — Session Identification: JSON stdin vs Env Var

- **Files:** E006 lines 118-123, E006 lines 136-148
- **Issue:** Hook uses `INPUT=$(cat)` and `jq .session_id` but PreToolUse hook
  uses `$CC_SESSION_ID` env var. Unclear which mechanism for which hook type
- **Fix:** Clarify per hook type
- [x] Resolved

### C42 — AskUserQuestion: PreToolUse vs Notification Duplicate

- **Files:** E006 lines 125-154, S006.03 lines 145-160
- **Issue:** PreToolUse + AskUserQuestion → "question" AND Notification hook
  also for "Question". Could cause duplicate status updates
- **Fix:** Clarify hook firing order and precedence
- [x] Resolved

---

## Cluster 8: Working Directory & Path Inconsistencies

**Severity:** High **Affects:** E002, E003, E010

### C43 — working_dir Field Source Ambiguity

- **Files:** S002.01 line 138, S002.04 line 163, D8 line 157
- **Issue:** S002.01 says "from cwd in JSON stdin", S002.04 shows it in SET
  command metadata, D8 lists cwd as available but no example
- **Fix:** Add "Hook JSON stdin Example" section
- [x] Resolved

### C44 — Metadata Field Name: working_dir vs cwd

- **Files:** S002.01 line 98, S003.05 line 43, S003.05 line 144
- **Issue:** Internal uses `working_dir: PathBuf` but CLI uses `--cwd` and JSON
  uses `"cwd"`
- **Fix:** Standardize on "working_dir" internally, accept "cwd" as CLI alias
- [x] Resolved

### C45 — `--cwd` Flag May Not Exist in Claude CLI

- **Files:** S010.02, S010.03
- **Issue:** Stories reference `claude --resume <id> --cwd <dir>` but this is
  not verified
- **Fix:** Verify or use `cd && claude`
- [x] Resolved

### C46 — Working Directory via Command::current_dir Won't Work for Zellij

- **Files:** S010.03 lines 146-150
- **Issue:** `command.current_dir(dir)` won't work because Zellij CLI's spawned
  pane doesn't inherit parent's cwd
- **Fix:** Use `--cwd` flag on claude command or `cd && claude`
- [x] Resolved

### C47 — Stale basename "$PWD" Pattern

- **Files:** S008.01 line 198, D8 line 167, E010 stories
- **Issue:** basename "$PWD" pattern is stale; may still appear in E010 stories
- **Fix:** Audit all E010 stories and remaining epics
- [x] Resolved

---

## Cluster 9: Service Management (launchd/systemd)

**Severity:** High **Affects:** E012, E013

### C48 — --daemonize Flag Referenced but Never Defined

- **Files:** S013.01 lines 122-124, S013.02 lines 109-113
- **Issue:** Both reference "does NOT use --daemonize flag" but the flag doesn't
  exist in architecture
- **Fix:** Define in E001 or remove references
- [x] Resolved

### C49 — Plist Tilde Expansion Unreliable

- **Files:** S013.01 line 53
- **Issue:** `~/.local/state/...` in StandardErrorPath; launchd doesn't expand
  tilde reliably
- **Fix:** Use $HOME expansion or absolute path
- [x] Resolved

### C50 — Log Directory Creation Responsibility Contradiction

- **Files:** S012.01 lines 33-35, S013.04 lines 122-128
- **Issue:** S012.01 says daemon creates directory; S013.04 gives manual mkdir
  instructions
- **Fix:** Confirm daemon auto-creates, make manual optional
- [x] Resolved

### C51 — systemd enable vs start Semantics

- **Files:** S013.02, S013.03
- **Issue:** S013.02: "Daemon starts after enable". But `systemctl enable`
  doesn't start immediately
- **Fix:** Update S013.02 AC
- [x] Resolved

### C52 — Uninstall launchctl Ordering Inconsistency

- **Files:** S013.03
- **Issue:** Table shows "unload, remove plist" but sequence shows "stop first,
  then remove"
- **Fix:** Align table with sequence
- [x] Resolved

### C53 — No Service File Syntax Validation in S013.03

- **Files:** S013.01 (has plutil -lint), S013.03
- **Issue:** S013.01 validates plist syntax but S013.03 doesn't validate before
  copy
- **Fix:** Add validation to S013.03
- [x] Resolved

### C54 — Restart Delay vs Backoff Mismatch

- **Files:** E013 line 49, S013.02 line 44
- **Issue:** E013 says "backoff delay" but S013.02 uses `RestartSec=5` (fixed,
  not backoff)
- **Fix:** Change E013 to "5-second delay" or implement actual backoff
- [x] Resolved

### C55 — XDG_STATE_HOME Not in launchd Environment

- **Files:** S012.01, S013.01 plist
- **Issue:** S012.01 uses XDG_STATE_HOME with HOME fallback but plist only sets
  HOME
- **Fix:** Document path difference or inherit full env
- [x] Resolved

### C56 — E013 Blocked on Incomplete E012

- **Files:** E013, E012
- **Issue:** E013 lists E012 as dependency, E012 is "In Progress"
- **Fix:** Clarify hard vs soft dependency
- [x] Resolved

### C57 — Platform Detection Fallback Missing

- **Files:** S013.03
- **Issue:** Uses `#[cfg(target_os)]` but no unsupported platform handling
- **Fix:** Add "platform not supported" error
- [x] Resolved

### C58 — No Uninstall Rollback Testing

- **Files:** S013.01, S013.02
- **Issue:** Manual tests don't verify clean uninstall
- **Fix:** Add full install→uninstall test cycle
- [x] Resolved

### C59 — Binary Installation Path Not Covered

- **Files:** All assume `/usr/local/bin/acd`
- **Issue:** No story covers getting binary to install path
- **Fix:** Add story or expand S013.04
- [x] Resolved

### C60 — Resource File Embedding Won't Work After cargo install

- **Files:** S013.03 line 138
- **Issue:** `std::fs::copy("resources/...")` won't work after `cargo install`
- **Fix:** Use `include_str!` macro
- [x] Resolved

---

## Cluster 10: Daemon Lifecycle & Auto-Stop

**Severity:** High **Affects:** E001, E004, E005

### C61 — Daemon Auto-Stop with Active TUI Subscribers

- **Files:** D3, D5
- **Issue:** D3: "only fetches usage when ≥1 TUI subscribed". D5: "auto-stop
  after 60 minutes idle". If TUI is subscribed, is daemon "idle"?
- **Fix:** Clarify daemon NOT idle if any TUI subscribed
- [x] Resolved

### C62 — Auto-Stop 30→60 Not Updated Everywhere

- **Files:** D5 (60 min / 3600s), Q25
- **Issue:** Q25 still shows 1800s
- **Fix:** Update Q25
- [x] Resolved

### C63 — Auto-Stop Constants Not in Any Story

- **Files:** D5 Rust constants, epics/stories
- **Issue:** D5 shows Rust constants but not referenced in epics or stories
- **Fix:** Add to E001 or remove from arch doc
- [x] Resolved

### C64 — SIGHUP Handler Missing from AC

- **Files:** S001.01 line 141, S001.01 line 59
- **Issue:** Signal table shows "SIGHUP | Reload configuration" but AC only
  checks SIGTERM/SIGINT
- **Fix:** Add SIGHUP to AC or note deferred to E007
- [x] Resolved

### C65 — Auto-Start Timeout Math Doesn't Add Up

- **Files:** S001.04 line 161, S001.04 line 135
- **Issue:** "Total max wait | ~5 seconds" but math gives ~2630ms
- **Fix:** Fix math or adjust values
- [x] Resolved

---

## Cluster 11: TUI Architecture Gaps

**Severity:** Medium **Affects:** E004

### C66 — TUI Reconnection Strategy Incomplete

- **Files:** E004 lines 144-153
- **Issue:** "exponential backoff (100ms → 5s max)" but no exact schedule, no
  max retry count
- **Fix:** Specify schedule and whether retry is indefinite
- [x] Resolved

### C67 — Session Detail View Missing E009 Dependency

- **Files:** S004.04 lines 49-59
- **Issue:** Shows API usage display but dependencies don't include E009
- **Fix:** Add E009 as dependency or note usage display is optional
- [x] Resolved

### C68 — View State Enum Help Variant Unimplemented

- **Files:** S004.04 lines 122-135, S004.03 line 107
- **Issue:** `enum View { Dashboard, Detail, Help }` and `?` for help, but no
  story implements Help view
- **Fix:** Create story or remove from View enum
- [x] Resolved

### C69 — tokio::select! Event Loop Split Unclear

- **Files:** S004.01 lines 154-176, S004.02 lines 188-205
- **Issue:** Unclear which story implements the daemon arm of the event loop
- **Fix:** Clarify S004.01 scaffolds, S004.02 adds daemon arm
- [x] Resolved

### C70 — Terminal Restoration on Panic: CI Testing Unclear

- **Files:** S004.01 lines 68-69, S004.01 lines 83-84
- **Issue:** "spawn subprocess, force panic" — how to verify in CI?
- **Fix:** Clarify manual verification, out of scope for CI
- [x] Resolved

### C71 — Project Structure Path Notation Inconsistency

- **Files:** S004.01: `src/tui/`, E004:
  `crates/agent-console-dashboard/src/tui/`
- **Issue:** Relative vs full path in different docs
- **Fix:** Standardize notation
- [x] Resolved

---

## Cluster 12: Keyboard Shortcuts & Layout

**Severity:** Medium **Affects:** E004, E005

### C72 — Layout Presets Keyboard Shortcut Scope Conflict

- **Files:** S005.05 lines 73-77, E004 lines 89-98, S004.03 lines 103-108
- **Issue:** S005.05 says "main dashboard view" only, E004 says "Context: Any",
  S004.03 has no restriction
- **Fix:** Standardize, recommend main view only
- [x] Resolved

---

## Cluster 13: Zellij & Terminal Integration

**Severity:** Medium **Affects:** E010

### C73 — S010.02 and S010.03 Overlapping Responsibilities

- **Files:** S010.02, S010.03
- **Issue:** Zellij-specific pane management vs terminal abstraction layer —
  relationship unclear
- **Fix:** Clarify S010.02 consumes S010.03, add explicit dependency
- [x] Resolved

### C74 — S010.01 Missing Dependency on S005.05

- **Files:** S010.01 lines 43-44
- **Issue:** Depends on S005.05 layout presets which may not be implemented
- **Fix:** Verify S005.05 exists
- [x] Resolved

### C75 — Zellij CLI Version Compatibility Not Specified

- **Files:** E010 lines 102-103
- **Issue:** "Tested with Zellij 0.39.x+" but stories don't specify minimum
  version
- **Fix:** Add minimum version to S010.01 AC
- [x] Resolved

### C76 — Duplicate Zellij Environment Detection

- **Files:** S010.02 lines 82-85, S010.03 lines 84-93
- **Issue:** `is_inside_zellij()` in S010.02 and `TerminalEnvironment::detect()`
  in S010.03
- **Fix:** Remove from S010.02, use S010.03
- [x] Resolved

### C77 — tmux Variant in Enum but Deferred

- **Files:** S010.03, Q9
- **Issue:** `Tmux` variant in TerminalEnvironment but Q9 says "On request only"
- **Fix:** Remove Tmux variant or mark placeholder
- [x] Resolved

### C78 — No Testing Strategy for Zellij Features

- **Files:** All E010 stories
- **Issue:** Manual test only, no mock/integration strategy
- **Fix:** Accept manual or add future plan
- [x] Resolved

---

## Cluster 14: claude-usage Crate (E011) Issues

**Severity:** Medium **Affects:** E011

### C79 — E011 AC Requires npm Package but S011.07 is Deferred

- **Files:** E011 lines 64-65, S011.07
- **Issue:** "npm package available via napi-rs" in AC but S011.07 is Deferred
- **Fix:** Remove npm AC or mark as "Deferred"
- [x] Resolved

### C80 — S011.02 Security Notes Contradict Implementation

- **Files:** S011.02 lines 118-121, S011.02 lines 153-157
- **Issue:** Returns `Ok(token.to_string())` but says "No storage beyond
  function scope". Token exists in memory during call chain
- **Fix:** Clarify security notes
- [x] Resolved

### C81 — S011.03 Env Var Override Not in Epic

- **Files:** S011.03 lines 19-20, E011
- **Issue:** `CLAUDE_CODE_OAUTH_TOKEN` override in story but not in epic
- **Fix:** Add to E011 credential sources
- [x] Resolved

### C82 — S011.04 Blocking Client but Daemon is Async

- **Files:** S011.04 line 21
- **Issue:** Uses blocking reqwest but daemon is tokio-based, needs
  spawn_blocking
- **Fix:** Document why blocking chosen, note async needed later
- [x] Resolved

### C83 — Credential Expiration Overflow

- **Files:** S011.02 lines 105-110
- **Issue:** `as_millis() as i64` — u128 → i64 cast technically unsound
- **Fix:** Use `.as_secs()` or `.try_into()`
- [x] Resolved

### C84 — S011.05 Optional Fields vs API Response

- **Files:** S011.05 lines 87-91, E011 lines 79-86
- **Issue:** seven_day_sonnet, extra_usage as Option but E011 says API always
  includes them
- **Fix:** Verify actual API behavior
- [x] Resolved

### C85 — Error Messages Reference Wrong CLI Command

- **Files:** S011.02 line 131
- **Issue:** "Run `claude` to login" — may be `claude-code` not `claude`
- **Fix:** Verify correct command name
- [x] Resolved

### C86 — Workspace Structure: Crate Name ≠ Binary Name

- **Files:** S011.01 lines 30-48, S011.01 lines 108-117
- **Issue:** Binary name "acd" not explained relative to crate name
- **Fix:** Add note referencing Q12
- [x] Resolved

### C87 — macOS Keychain ACL: Implementation vs Recommendation Conflict

- **Files:** S011.02, macos-keychain-acl.md
- **Issue:** S011.02 uses security-framework crate (will prompt); recommendation
  doc says use /usr/bin/security CLI
- **Fix:** Either change implementation or document first-run prompt
- [x] Resolved

---

## Cluster 15: Health Check & Diagnostics

**Severity:** Medium **Affects:** E012

### C88 — Socket Path Hardcoded vs Configurable

- **Files:** S012.02 line 50, E007
- **Issue:** socket_path in output but E007 not done yet
- **Fix:** Hardcode in v0, note E007 dependency
- [x] Resolved

### C89 — Health Check Memory Display Logic Missing

- **Files:** S012.02
- **Issue:** `memory_mb: Option<f64>` but no formatting spec for "N/A" vs "2.1
  MB"
- **Fix:** Add display formatting
- [x] Resolved

### C90 — Duplicate SessionCounts Struct

- **Files:** S012.02, S012.03
- **Issue:** Both define identical SessionCounts
- **Fix:** Share type
- [x] Resolved

### C91 — Service Status vs Health Check Output Formats Differ

- **Files:** S013.03, S012.02
- **Issue:** "Service status: running (via launchd)" vs "Status: running" —
  unclear if intentional
- **Fix:** Clarify different purposes
- [x] Resolved

### C92 — S012.03 Depends on S012.02

- **Files:** S012.03 lines 71-73
- **Issue:** Both are Draft; dependency may delay
- **Fix:** Confirm or remove dependency
- [x] Resolved

---

## Cluster 16: Module Visibility & Code Organization

**Severity:** Medium **Affects:** E003, E007, E008

### C93 — Client Module Visibility Confusion

- **Files:** S003.06 line 71, S003.06 line 9
- **Issue:** "may need to be `pub(crate)`" vs "ensure client module is not
  accidentally exposed as public API" — seem contradictory
- **Fix:** Clarify pub(crate) is acceptable, concern is only about pub exports
- [x] Resolved

### C94 — S007.02 Missing Integration Point for xdg Module

- **Files:** S007.02 line 161
- **Issue:** `xdg::config_path()` without module path
- **Fix:** Add `use crate::config::xdg;` to code examples
- [x] Resolved

### C95 — S007.02 SIGHUP Handler Integration Point Missing

- **Files:** S007.02 line 53
- **Issue:** Creates `daemon/reload.rs` but no mention of where handler is
  called from main loop
- **Fix:** Add integration point
- [x] Resolved

### C96 — S008.02 Missing ErrorCode Enum

- **Files:** S008.02 lines 121-125, S008.02 line 145
- **Issue:** Error codes listed and `ErrorCode::SessionNotFound` used but enum
  never defined
- **Fix:** Add enum definition or reference S003.01
- [x] Resolved

---

## Cluster 17: Stale/Removed Story References

**Severity:** Medium **Affects:** E008, E009

### C97 — S008.03 Moved but Still Listed in E008

- **Files:** E008 line 33
- **Issue:** Lists S008.03 with status "Moved"
- **Fix:** Remove from main table or move to "Removed" section
- [x] Resolved

### C98 — S009.02 Cut but Still Listed in E009

- **Files:** E009 line 29
- **Issue:** Lists S009.02 with status "Cut"
- **Fix:** Remove from main table or move to "Removed" section
- [x] Resolved

---

## Cluster 18: Cross-Reference & Tracking Gaps

**Severity:** Medium **Affects:** E003, E010, E011

### C99 — API_USAGE Command Mentioned but Missing

- **Files:** S003.05 line 59
- **Issue:** Note says handled by E011, but no command exists. May confuse
  readers
- **Fix:** Explicitly state there is NO api-usage CLI command
- [x] Resolved

### C100 — D3 "Temporary Decision" Not Tracked

- **Files:** D3 lines 79-82
- **Issue:** "Temporary decision; revisit in P3 issue" — no P3 issue referenced
- **Fix:** Create issue or remove "temporary"
- [x] Resolved

### C101 — D4 Timer Alignment Issue Not Tracked

- **Files:** D4 lines 96-99
- **Issue:** "Tracked as separate P3 investigation issue" — no issue referenced
- **Fix:** Create issue or defer explicitly
- [x] Resolved

### C102 — Concurrency.md References Missing Amendment

- **Files:** concurrency.md line 123
- **Issue:** References Amendment 2; file at `2026-01-31-decision-amendments.md`
- **Fix:** Verify reference works
- [x] Resolved

### C103 — E010 and E011 Don't Cross-Reference

- **Files:** E010, E011
- **Issue:** Related epics with no links between them
- **Fix:** Add cross-references
- [x] Resolved

### C104 — Complexity Review Resolutions Unclear

- **Files:** S003.01 line 200, E002 line 150
- **Issue:** "Address these during implementation" — no tracking mechanism
- **Fix:** Create follow-up tracking or confirm resolution
- [x] Resolved

---

## Cluster 19: Minor Consistency Issues

**Severity:** Low **Affects:** E001, E003, E005, E011

### C105 — Socket Path Configuration Minor Inconsistency

- **Files:** S001.01, S001.02, S001.04, E001 line 94
- **Issue:** All show `/tmp/agent-console.sock`, E001 shows as option.
  Consistent but could be clearer
- **Fix:** Minor, no action required unless consolidating config
- [x] Resolved

### C106 — Inconsistent Terminology: "Usage" vs "API Usage"

- **Files:** E009, S009.01
- **Issue:** E009 says "API Usage Tracking", S009.01 says "claude-usage crate",
  "usage data"
- **Fix:** Minor, acceptable given context
- [x] Resolved

### C107 — Daemon State Field: usage vs api_usage

- **Files:** S009.01 line 110, widget name "api-usage"
- **Issue:** Field named `usage` but widget named `api-usage`
- **Fix:** Minor consistency question
- [x] Resolved

### C108 — Binary Size Not Tracked

- **Files:** E011
- **Issue:** E011 discusses size but no limits set
- **Fix:** Add measurement or accept informal
- [x] Resolved

### C109 — Widget min_width Retracted Concern

- **Files:** Agent 2 report item 20
- **Issue:** Originally flagged but retracted as actually consistent
- **Fix:** No action needed
- [x] Resolved

### C110 — WidgetContext Field Consistency Retracted

- **Files:** Agent 3 report item 18
- **Issue:** Originally flagged but retracted as actually consistent
- **Fix:** No action needed
- [x] Resolved

### C111 — S010.01 Session ID basename Pattern Audit

- **Files:** D8 line 167, E010 stories
- **Issue:** Stale basename pattern may persist in E010 stories (overlaps with
  C47)
- **Fix:** Covered by C47 audit
- [x] Resolved

### C112 — S008.01 Stale Pattern Warning Cross-Check

- **Files:** S008.01 line 198
- **Issue:** Warns about stale basename pattern, suggests checking other epics
- **Fix:** Covered by C47 audit
- [x] Resolved
