//! ACD hook management for Claude Code settings.json.
//!
//! Provides functions to install, clean up, and uninstall the three hooks
//! that connect Claude Code events to the ACD daemon:
//! - `Stop` — notifies daemon when Claude Code stops
//! - `SessionStart` — notifies daemon when a session begins
//! - `UserPromptSubmit` — tracks prompt submissions

use claude_hooks::{HookEvent, HookHandler};
use tracing::{error, info, warn};

/// Clean up any existing ACD hooks from settings.json.
///
/// This ensures a clean state even if the daemon crashed previously
/// and failed to uninstall hooks. All hooks installed by "acd" are removed.
///
/// Errors are logged but do not fail the operation.
pub(crate) fn cleanup_existing() {
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
pub(crate) fn install() {
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
pub(crate) fn uninstall() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use std::fs;

    #[test]
    #[serial(home)]
    fn test_cleanup_existing_handles_no_hooks() {
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

        cleanup_existing();
    }

    #[test]
    #[serial(home)]
    fn test_cleanup_existing_removes_acd_hooks() {
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

        let handler = HookHandler {
            r#type: "command".to_string(),
            command: "/tmp/test-cleanup.sh".to_string(),
            timeout: Some(600),
            r#async: None,
            status_message: None,
        };
        claude_hooks::install(HookEvent::Stop, handler, None, "acd")
            .expect("failed to install test hook");

        let entries = claude_hooks::list().expect("failed to list hooks");
        assert_eq!(entries.len(), 1, "should have 1 hook before cleanup");

        cleanup_existing();

        let entries = claude_hooks::list().expect("failed to list hooks after cleanup");
        assert_eq!(entries.len(), 0, "should have 0 hooks after cleanup");
    }

    #[test]
    #[serial(home)]
    fn test_cleanup_existing_preserves_non_acd_hooks() {
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

        let acd_handler = HookHandler {
            r#type: "command".to_string(),
            command: "/tmp/acd-hook.sh".to_string(),
            timeout: Some(600),
            r#async: None,
            status_message: None,
        };
        claude_hooks::install(HookEvent::Stop, acd_handler, None, "acd")
            .expect("failed to install acd hook");

        let other_handler = HookHandler {
            r#type: "command".to_string(),
            command: "/tmp/other-hook.sh".to_string(),
            timeout: Some(300),
            r#async: None,
            status_message: None,
        };
        claude_hooks::install(HookEvent::SessionStart, other_handler, None, "other-app")
            .expect("failed to install other hook");

        let entries = claude_hooks::list().expect("failed to list hooks");
        assert_eq!(entries.len(), 2, "should have 2 hooks before cleanup");

        cleanup_existing();

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
    fn test_install_installs_three_hooks() {
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

        install();

        let entries = claude_hooks::list().expect("failed to list hooks");
        assert_eq!(entries.len(), 3, "should have 3 hooks installed");

        for entry in &entries {
            assert!(entry.managed, "hook should be managed");
            assert_eq!(
                entry.metadata.as_ref().expect("should have metadata").installed_by,
                "acd"
            );
        }

        let events: Vec<HookEvent> = entries.iter().map(|e| e.event).collect();
        assert!(events.contains(&HookEvent::Stop), "should have Stop hook");
        assert!(events.contains(&HookEvent::SessionStart), "should have SessionStart hook");
        assert!(events.contains(&HookEvent::UserPromptSubmit), "should have UserPromptSubmit hook");
    }

    #[test]
    #[serial(home)]
    fn test_install_is_idempotent() {
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

        install();
        let entries1 = claude_hooks::list().expect("failed to list hooks after first install");
        assert_eq!(entries1.len(), 3, "should have 3 hooks after first install");

        install();
        let entries2 = claude_hooks::list().expect("failed to list hooks after second install");
        assert_eq!(entries2.len(), 3, "should still have 3 hooks after second install (no duplicates)");
    }

    #[test]
    #[serial(home)]
    fn test_uninstall_removes_all_hooks() {
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

        install();
        let entries = claude_hooks::list().expect("failed to list hooks");
        assert_eq!(entries.len(), 3, "should have 3 hooks installed");

        uninstall();

        let entries = claude_hooks::list().expect("failed to list hooks after uninstall");
        assert_eq!(entries.len(), 0, "should have 0 hooks after uninstall");
    }

    #[test]
    #[serial(home)]
    fn test_uninstall_is_idempotent() {
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

        install();

        uninstall();
        let entries1 = claude_hooks::list().expect("failed to list hooks after first uninstall");
        assert_eq!(entries1.len(), 0, "should have 0 hooks after first uninstall");

        uninstall();
        let entries2 = claude_hooks::list().expect("failed to list hooks after second uninstall");
        assert_eq!(entries2.len(), 0, "should still have 0 hooks after second uninstall");
    }

    #[test]
    #[serial(home)]
    fn test_hook_commands_use_correct_paths() {
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

        install();

        let entries = claude_hooks::list().expect("failed to list hooks");

        for entry in &entries {
            let cmd = &entry.handler.command;

            assert!(cmd.contains("/hooks/"), "hook command should reference hooks directory: {}", cmd);
            assert!(!cmd.contains("$SESSION_ID"), "hook command should not have $SESSION_ID (data comes via JSON stdin): {}", cmd);
            assert!(!cmd.contains("$ARGS"), "hook command should not have $ARGS (data comes via JSON stdin): {}", cmd);

            match entry.event {
                HookEvent::Stop => assert!(cmd.contains("stop.sh"), "Stop hook should call stop.sh"),
                HookEvent::SessionStart => assert!(cmd.contains("start.sh"), "SessionStart hook should call start.sh"),
                HookEvent::UserPromptSubmit => assert!(cmd.contains("user-prompt-submit.sh"), "UserPromptSubmit hook should call user-prompt-submit.sh"),
                _ => panic!("unexpected hook event: {:?}", entry.event),
            }

            assert_eq!(entry.handler.timeout, Some(600), "hook timeout should be 600 seconds");
        }
    }

    #[test]
    #[serial(home)]
    fn test_install_handles_missing_settings() {
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        env::set_var("HOME", temp_dir.path());

        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).expect("failed to create .claude dir");

        install();
    }

    #[test]
    #[serial(home)]
    fn test_uninstall_handles_missing_settings() {
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        env::set_var("HOME", temp_dir.path());

        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).expect("failed to create .claude dir");

        uninstall();
    }

    #[test]
    #[serial(home)]
    fn test_layer1_skips_when_all_hooks_exist() {
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        env::set_var("HOME", temp_dir.path());
        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).expect("failed to create .claude dir");
        fs::write(
            claude_dir.join("settings.json"),
            r#"{"hooks": {}, "cleanupPeriodDays": 7}"#,
        )
        .expect("failed to write settings.json");

        install();
        let entries1 = claude_hooks::list().expect("failed to list hooks");
        assert_eq!(entries1.len(), 3);

        let timestamps1: Vec<String> = entries1
            .iter()
            .map(|e| e.metadata.as_ref().expect("metadata").added_at.clone())
            .collect();

        install();
        let entries2 = claude_hooks::list().expect("failed to list hooks");
        assert_eq!(entries2.len(), 3);

        let timestamps2: Vec<String> = entries2
            .iter()
            .map(|e| e.metadata.as_ref().expect("metadata").added_at.clone())
            .collect();

        assert_eq!(timestamps1, timestamps2, "Layer 1 should skip when all hooks exist");
    }

    #[test]
    #[serial(home)]
    fn test_layer1_installs_only_missing_hooks() {
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        env::set_var("HOME", temp_dir.path());
        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).expect("failed to create .claude dir");
        fs::write(
            claude_dir.join("settings.json"),
            r#"{"hooks": {}, "cleanupPeriodDays": 7}"#,
        )
        .expect("failed to write settings.json");

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

        install();

        let entries = claude_hooks::list().expect("failed to list hooks");
        assert_eq!(entries.len(), 3);

        let events: Vec<HookEvent> = entries.iter().map(|e| e.event).collect();
        assert!(events.contains(&HookEvent::Stop));
        assert!(events.contains(&HookEvent::SessionStart));
        assert!(events.contains(&HookEvent::UserPromptSubmit));

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
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        env::set_var("HOME", temp_dir.path());
        let claude_dir = temp_dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).expect("failed to create .claude dir");
        fs::write(
            claude_dir.join("settings.json"),
            r#"{"hooks": {}, "cleanupPeriodDays": 7}"#,
        )
        .expect("failed to write settings.json");

        let handler = HookHandler {
            r#type: "command".to_string(),
            command: "/test/hook.sh".to_string(),
            timeout: Some(600),
            r#async: None,
            status_message: None,
        };
        claude_hooks::install(HookEvent::Stop, handler.clone(), None, "test")
            .expect("first install should succeed");

        let result = claude_hooks::install(HookEvent::Stop, handler, None, "test");

        assert!(matches!(
            result,
            Err(claude_hooks::Error::Hook(claude_hooks::HookError::AlreadyExists { .. }))
        ), "Layer 2 should return AlreadyExists");

        assert_eq!(claude_hooks::list().expect("list").len(), 1);
    }
}
