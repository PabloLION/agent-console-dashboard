//! Integration tests for the `acd claude-hook` subcommand.
//!
//! These tests exercise the full CLI flow: spawn the daemon, pipe JSON to
//! `claude-hook` stdin, then verify state via `dump`. They catch bugs that
//! unit tests miss because they test the real binary end-to-end.

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::TempDir;

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

const ACD_BIN: &str = env!("CARGO_BIN_EXE_acd");

/// RAII guard that kills the daemon process on drop (even on panic).
struct DaemonGuard {
    child: std::process::Child,
}

impl Drop for DaemonGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Returns a unique socket path inside the given temp dir.
fn test_socket(temp_dir: &TempDir) -> PathBuf {
    let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    temp_dir.path().join(format!("acd-test-{}.sock", n))
}

fn acd_cmd() -> Command {
    Command::new(ACD_BIN)
}

/// Spawn the daemon in the background, wait for the socket to appear.
fn start_daemon(socket: &Path) -> DaemonGuard {
    let child = std::process::Command::new(ACD_BIN)
        .args(["daemon", "--socket", socket.to_str().expect("valid path")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn daemon");

    // Wait for socket to appear (max ~2 s)
    for _ in 0..200 {
        if socket.exists() {
            return DaemonGuard { child };
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("daemon did not create socket within 2 s");
}

/// Run `acd dump` and parse the JSON output.
fn dump(socket: &Path) -> serde_json::Value {
    let output = acd_cmd()
        .args(["dump", "--socket", socket.to_str().expect("valid path")])
        .output()
        .expect("failed to run dump");
    let stdout = String::from_utf8(output.stdout).expect("valid utf8");
    serde_json::from_str(&stdout).expect("dump should return valid JSON")
}

/// Run `acd claude-hook <status>` with the given JSON on stdin.
fn claude_hook(socket: &Path, status: &str, stdin_json: &str) -> assert_cmd::assert::Assert {
    acd_cmd()
        .args([
            "claude-hook",
            "--socket",
            socket.to_str().expect("valid path"),
            status,
        ])
        .write_stdin(stdin_json)
        .assert()
}

// -----------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------

#[test]
fn hook_creates_session_with_working_status() {
    let tmp = TempDir::new().expect("temp dir");
    let sock = test_socket(&tmp);
    let _daemon = start_daemon(&sock);

    let hook_json = r#"{"session_id":"sess-1","cwd":"/home/user/project"}"#;
    claude_hook(&sock, "working", hook_json)
        .success()
        .stdout(predicate::str::contains(r#""continue": true"#));

    let state = dump(&sock);
    let sessions = state["sessions"].as_array().expect("sessions array");
    assert_eq!(sessions.len(), 1, "should have exactly 1 session");

    let s = &sessions[0];
    assert_eq!(s["id"], "sess-1");
    assert_eq!(s["status"], "working");
}

#[test]
fn hook_forwards_cwd_as_working_dir() {
    let tmp = TempDir::new().expect("temp dir");
    let sock = test_socket(&tmp);
    let _daemon = start_daemon(&sock);

    let hook_json = r#"{"session_id":"sess-wd","cwd":"/home/user/my-project"}"#;
    claude_hook(&sock, "working", hook_json).success();

    let state = dump(&sock);
    let s = &state["sessions"][0];
    assert_eq!(
        s["working_dir"], "/home/user/my-project",
        "working_dir should match the cwd from hook JSON stdin"
    );
}

#[test]
fn hook_without_cwd_defaults_to_unknown() {
    let tmp = TempDir::new().expect("temp dir");
    let sock = test_socket(&tmp);
    let _daemon = start_daemon(&sock);

    let hook_json = r#"{"session_id":"sess-no-cwd"}"#;
    claude_hook(&sock, "working", hook_json).success();

    let state = dump(&sock);
    let s = &state["sessions"][0];
    assert_eq!(
        s["working_dir"], "<unknown>",
        "working_dir should default to '<unknown>' when cwd is absent"
    );
}

#[test]
fn hook_transitions_status() {
    let tmp = TempDir::new().expect("temp dir");
    let sock = test_socket(&tmp);
    let _daemon = start_daemon(&sock);

    let hook_json = r#"{"session_id":"sess-tr","cwd":"/project"}"#;

    // Create with working
    claude_hook(&sock, "working", hook_json).success();
    let state = dump(&sock);
    assert_eq!(state["sessions"][0]["status"], "working");

    // Transition to attention
    claude_hook(&sock, "attention", hook_json).success();
    let state = dump(&sock);
    assert_eq!(state["sessions"][0]["status"], "attention");
}

#[test]
fn hook_multiple_sessions() {
    let tmp = TempDir::new().expect("temp dir");
    let sock = test_socket(&tmp);
    let _daemon = start_daemon(&sock);

    claude_hook(
        &sock,
        "working",
        r#"{"session_id":"multi-1","cwd":"/proj/a"}"#,
    )
    .success();
    claude_hook(
        &sock,
        "attention",
        r#"{"session_id":"multi-2","cwd":"/proj/b"}"#,
    )
    .success();

    let state = dump(&sock);
    let sessions = state["sessions"].as_array().expect("sessions array");
    assert_eq!(sessions.len(), 2, "should have 2 sessions");

    let ids: Vec<&str> = sessions
        .iter()
        .map(|s| s["id"].as_str().expect("id"))
        .collect();
    assert!(ids.contains(&"multi-1"));
    assert!(ids.contains(&"multi-2"));
}

#[test]
fn hook_invalid_json_exits_with_code_2() {
    let tmp = TempDir::new().expect("temp dir");
    let sock = test_socket(&tmp);
    let _daemon = start_daemon(&sock);

    claude_hook(&sock, "working", "not json at all")
        .code(2)
        .stderr(predicate::str::contains("failed to parse JSON"));
}

#[test]
fn hook_missing_session_id_exits_with_code_2() {
    let tmp = TempDir::new().expect("temp dir");
    let sock = test_socket(&tmp);
    let _daemon = start_daemon(&sock);

    claude_hook(&sock, "working", r#"{"cwd":"/some/path"}"#)
        .code(2)
        .stderr(predicate::str::contains("failed to parse JSON"));
}

#[test]
fn hook_unknown_fields_are_ignored() {
    let tmp = TempDir::new().expect("temp dir");
    let sock = test_socket(&tmp);
    let _daemon = start_daemon(&sock);

    let hook_json = r#"{"session_id":"sess-uf","cwd":"/proj","unknown_field":"value","extra":42}"#;
    claude_hook(&sock, "working", hook_json)
        .success()
        .stdout(predicate::str::contains(r#""continue": true"#));

    let state = dump(&sock);
    assert_eq!(state["sessions"][0]["id"], "sess-uf");
}

#[test]
fn set_closed_marks_session_closed() {
    let tmp = TempDir::new().expect("temp dir");
    let sock = test_socket(&tmp);
    let _daemon = start_daemon(&sock);

    // Create session
    claude_hook(
        &sock,
        "working",
        r#"{"session_id":"sess-cl","cwd":"/proj"}"#,
    )
    .success();

    // Close via set command
    acd_cmd()
        .args([
            "set",
            "--socket",
            sock.to_str().expect("valid path"),
            "sess-cl",
            "closed",
        ])
        .assert()
        .success();

    let state = dump(&sock);
    let s = &state["sessions"][0];
    assert_eq!(s["status"], "closed", "status should be closed");
    assert_eq!(
        s["closed"], true,
        "closed flag should be true when status is closed"
    );
}

#[test]
fn dump_session_counts_are_correct() {
    let tmp = TempDir::new().expect("temp dir");
    let sock = test_socket(&tmp);
    let _daemon = start_daemon(&sock);

    // Create 2 sessions
    claude_hook(
        &sock,
        "working",
        r#"{"session_id":"cnt-1","cwd":"/proj/a"}"#,
    )
    .success();
    claude_hook(
        &sock,
        "working",
        r#"{"session_id":"cnt-2","cwd":"/proj/b"}"#,
    )
    .success();

    let state = dump(&sock);
    assert_eq!(state["session_counts"]["active"], 2);
    assert_eq!(state["session_counts"]["closed"], 0);
}

#[test]
fn status_command_shows_running() {
    let tmp = TempDir::new().expect("temp dir");
    let sock = test_socket(&tmp);
    let _daemon = start_daemon(&sock);

    acd_cmd()
        .args(["status", "--socket", sock.to_str().expect("valid path")])
        .assert()
        .success()
        .stdout(predicate::str::contains("running"));
}

#[test]
fn status_command_not_running() {
    let tmp = TempDir::new().expect("temp dir");
    let sock = test_socket(&tmp);
    // No daemon started

    acd_cmd()
        .args(["status", "--socket", sock.to_str().expect("valid path")])
        .assert()
        .failure()
        .stdout(predicate::str::contains("not running"));
}
