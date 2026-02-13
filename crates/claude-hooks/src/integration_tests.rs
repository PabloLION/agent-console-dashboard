use super::*;
use serial_test::serial;
use std::env;
use std::fs;
use tempfile::tempdir;

/// Setup test environment with temp directory
fn setup_test_env() -> tempfile::TempDir {
    let dir = tempdir().expect("Failed to create temp directory");

    // Override HOME to point to temp directory
    env::set_var("HOME", dir.path());

    // Create .claude directory
    let claude_dir = dir.path().join(".claude");
    fs::create_dir_all(&claude_dir).expect("Failed to create .claude directory");

    // Create empty settings.json with hooks object (not array)
    let settings = serde_json::json!({
        "hooks": {},
        "cleanupPeriodDays": 7
    });
    let settings_path = claude_dir.join("settings.json");
    fs::write(
        &settings_path,
        serde_json::to_string_pretty(&settings).expect("Failed to serialize settings"),
    )
    .expect("Failed to write settings.json");

    dir
}

#[test]
#[serial(home)]
fn test_install_list_uninstall_workflow() {
    let _dir = setup_test_env();

    // Install hook
    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/stop.sh".to_string(),
        timeout: Some(600),
        r#async: None,
        status_message: None,
    };

    let result = install(HookEvent::Stop, handler.clone(), None, "test");
    assert!(result.is_ok(), "Install should succeed: {:?}", result.err());

    // List hooks - should show as managed
    let entries = list().expect("List should succeed");
    assert_eq!(entries.len(), 1, "Should have exactly 1 hook");
    assert!(entries[0].managed, "Hook should be managed");
    assert_eq!(entries[0].event, HookEvent::Stop);
    assert_eq!(entries[0].handler.command, "/path/to/stop.sh");
    assert!(
        entries[0].metadata.is_some(),
        "Managed hook should have metadata"
    );

    // Uninstall hook
    let result = uninstall(HookEvent::Stop, "/path/to/stop.sh");
    assert!(
        result.is_ok(),
        "Uninstall should succeed: {:?}",
        result.err()
    );

    // List hooks - should be empty
    let entries = list().expect("List should succeed");
    assert_eq!(entries.len(), 0, "Should have no hooks after uninstall");
}

#[test]
#[serial(home)]
fn test_install_duplicate_fails() {
    let _dir = setup_test_env();

    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/stop.sh".to_string(),
        timeout: Some(600),
        r#async: None,
        status_message: None,
    };

    // First install should succeed
    let result = install(HookEvent::Stop, handler.clone(), None, "test");
    assert!(result.is_ok(), "First install should succeed");

    // Second install should fail with AlreadyExists
    let result = install(HookEvent::Stop, handler, None, "test");
    assert!(result.is_err(), "Second install should fail");

    match result.unwrap_err() {
        Error::Hook(HookError::AlreadyExists { event, command }) => {
            assert_eq!(event, HookEvent::Stop);
            assert_eq!(command, "/path/to/stop.sh");
        }
        e => panic!("Expected AlreadyExists error, got: {:?}", e),
    }
}

#[test]
#[serial(home)]
fn test_uninstall_unmanaged_fails() {
    let _dir = setup_test_env();

    // Try to uninstall hook that doesn't exist
    let result = uninstall(HookEvent::Stop, "/unmanaged/hook.sh");
    assert!(result.is_err(), "Uninstall of unmanaged hook should fail");

    match result.unwrap_err() {
        Error::Hook(HookError::NotManaged { event, command }) => {
            assert_eq!(event, HookEvent::Stop);
            assert_eq!(command, "/unmanaged/hook.sh");
        }
        e => panic!("Expected NotManaged error, got: {:?}", e),
    }
}

#[test]
#[serial(home)]
fn test_list_shows_unmanaged_hooks() {
    let _dir = setup_test_env();

    // Manually add hook to settings.json (not via install)
    let settings = settings::read_settings().expect("Failed to read settings");
    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/unmanaged/hook.sh".to_string(),
        timeout: None,
        r#async: None,
        status_message: None,
    };
    let updated = settings::add_hook(settings, HookEvent::SessionStart, handler, None);
    settings::write_settings_atomic(updated).expect("Failed to write settings");

    // List should show hook as unmanaged
    let entries = list().expect("List should succeed");
    assert_eq!(entries.len(), 1, "Should have 1 hook");
    assert!(!entries[0].managed, "Hook should be unmanaged");
    assert_eq!(entries[0].event, HookEvent::SessionStart);
    assert!(
        entries[0].metadata.is_none(),
        "Unmanaged hook should not have metadata"
    );
}

#[test]
#[serial(home)]
fn test_install_multiple_hooks() {
    let _dir = setup_test_env();

    // Install Stop hook
    let stop_handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/stop.sh".to_string(),
        timeout: Some(600),
        r#async: None,
        status_message: None,
    };
    install(HookEvent::Stop, stop_handler, None, "test").expect("Stop install should succeed");

    // Install SessionStart hook
    let start_handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/start.sh".to_string(),
        timeout: Some(300),
        r#async: None,
        status_message: None,
    };
    install(HookEvent::SessionStart, start_handler, None, "test")
        .expect("SessionStart install should succeed");

    // List should show both hooks
    let entries = list().expect("List should succeed");
    assert_eq!(entries.len(), 2, "Should have 2 hooks");
    assert!(
        entries.iter().all(|e| e.managed),
        "All hooks should be managed"
    );

    // Verify both events are present
    let events: Vec<HookEvent> = entries.iter().map(|e| e.event).collect();
    assert!(events.contains(&HookEvent::Stop));
    assert!(events.contains(&HookEvent::SessionStart));
}

#[test]
#[serial(home)]
fn test_uninstall_preserves_other_hooks() {
    let _dir = setup_test_env();

    // Install two hooks
    let stop_handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/stop.sh".to_string(),
        timeout: Some(600),
        r#async: None,
        status_message: None,
    };
    install(HookEvent::Stop, stop_handler, None, "test").expect("Stop install should succeed");

    let start_handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/start.sh".to_string(),
        timeout: Some(300),
        r#async: None,
        status_message: None,
    };
    install(HookEvent::SessionStart, start_handler, None, "test")
        .expect("SessionStart install should succeed");

    // Uninstall Stop hook
    uninstall(HookEvent::Stop, "/path/to/stop.sh").expect("Uninstall should succeed");

    // List should show only SessionStart hook
    let entries = list().expect("List should succeed");
    assert_eq!(entries.len(), 1, "Should have 1 hook remaining");
    assert_eq!(entries[0].event, HookEvent::SessionStart);
    assert_eq!(entries[0].handler.command, "/path/to/start.sh");
}

#[test]
#[serial(home)]
fn test_install_with_optional_fields() {
    let _dir = setup_test_env();

    // Install hook with all optional fields
    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/async.sh".to_string(),
        timeout: Some(900),
        r#async: Some(true),
        status_message: Some("Running...".to_string()),
    };

    install(HookEvent::PostToolUse, handler, None, "test").expect("Install should succeed");

    // List and verify optional fields are preserved
    let entries = list().expect("List should succeed");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].handler.timeout, Some(900));
    assert_eq!(entries[0].handler.r#async, Some(true));
}

#[test]
#[serial(home)]
fn test_metadata_is_preserved() {
    let _dir = setup_test_env();

    // Install hook
    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/test.sh".to_string(),
        timeout: None,
        r#async: None,
        status_message: None,
    };

    install(HookEvent::Stop, handler, None, "test-installer").expect("Install should succeed");

    // List and verify metadata
    let entries = list().expect("List should succeed");
    assert_eq!(entries.len(), 1);

    let metadata = entries[0].metadata.as_ref().expect("Should have metadata");
    assert_eq!(metadata.installed_by, "test-installer");
    assert!(!metadata.added_at.is_empty(), "Should have timestamp");
}

#[test]
#[serial(home)]
fn test_hook_in_registry_but_not_settings() {
    let _dir = setup_test_env();

    // Install hook
    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/test.sh".to_string(),
        timeout: None,
        r#async: None,
        status_message: None,
    };

    install(HookEvent::Stop, handler, None, "test").expect("Install should succeed");

    // Manually remove from settings.json (simulate user deletion)
    let settings = settings::read_settings().expect("Failed to read settings");
    let updated = settings::remove_hook(settings, HookEvent::Stop, "/path/to/test.sh");
    settings::write_settings_atomic(updated).expect("Failed to write settings");

    // Uninstall should still succeed (cleans up registry)
    let result = uninstall(HookEvent::Stop, "/path/to/test.sh");
    assert!(
        result.is_ok(),
        "Uninstall should succeed even if hook not in settings"
    );

    // List should be empty
    let entries = list().expect("List should succeed");
    assert_eq!(entries.len(), 0, "Should have no hooks");
}

#[test]
#[serial(home)]
fn test_different_commands_same_event() {
    let _dir = setup_test_env();

    // Install two hooks for same event but different commands
    let handler1 = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/stop1.sh".to_string(),
        timeout: None,
        r#async: None,
        status_message: None,
    };
    install(HookEvent::Stop, handler1, None, "test").expect("First install should succeed");

    let handler2 = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/stop2.sh".to_string(),
        timeout: None,
        r#async: None,
        status_message: None,
    };
    install(HookEvent::Stop, handler2, None, "test").expect("Second install should succeed");

    // List should show both hooks
    let entries = list().expect("List should succeed");
    assert_eq!(entries.len(), 2, "Should have 2 hooks");

    // Both should be for Stop event
    assert!(entries.iter().all(|e| e.event == HookEvent::Stop));

    // Commands should be different
    let commands: Vec<&str> = entries.iter().map(|e| e.handler.command.as_str()).collect();
    assert!(commands.contains(&"/path/to/stop1.sh"));
    assert!(commands.contains(&"/path/to/stop2.sh"));

    // Uninstall first hook
    uninstall(HookEvent::Stop, "/path/to/stop1.sh").expect("Uninstall should succeed");

    // List should show only second hook
    let entries = list().expect("List should succeed");
    assert_eq!(entries.len(), 1, "Should have 1 hook remaining");
    assert_eq!(entries[0].handler.command, "/path/to/stop2.sh");
}

#[test]
#[serial(home)]
fn test_install_with_matcher() {
    let _dir = setup_test_env();

    // Install PreToolUse hook with Bash matcher
    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/pre-bash.sh".to_string(),
        timeout: Some(10),
        r#async: None,
        status_message: None,
    };

    install(
        HookEvent::PreToolUse,
        handler,
        Some("Bash".to_string()),
        "test",
    )
    .expect("Install should succeed");

    // List should show the hook
    let entries = list().expect("List should succeed");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].event, HookEvent::PreToolUse);
    assert_eq!(entries[0].handler.command, "/path/to/pre-bash.sh");
}
