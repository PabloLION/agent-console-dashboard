//! Full workflow integration tests
//!
//! Tests complete workflows: install → list → uninstall
//! Verifies managed/unmanaged status tracking
//! Tests multiple hooks and complex scenarios

use claude_hooks::{install, list, uninstall, HookEvent, HookHandler};
use serial_test::serial;
use std::env;
use std::fs;
use tempfile::tempdir;

/// Setup isolated test environment with temp HOME directory
fn setup_test_env() -> tempfile::TempDir {
    let dir = tempdir().expect("Failed to create temp directory");

    // Override HOME to point to temp directory
    env::set_var("HOME", dir.path());

    // Create .claude directory
    let claude_dir = dir.path().join(".claude");
    fs::create_dir_all(&claude_dir).expect("Failed to create .claude directory");

    // Create minimal settings.json with hooks object (not array)
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
fn test_full_install_workflow() {
    let _dir = setup_test_env();

    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/stop.sh".to_string(),
        timeout: Some(600),
        r#async: None,
        status_message: None,
    };

    // Install
    install(HookEvent::Stop, handler.clone(), None, "test").expect("Install should succeed");

    // Verify in list
    let entries = list().expect("List should succeed");
    assert_eq!(entries.len(), 1);
    assert!(entries[0].managed);
    assert_eq!(entries[0].handler.command, "/path/to/stop.sh");

    // Uninstall
    uninstall(HookEvent::Stop, "/path/to/stop.sh").expect("Uninstall should succeed");

    // Verify removed
    let entries = list().expect("List should succeed");
    assert_eq!(entries.len(), 0);
}

#[test]
#[serial(home)]
fn test_install_preserves_existing_hooks() {
    let _dir = setup_test_env();

    // Manually add a hook first (using correct format)
    let settings = serde_json::json!({
        "hooks": {
            "SessionStart": [
                {
                    "hooks": [
                        { "command": "/existing/hook.sh", "type": "command" }
                    ]
                }
            ]
        }
    });
    fs::write(
        env::var("HOME").expect("HOME not set") + "/.claude/settings.json",
        serde_json::to_string_pretty(&settings).expect("Serialize failed"),
    )
    .expect("Write failed");

    // Install new hook
    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/new/hook.sh".to_string(),
        timeout: None,
        r#async: None,
        status_message: None,
    };

    install(HookEvent::Stop, handler, None, "test").expect("Install should succeed");

    // Verify both hooks exist
    let entries = list().expect("List should succeed");
    assert_eq!(entries.len(), 2);

    // One managed, one unmanaged
    let managed_count = entries.iter().filter(|e| e.managed).count();
    let unmanaged_count = entries.iter().filter(|e| !e.managed).count();
    assert_eq!(managed_count, 1);
    assert_eq!(unmanaged_count, 1);
}

#[test]
#[serial(home)]
fn test_uninstall_preserves_unmanaged_hooks() {
    let _dir = setup_test_env();

    // Create settings with unmanaged hook
    let settings = serde_json::json!({
        "hooks": {
            "SessionStart": [
                {
                    "hooks": [
                        { "command": "/unmanaged/hook.sh", "type": "command" }
                    ]
                }
            ]
        }
    });
    fs::write(
        env::var("HOME").expect("HOME not set") + "/.claude/settings.json",
        serde_json::to_string_pretty(&settings).expect("Serialize failed"),
    )
    .expect("Write failed");

    // Install managed hook
    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/managed/hook.sh".to_string(),
        timeout: None,
        r#async: None,
        status_message: None,
    };
    install(HookEvent::Stop, handler, None, "test").expect("Install should succeed");

    // Verify both exist
    let entries = list().expect("List should succeed");
    assert_eq!(entries.len(), 2);

    // Uninstall managed hook
    uninstall(HookEvent::Stop, "/managed/hook.sh").expect("Uninstall should succeed");

    // Verify unmanaged hook preserved
    let entries = list().expect("List should succeed");
    assert_eq!(entries.len(), 1);
    assert!(!entries[0].managed);
    assert_eq!(entries[0].handler.command, "/unmanaged/hook.sh");
}

#[test]
#[serial(home)]
fn test_multiple_hooks_different_events() {
    let _dir = setup_test_env();

    // Install hooks for different events
    let events = vec![
        (HookEvent::SessionStart, "/path/to/start.sh"),
        (HookEvent::Stop, "/path/to/stop.sh"),
        (HookEvent::PreToolUse, "/path/to/pretool.sh"),
    ];

    for (event, command) in &events {
        let handler = HookHandler {
            r#type: "command".to_string(),
            command: command.to_string(),
            timeout: None,
            r#async: None,
            status_message: None,
        };
        install(*event, handler, None, "test").expect("Install should succeed");
    }

    // Verify all hooks present
    let entries = list().expect("List should succeed");
    assert_eq!(entries.len(), 3);
    assert!(entries.iter().all(|e| e.managed));

    // Verify all events represented
    let found_events: Vec<HookEvent> = entries.iter().map(|e| e.event).collect();
    assert!(found_events.contains(&HookEvent::SessionStart));
    assert!(found_events.contains(&HookEvent::Stop));
    assert!(found_events.contains(&HookEvent::PreToolUse));
}

#[test]
#[serial(home)]
fn test_multiple_hooks_same_event() {
    let _dir = setup_test_env();

    // Install multiple hooks for same event
    let commands = vec![
        "/path/to/stop1.sh",
        "/path/to/stop2.sh",
        "/path/to/stop3.sh",
    ];

    for command in &commands {
        let handler = HookHandler {
            r#type: "command".to_string(),
            command: command.to_string(),
            timeout: None,
            r#async: None,
            status_message: None,
        };
        install(HookEvent::Stop, handler, None, "test").expect("Install should succeed");
    }

    // Verify all hooks present
    let entries = list().expect("List should succeed");
    assert_eq!(entries.len(), 3);
    assert!(entries.iter().all(|e| e.event == HookEvent::Stop));

    // Uninstall middle one
    uninstall(HookEvent::Stop, "/path/to/stop2.sh").expect("Uninstall should succeed");

    // Verify others preserved
    let entries = list().expect("List should succeed");
    assert_eq!(entries.len(), 2);
    let found_commands: Vec<&str> = entries.iter().map(|e| e.handler.command.as_str()).collect();
    assert!(found_commands.contains(&"/path/to/stop1.sh"));
    assert!(found_commands.contains(&"/path/to/stop3.sh"));
}

#[test]
#[serial(home)]
fn test_install_with_all_optional_fields() {
    let _dir = setup_test_env();

    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/async.sh".to_string(),
        timeout: Some(900),
        r#async: Some(true),
        status_message: Some("Running...".to_string()),
    };

    install(
        HookEvent::PostToolUse,
        handler,
        Some("*.rs".to_string()),
        "test",
    )
    .expect("Install should succeed");

    // Verify all fields preserved
    let entries = list().expect("List should succeed");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].handler.timeout, Some(900));
    assert_eq!(entries[0].handler.r#async, Some(true));
}

#[test]
#[serial(home)]
fn test_metadata_fields_populated() {
    let _dir = setup_test_env();

    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/test.sh".to_string(),
        timeout: None,
        r#async: None,
        status_message: None,
    };

    install(HookEvent::Stop, handler, None, "my-installer").expect("Install should succeed");

    // Verify metadata
    let entries = list().expect("List should succeed");
    assert_eq!(entries.len(), 1);

    let metadata = entries[0].metadata.as_ref().expect("Should have metadata");
    assert_eq!(metadata.installed_by, "my-installer");
    assert!(!metadata.added_at.is_empty());
    // Timestamp format: yyyyMMdd-hhmmss
    assert!(metadata.added_at.len() >= 15);
}

#[test]
#[serial(home)]
fn test_list_empty_hooks_object() {
    let _dir = setup_test_env();

    // Verify empty list returns empty vector
    let entries = list().expect("List should succeed");
    assert_eq!(entries.len(), 0);
}

#[test]
#[serial(home)]
fn test_roundtrip_preserves_settings_keys() {
    let _dir = setup_test_env();

    // Create settings with various keys
    let settings = serde_json::json!({
        "hooks": {},
        "cleanupPeriodDays": 7,
        "env": {"TEST": "value"},
        "permissions": {},
        "statusLine": true,
        "enabledPlugins": ["plugin1"],
        "customKey": "should be preserved"
    });
    fs::write(
        env::var("HOME").expect("HOME not set") + "/.claude/settings.json",
        serde_json::to_string_pretty(&settings).expect("Serialize failed"),
    )
    .expect("Write failed");

    // Install and uninstall hook
    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/test.sh".to_string(),
        timeout: None,
        r#async: None,
        status_message: None,
    };
    install(HookEvent::Stop, handler, None, "test").expect("Install should succeed");
    uninstall(HookEvent::Stop, "/path/to/test.sh").expect("Uninstall should succeed");

    // Verify all non-hook keys preserved
    let content =
        fs::read_to_string(env::var("HOME").expect("HOME not set") + "/.claude/settings.json")
            .expect("Read failed");
    let final_settings: serde_json::Value = serde_json::from_str(&content).expect("Parse failed");

    assert_eq!(final_settings["cleanupPeriodDays"], 7);
    assert_eq!(final_settings["env"]["TEST"], "value");
    assert_eq!(final_settings["customKey"], "should be preserved");
    assert_eq!(final_settings["statusLine"], true);
}
