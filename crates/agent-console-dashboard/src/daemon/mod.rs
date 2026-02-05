//! Daemon module for the Agent Console Dashboard.
//!
//! This module provides process lifecycle management, daemonization, and the
//! main entry point for running the daemon.

pub mod logging;
pub mod server;
pub mod session;
pub mod store;
pub mod usage;

// Re-export commonly used types for convenience
pub use server::SocketServer;
pub use store::SessionStore;

use crate::DaemonConfig;
use fork::{daemon, Fork};
use std::error::Error;
use tokio::runtime::Runtime;
use tokio::signal;
use tokio::signal::unix::{signal as unix_signal, SignalKind};
use tracing::{error, info, warn};
use claude_hooks::{HookEvent, HookHandler};

/// Result type alias for daemon operations.
pub type DaemonResult<T> = Result<T, Box<dyn Error>>;

/// Wait for a shutdown signal (SIGINT or SIGTERM).
///
/// This async function blocks until either Ctrl+C (SIGINT) or SIGTERM
/// is received, enabling graceful shutdown of the daemon.
///
/// If SIGTERM handler registration fails, falls back to SIGINT only
/// with a warning message.
async fn wait_for_shutdown() {
    match unix_signal(SignalKind::terminate()) {
        Ok(mut sigterm) => {
            tokio::select! {
                _ = signal::ctrl_c() => {
                    info!("received SIGINT (Ctrl+C), shutting down");
                },
                _ = sigterm.recv() => {
                    info!("received SIGTERM, shutting down");
                },
            }
        }
        Err(e) => {
            warn!(error = %e, "could not register SIGTERM handler, using SIGINT only");
            if let Err(e) = signal::ctrl_c().await {
                error!(error = %e, "failed waiting for SIGINT");
            } else {
                info!("received SIGINT (Ctrl+C), shutting down");
            }
        }
    }
}

/// Daemonize the current process.
///
/// This function forks the process and detaches it from the terminal.
/// The parent process exits immediately with code 0, and the child
/// continues execution as a background daemon.
///
/// # Arguments
///
/// * `nochdir` - If false, changes the working directory to `/`.
///   If true, keeps the current working directory.
/// * `noclose` - If false, redirects stdin/stdout/stderr to /dev/null.
///   If true, keeps the standard file descriptors.
///
/// # Returns
///
/// * `Ok(())` - On success (in the child process)
/// * `Err(...)` - If the fork operation fails
///
/// # Note
///
/// This function MUST be called BEFORE starting the Tokio runtime,
/// as forking after Tokio initialization corrupts global state for
/// signal handling.
pub fn daemonize_process(nochdir: bool, noclose: bool) -> DaemonResult<()> {
    match daemon(nochdir, noclose) {
        Ok(Fork::Child) => {
            // Daemon child process continues here
            Ok(())
        }
        Ok(Fork::Parent(_)) => {
            // Parent exits immediately
            std::process::exit(0);
        }
        Err(e) => Err(Box::new(std::io::Error::other(format!(
            "Failed to daemonize: {}",
            e
        )))),
    }
}

/// Clean up any existing ACD hooks from settings.json.
///
/// This ensures a clean state even if the daemon crashed previously
/// and failed to uninstall hooks. All hooks installed by "acd" are removed.
///
/// Errors are logged but do not fail the operation.
fn cleanup_existing_acd_hooks() {
    match claude_hooks::list() {
        Ok(entries) => {
            for entry in entries {
                if entry.managed {
                    if let Some(metadata) = &entry.metadata {
                        if metadata.installed_by == "acd" {
                            info!("cleaning up existing ACD hook: {:?}", entry.event);
                            if let Err(e) = claude_hooks::uninstall(entry.event, &entry.handler.command) {
                                warn!(
                                    error = %e,
                                    event = ?entry.event,
                                    "failed to clean up existing hook (will retry install)"
                                );
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            warn!(error = %e, "failed to list hooks for cleanup (continuing anyway)");
        }
    }
}

/// Install ACD hooks into Claude Code settings.json.
///
/// Installs three hooks:
/// - Stop hook: Notifies daemon when Claude Code stops
/// - SessionStart hook: Notifies daemon when Claude Code starts
/// - UserPromptSubmit hook: Tracks prompt submissions
///
/// Layer 1 safety: Lists existing ACD hooks first and only installs missing ones.
/// Layer 2 safety: claude-hooks crate checks registry before installing.
///
/// Hook script paths are determined relative to the binary location.
/// Errors are logged but do not fail the operation.
fn install_acd_hooks() {
    // Layer 1: Check which ACD hooks are already installed
    let existing_acd_hooks: Vec<HookEvent> = match claude_hooks::list() {
        Ok(entries) => entries
            .iter()
            .filter(|e| {
                e.managed
                    && e.metadata
                        .as_ref()
                        .is_some_and(|m| m.installed_by == "acd")
            })
            .map(|e| e.event)
            .collect(),
        Err(e) => {
            warn!(error = %e, "failed to list hooks, will attempt fresh install");
            Vec::new()
        }
    };

    // If all 3 hooks already exist, skip installation
    let has_stop = existing_acd_hooks.contains(&HookEvent::Stop);
    let has_start = existing_acd_hooks.contains(&HookEvent::SessionStart);
    let has_prompt = existing_acd_hooks.contains(&HookEvent::UserPromptSubmit);

    if has_stop && has_start && has_prompt {
        info!("all ACD hooks already installed, skipping");
        return;
    }

    // Determine hook script directory
    let hooks_dir = match std::env::current_exe() {
        Ok(exe_path) => exe_path
            .parent()
            .expect("binary should have parent directory")
            .join("hooks"),
        Err(e) => {
            error!(error = %e, "failed to determine executable path, cannot install hooks");
            return;
        }
    };

    info!(hooks_dir = %hooks_dir.display(), "installing ACD hooks");

    // Install Stop hook (if missing)
    if !has_stop {
        let stop_hook = HookHandler {
            r#type: "command".to_string(),
            command: format!("{}/stop.sh", hooks_dir.display()),
            timeout: Some(600),
            r#async: None,
            status_message: None,
        };

        match claude_hooks::install(HookEvent::Stop, stop_hook, None, "acd") {
            Ok(_) => info!("installed Stop hook"),
            Err(e) => error!(error = %e, "failed to install Stop hook"),
        }
    }

    // Install SessionStart hook (if missing)
    if !has_start {
        let start_hook = HookHandler {
            r#type: "command".to_string(),
            command: format!("{}/start.sh", hooks_dir.display()),
            timeout: Some(600),
            r#async: None,
            status_message: None,
        };

        match claude_hooks::install(HookEvent::SessionStart, start_hook, None, "acd") {
            Ok(_) => info!("installed SessionStart hook"),
            Err(e) => error!(error = %e, "failed to install SessionStart hook"),
        }
    }

    // Install UserPromptSubmit hook (if missing)
    if !has_prompt {
        let prompt_hook = HookHandler {
            r#type: "command".to_string(),
            command: format!("{}/user-prompt-submit.sh", hooks_dir.display()),
            timeout: Some(600),
            r#async: None,
            status_message: None,
        };

        match claude_hooks::install(HookEvent::UserPromptSubmit, prompt_hook, None, "acd") {
            Ok(_) => info!("installed UserPromptSubmit hook"),
            Err(e) => error!(error = %e, "failed to install UserPromptSubmit hook"),
        }
    }
}

/// Uninstall ACD hooks from Claude Code settings.json.
///
/// Removes all three hooks installed during startup.
/// Errors are logged but do not fail the operation.
fn uninstall_acd_hooks() {
    // Determine hook script directory
    let hooks_dir = match std::env::current_exe() {
        Ok(exe_path) => exe_path
            .parent()
            .expect("binary should have parent directory")
            .join("hooks"),
        Err(e) => {
            error!(error = %e, "failed to determine executable path, cannot uninstall hooks");
            return;
        }
    };

    info!(hooks_dir = %hooks_dir.display(), "uninstalling ACD hooks");

    // Uninstall Stop hook
    let stop_cmd = format!("{}/stop.sh", hooks_dir.display());
    if let Err(e) = claude_hooks::uninstall(HookEvent::Stop, &stop_cmd) {
        warn!(error = %e, "failed to uninstall Stop hook (may not exist)");
    } else {
        info!("uninstalled Stop hook");
    }

    // Uninstall SessionStart hook
    let start_cmd = format!("{}/start.sh", hooks_dir.display());
    if let Err(e) = claude_hooks::uninstall(HookEvent::SessionStart, &start_cmd) {
        warn!(error = %e, "failed to uninstall SessionStart hook (may not exist)");
    } else {
        info!("uninstalled SessionStart hook");
    }

    // Uninstall UserPromptSubmit hook
    let prompt_cmd = format!("{}/user-prompt-submit.sh", hooks_dir.display());
    if let Err(e) = claude_hooks::uninstall(HookEvent::UserPromptSubmit, &prompt_cmd) {
        warn!(error = %e, "failed to uninstall UserPromptSubmit hook (may not exist)");
    } else {
        info!("uninstalled UserPromptSubmit hook");
    }
}

/// Run the daemon with the given configuration.
///
/// This is the main entry point for the daemon. It performs daemonization
/// if requested, then starts the Tokio runtime and runs the main event loop.
///
/// # Arguments
///
/// * `config` - The daemon configuration containing socket path and daemonize flag.
///
/// # Returns
///
/// * `Ok(())` - On successful shutdown
/// * `Err(...)` - If the daemon fails to start or encounters an error
///
/// # Example
///
/// ```no_run
/// use agent_console::{DaemonConfig, daemon::run_daemon};
/// use std::path::PathBuf;
///
/// let config = DaemonConfig::new(
///     PathBuf::from("/tmp/agent-console.sock"),
///     false, // foreground mode
/// );
/// run_daemon(config).expect("Failed to run daemon");
/// ```
pub fn run_daemon(config: DaemonConfig) -> DaemonResult<()> {
    // CRITICAL: Daemonize BEFORE starting Tokio runtime
    // Forking after Tokio initialization corrupts global state for signal handling
    if config.daemonize {
        // Production mode: change to /, redirect stdio to /dev/null
        daemonize_process(false, false)?;
    }

    // Initialize logging after daemonize (stderr may be redirected)
    logging::init();

    info!(
        socket_path = %config.socket_path.display(),
        daemonize = config.daemonize,
        "agent console daemon starting"
    );

    // Install hooks (idempotent - skips if already exist)
    // Hooks persist across daemon restarts; only removed when ACD is uninstalled
    install_acd_hooks();

    // Create Tokio runtime AFTER daemonization
    // Using current_thread runtime for simpler daemon workloads
    let runtime = Runtime::new().map_err(|e| {
        Box::new(std::io::Error::other(format!(
            "Failed to create Tokio runtime: {}",
            e
        ))) as Box<dyn Error>
    })?;

    info!("daemon running, press Ctrl+C or send SIGTERM to stop");

    // Run the main event loop
    runtime.block_on(async {
        let mut server = SocketServer::new(config.socket_path.display().to_string());
        if let Err(e) = server.start().await {
            error!("failed to start socket server: {}", e);
            return;
        }

        let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);

        // Spawn the accept loop
        let server_handle = tokio::spawn(async move {
            if let Err(e) = server.run_with_shutdown(shutdown_rx).await {
                error!("socket server error: {}", e);
            }
        });

        // Wait for shutdown signal
        wait_for_shutdown().await;

        // Signal server to stop
        let _ = shutdown_tx.send(());
        let _ = server_handle.await;
    });

    // Hooks remain installed after daemon stops (intentional)
    // Use `acd uninstall-hooks` to remove them when removing ACD entirely

    info!("daemon stopped");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::env;
    use std::fs;
    use serial_test::serial;

    #[test]
    fn test_daemonize_process_returns_result() {
        // We can't actually test fork in a unit test, but we can verify
        // the function signature and that DaemonResult is properly defined
        let _result: DaemonResult<()> = Ok(());
    }

    #[test]
    fn test_run_daemon_returns_result() {
        // We can't easily test run_daemon as it may daemonize,
        // but we can verify the function accepts DaemonConfig
        // Note: This test only verifies type compatibility
        let _config = DaemonConfig::new(PathBuf::from("/tmp/test.sock"), false);
    }

    #[test]
    fn test_daemon_config_used_correctly() {
        // Verify DaemonConfig fields are accessible
        let config = DaemonConfig::new(PathBuf::from("/tmp/test.sock"), true);
        assert!(config.daemonize);
        assert_eq!(config.socket_path, PathBuf::from("/tmp/test.sock"));
    }

    #[test]
    #[serial(home)]
    fn test_cleanup_existing_acd_hooks_handles_no_hooks() {
        // Setup test environment
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        env::set_var("HOME", temp_dir.path());

        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).expect("failed to create .claude dir");

        let settings = serde_json::json!({
            "hooks": {},
            "cleanupPeriodDays": 7
        });
        fs::write(
            claude_dir.join("settings.json"),
            serde_json::to_string_pretty(&settings).expect("failed to serialize settings"),
        )
        .expect("failed to write settings.json");

        // Should not panic when no hooks exist
        cleanup_existing_acd_hooks();
    }

    #[test]
    #[serial(home)]
    fn test_cleanup_existing_acd_hooks_removes_acd_hooks() {
        // Setup test environment
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        env::set_var("HOME", temp_dir.path());

        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).expect("failed to create .claude dir");

        let settings = serde_json::json!({
            "hooks": {},
            "cleanupPeriodDays": 7
        });
        fs::write(
            claude_dir.join("settings.json"),
            serde_json::to_string_pretty(&settings).expect("failed to serialize settings"),
        )
        .expect("failed to write settings.json");

        // Install a hook using claude-hooks library
        let handler = HookHandler {
            r#type: "command".to_string(),
            command: "/tmp/test-cleanup.sh".to_string(),
            timeout: Some(600),
            r#async: None,
            status_message: None,
        };
        claude_hooks::install(HookEvent::Stop, handler, None, "acd")
            .expect("failed to install test hook");

        // Verify hook exists
        let entries = claude_hooks::list().expect("failed to list hooks");
        assert_eq!(entries.len(), 1, "should have 1 hook before cleanup");

        // Run cleanup
        cleanup_existing_acd_hooks();

        // Verify hook removed
        let entries = claude_hooks::list().expect("failed to list hooks after cleanup");
        assert_eq!(entries.len(), 0, "should have 0 hooks after cleanup");
    }

    #[test]
    #[serial(home)]
    fn test_cleanup_existing_acd_hooks_preserves_non_acd_hooks() {
        // Setup test environment
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        env::set_var("HOME", temp_dir.path());

        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).expect("failed to create .claude dir");

        let settings = serde_json::json!({
            "hooks": {},
            "cleanupPeriodDays": 7
        });
        fs::write(
            claude_dir.join("settings.json"),
            serde_json::to_string_pretty(&settings).expect("failed to serialize settings"),
        )
        .expect("failed to write settings.json");

        // Install an ACD hook
        let acd_handler = HookHandler {
            r#type: "command".to_string(),
            command: "/tmp/acd-hook.sh".to_string(),
            timeout: Some(600),
            r#async: None,
            status_message: None,
        };
        claude_hooks::install(HookEvent::Stop, acd_handler, None, "acd")
            .expect("failed to install acd hook");

        // Install a non-ACD hook
        let other_handler = HookHandler {
            r#type: "command".to_string(),
            command: "/tmp/other-hook.sh".to_string(),
            timeout: Some(300),
            r#async: None,
            status_message: None,
        };
        claude_hooks::install(HookEvent::SessionStart, other_handler, None, "other-app")
            .expect("failed to install other hook");

        // Verify both hooks exist
        let entries = claude_hooks::list().expect("failed to list hooks");
        assert_eq!(entries.len(), 2, "should have 2 hooks before cleanup");

        // Run cleanup
        cleanup_existing_acd_hooks();

        // Verify only ACD hook removed
        let entries = claude_hooks::list().expect("failed to list hooks after cleanup");
        assert_eq!(entries.len(), 1, "should have 1 hook after cleanup");
        assert_eq!(entries[0].handler.command, "/tmp/other-hook.sh");
        assert_eq!(
            entries[0].metadata.as_ref().expect("should have metadata").installed_by,
            "other-app"
        );
    }

    #[test]
    #[serial(home)]
    fn test_install_acd_hooks_installs_three_hooks() {
        // Setup test environment
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        env::set_var("HOME", temp_dir.path());

        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).expect("failed to create .claude dir");

        let settings = serde_json::json!({
            "hooks": {},
            "cleanupPeriodDays": 7
        });
        fs::write(
            claude_dir.join("settings.json"),
            serde_json::to_string_pretty(&settings).expect("failed to serialize settings"),
        )
        .expect("failed to write settings.json");

        // Run install
        install_acd_hooks();

        // Verify 3 hooks installed
        let entries = claude_hooks::list().expect("failed to list hooks");
        assert_eq!(entries.len(), 3, "should have 3 hooks installed");

        // Verify all hooks are managed by "acd"
        for entry in &entries {
            assert!(entry.managed, "hook should be managed");
            assert_eq!(
                entry.metadata.as_ref().expect("should have metadata").installed_by,
                "acd"
            );
        }

        // Verify all three events are present
        let events: Vec<HookEvent> = entries.iter().map(|e| e.event).collect();
        assert!(events.contains(&HookEvent::Stop), "should have Stop hook");
        assert!(events.contains(&HookEvent::SessionStart), "should have SessionStart hook");
        assert!(events.contains(&HookEvent::UserPromptSubmit), "should have UserPromptSubmit hook");
    }

    #[test]
    #[serial(home)]
    fn test_install_acd_hooks_is_idempotent() {
        // Setup test environment
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        env::set_var("HOME", temp_dir.path());

        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).expect("failed to create .claude dir");

        let settings = serde_json::json!({
            "hooks": {},
            "cleanupPeriodDays": 7
        });
        fs::write(
            claude_dir.join("settings.json"),
            serde_json::to_string_pretty(&settings).expect("failed to serialize settings"),
        )
        .expect("failed to write settings.json");

        // First install
        install_acd_hooks();
        let entries1 = claude_hooks::list().expect("failed to list hooks after first install");
        assert_eq!(entries1.len(), 3, "should have 3 hooks after first install");

        // Second install should not panic or duplicate
        install_acd_hooks();
        let entries2 = claude_hooks::list().expect("failed to list hooks after second install");
        assert_eq!(entries2.len(), 3, "should still have 3 hooks after second install (no duplicates)");
    }

    #[test]
    #[serial(home)]
    fn test_uninstall_acd_hooks_removes_all_hooks() {
        // Setup test environment
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        env::set_var("HOME", temp_dir.path());

        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).expect("failed to create .claude dir");

        let settings = serde_json::json!({
            "hooks": {},
            "cleanupPeriodDays": 7
        });
        fs::write(
            claude_dir.join("settings.json"),
            serde_json::to_string_pretty(&settings).expect("failed to serialize settings"),
        )
        .expect("failed to write settings.json");

        // Install hooks first
        install_acd_hooks();
        let entries = claude_hooks::list().expect("failed to list hooks");
        assert_eq!(entries.len(), 3, "should have 3 hooks installed");

        // Uninstall all hooks
        uninstall_acd_hooks();

        // Verify all hooks removed
        let entries = claude_hooks::list().expect("failed to list hooks after uninstall");
        assert_eq!(entries.len(), 0, "should have 0 hooks after uninstall");
    }

    #[test]
    #[serial(home)]
    fn test_uninstall_acd_hooks_is_idempotent() {
        // Setup test environment
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        env::set_var("HOME", temp_dir.path());

        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).expect("failed to create .claude dir");

        let settings = serde_json::json!({
            "hooks": {},
            "cleanupPeriodDays": 7
        });
        fs::write(
            claude_dir.join("settings.json"),
            serde_json::to_string_pretty(&settings).expect("failed to serialize settings"),
        )
        .expect("failed to write settings.json");

        // Install hooks
        install_acd_hooks();

        // First uninstall
        uninstall_acd_hooks();
        let entries1 = claude_hooks::list().expect("failed to list hooks after first uninstall");
        assert_eq!(entries1.len(), 0, "should have 0 hooks after first uninstall");

        // Second uninstall should not panic
        uninstall_acd_hooks();
        let entries2 = claude_hooks::list().expect("failed to list hooks after second uninstall");
        assert_eq!(entries2.len(), 0, "should still have 0 hooks after second uninstall");
    }

    #[test]
    #[serial(home)]
    fn test_hook_commands_use_correct_paths() {
        // Setup test environment
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        env::set_var("HOME", temp_dir.path());

        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).expect("failed to create .claude dir");

        let settings = serde_json::json!({
            "hooks": {},
            "cleanupPeriodDays": 7
        });
        fs::write(
            claude_dir.join("settings.json"),
            serde_json::to_string_pretty(&settings).expect("failed to serialize settings"),
        )
        .expect("failed to write settings.json");

        // Install hooks
        install_acd_hooks();

        // List and verify hook commands contain correct paths
        let entries = claude_hooks::list().expect("failed to list hooks");

        for entry in &entries {
            let cmd = &entry.handler.command;

            // Verify command contains /hooks/ directory
            assert!(cmd.contains("/hooks/"), "hook command should reference hooks directory: {}", cmd);

            // Verify command does NOT contain stale shell variable placeholders
            // Claude Code passes data via JSON stdin, not shell variables
            assert!(!cmd.contains("$SESSION_ID"), "hook command should not have $SESSION_ID (data comes via JSON stdin): {}", cmd);
            assert!(!cmd.contains("$ARGS"), "hook command should not have $ARGS (data comes via JSON stdin): {}", cmd);

            // Verify command ends with correct script based on event
            match entry.event {
                HookEvent::Stop => assert!(cmd.contains("stop.sh"), "Stop hook should call stop.sh"),
                HookEvent::SessionStart => assert!(cmd.contains("start.sh"), "SessionStart hook should call start.sh"),
                HookEvent::UserPromptSubmit => assert!(cmd.contains("user-prompt-submit.sh"), "UserPromptSubmit hook should call user-prompt-submit.sh"),
                _ => panic!("unexpected hook event: {:?}", entry.event),
            }

            // Verify timeout is 600 seconds
            assert_eq!(entry.handler.timeout, Some(600), "hook timeout should be 600 seconds");
        }
    }

    #[test]
    #[serial(home)]
    fn test_install_acd_hooks_handles_missing_settings() {
        // Setup test environment with no settings.json
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        env::set_var("HOME", temp_dir.path());

        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).expect("failed to create .claude dir");

        // Don't create settings.json - should handle gracefully
        install_acd_hooks();

        // Verify no panic occurred (function completed)
        // Note: Hooks won't actually be installed due to missing settings,
        // but function should log error and continue
    }

    #[test]
    #[serial(home)]
    fn test_uninstall_acd_hooks_handles_missing_settings() {
        // Setup test environment with no settings.json
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        env::set_var("HOME", temp_dir.path());

        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).expect("failed to create .claude dir");

        // Don't create settings.json - should handle gracefully
        uninstall_acd_hooks();

        // Verify no panic occurred (function completed)
    }

    #[test]
    #[serial(home)]
    fn test_layer1_skips_when_all_hooks_exist() {
        // Setup
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        env::set_var("HOME", temp_dir.path());
        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).expect("failed to create .claude dir");
        fs::write(
            claude_dir.join("settings.json"),
            r#"{"hooks": {}, "cleanupPeriodDays": 7}"#,
        )
        .expect("failed to write settings.json");

        // First install creates 3 hooks
        install_acd_hooks();
        let entries1 = claude_hooks::list().expect("failed to list hooks");
        assert_eq!(entries1.len(), 3);

        let timestamps1: Vec<String> = entries1
            .iter()
            .map(|e| e.metadata.as_ref().expect("metadata").added_at.clone())
            .collect();

        // Second install should skip (Layer 1 check)
        install_acd_hooks();
        let entries2 = claude_hooks::list().expect("failed to list hooks");
        assert_eq!(entries2.len(), 3);

        let timestamps2: Vec<String> = entries2
            .iter()
            .map(|e| e.metadata.as_ref().expect("metadata").added_at.clone())
            .collect();

        // Timestamps unchanged proves no reinstall happened
        assert_eq!(timestamps1, timestamps2, "Layer 1 should skip when all hooks exist");
    }

    #[test]
    #[serial(home)]
    fn test_layer1_installs_only_missing_hooks() {
        // Setup
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        env::set_var("HOME", temp_dir.path());
        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).expect("failed to create .claude dir");
        fs::write(
            claude_dir.join("settings.json"),
            r#"{"hooks": {}, "cleanupPeriodDays": 7}"#,
        )
        .expect("failed to write settings.json");

        // Pre-install only Stop hook (partial state)
        let stop_handler = HookHandler {
            r#type: "command".to_string(),
            command: "/some/path/hooks/stop.sh".to_string(),
            timeout: Some(600),
            r#async: None,
            status_message: None,
        };
        claude_hooks::install(HookEvent::Stop, stop_handler, None, "acd")
            .expect("failed to install Stop hook");
        assert_eq!(claude_hooks::list().expect("list").len(), 1);

        // install_acd_hooks should only add missing SessionStart and UserPromptSubmit
        install_acd_hooks();

        let entries = claude_hooks::list().expect("failed to list hooks");
        assert_eq!(entries.len(), 3);

        // Verify all three events present
        let events: Vec<HookEvent> = entries.iter().map(|e| e.event).collect();
        assert!(events.contains(&HookEvent::Stop));
        assert!(events.contains(&HookEvent::SessionStart));
        assert!(events.contains(&HookEvent::UserPromptSubmit));

        // Verify Stop hook preserved (original command, not overwritten)
        let stop_hook = entries.iter().find(|e| e.event == HookEvent::Stop).expect("Stop");
        assert_eq!(
            stop_hook.handler.command,
            "/some/path/hooks/stop.sh",
            "Stop hook should keep original command"
        );
    }

    #[test]
    #[serial(home)]
    fn test_layer2_registry_prevents_duplicate() {
        // Setup
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        env::set_var("HOME", temp_dir.path());
        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).expect("failed to create .claude dir");
        fs::write(
            claude_dir.join("settings.json"),
            r#"{"hooks": {}, "cleanupPeriodDays": 7}"#,
        )
        .expect("failed to write settings.json");

        // First install succeeds
        let handler = HookHandler {
            r#type: "command".to_string(),
            command: "/test/hook.sh".to_string(),
            timeout: Some(600),
            r#async: None,
            status_message: None,
        };
        claude_hooks::install(HookEvent::Stop, handler.clone(), None, "test")
            .expect("first install should succeed");

        // Second install blocked by Layer 2 (registry check)
        let result = claude_hooks::install(HookEvent::Stop, handler, None, "test");

        assert!(matches!(
            result,
            Err(claude_hooks::Error::Hook(claude_hooks::HookError::AlreadyExists { .. }))
        ), "Layer 2 should return AlreadyExists");

        // No duplicate created
        assert_eq!(claude_hooks::list().expect("list").len(), 1);
    }
}
