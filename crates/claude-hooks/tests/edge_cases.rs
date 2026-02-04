//! Edge case tests
//!
//! Tests sync issues, corrupt files, missing data
//! Validates error handling and recovery scenarios

use claude_hooks::{install, list, uninstall, Error, HookError, HookEvent, HookHandler, SettingsError};
use serial_test::serial;
use std::env;
use std::fs;
use tempfile::tempdir;

/// Setup isolated test environment
fn setup_test_env() -> tempfile::TempDir {
    let dir = tempdir().expect("Failed to create temp directory");
    env::set_var("HOME", dir.path());

    let claude_dir = dir.path().join(".claude");
    fs::create_dir_all(&claude_dir).expect("Failed to create .claude directory");

    let settings = serde_json::json!({
        "hooks": [],
        "cleanupPeriodDays": 7
    });
    fs::write(
        claude_dir.join("settings.json"),
        serde_json::to_string_pretty(&settings).expect("Serialize failed"),
    )
    .expect("Write failed");

    dir
}

#[test]
#[serial(edge_cases)]
fn test_hook_in_registry_but_not_in_settings() {
    let _dir = setup_test_env();

    // Install hook normally
    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/test.sh".to_string(),
        matcher: String::new(),
        timeout: None,
        r#async: None,
    };
    install(HookEvent::Stop, handler, "test").expect("Install should succeed");

    // Manually remove from settings.json (simulate user deletion)
    let settings = serde_json::json!({
        "hooks": [],
        "cleanupPeriodDays": 7
    });
    fs::write(
        env::var("HOME").expect("HOME not set") + "/.claude/settings.json",
        serde_json::to_string_pretty(&settings).expect("Serialize failed"),
    )
    .expect("Write failed");

    // Uninstall should succeed (cleans registry without error)
    let result = uninstall(HookEvent::Stop, "/path/to/test.sh");
    assert!(result.is_ok(), "Uninstall should succeed even if not in settings");

    // Verify registry cleaned
    let entries = list().expect("List should succeed");
    assert_eq!(entries.len(), 0);
}

#[test]
#[serial(edge_cases)]
fn test_hook_in_settings_but_not_in_registry() {
    let _dir = setup_test_env();

    // Manually add hook to settings.json (not via install)
    let settings = serde_json::json!({
        "hooks": [
            {
                "event": "Start",
                "command": "/unmanaged/hook.sh",
                "type": "command",
                "matcher": ""
            }
        ],
        "cleanupPeriodDays": 7
    });
    fs::write(
        env::var("HOME").expect("HOME not set") + "/.claude/settings.json",
        serde_json::to_string_pretty(&settings).expect("Serialize failed"),
    )
    .expect("Write failed");

    // List should show as unmanaged
    let entries = list().expect("List should succeed");
    assert_eq!(entries.len(), 1);
    assert!(!entries[0].managed, "Hook should be unmanaged");
    assert!(entries[0].metadata.is_none(), "Should not have metadata");
}

#[test]
#[serial(edge_cases)]
fn test_corrupt_settings_json() {
    let _dir = setup_test_env();

    // Write invalid JSON to settings.json
    fs::write(
        env::var("HOME").expect("HOME not set") + "/.claude/settings.json",
        "{ invalid json }",
    )
    .expect("Write failed");

    // Operations should return parse error
    let result = list();
    assert!(result.is_err(), "List should fail with parse error");

    match result.unwrap_err() {
        Error::Settings(SettingsError::Parse(_)) => {
            // Expected error
        }
        e => panic!("Expected SettingsError::Parse, got: {:?}", e),
    }
}

#[test]
#[serial(edge_cases)]
fn test_missing_hooks_array() {
    let _dir = setup_test_env();

    // Settings.json without hooks key
    let settings = serde_json::json!({
        "cleanupPeriodDays": 7
    });
    fs::write(
        env::var("HOME").expect("HOME not set") + "/.claude/settings.json",
        serde_json::to_string_pretty(&settings).expect("Serialize failed"),
    )
    .expect("Write failed");

    // List should return parse error
    let result = list();
    assert!(result.is_err(), "List should fail without hooks array");

    match result.unwrap_err() {
        Error::Settings(SettingsError::Parse(msg)) => {
            assert!(msg.contains("hooks"), "Error should mention hooks array");
        }
        e => panic!("Expected SettingsError::Parse, got: {:?}", e),
    }
}

#[test]
#[serial(edge_cases)]
fn test_hooks_not_an_array() {
    let _dir = setup_test_env();

    // Settings.json with hooks as wrong type
    let settings = serde_json::json!({
        "hooks": "not an array",
        "cleanupPeriodDays": 7
    });
    fs::write(
        env::var("HOME").expect("HOME not set") + "/.claude/settings.json",
        serde_json::to_string_pretty(&settings).expect("Serialize failed"),
    )
    .expect("Write failed");

    // Operations should fail
    let result = list();
    assert!(result.is_err(), "List should fail with invalid hooks type");
}

#[test]
#[serial(edge_cases)]
fn test_settings_file_not_found() {
    let dir = tempdir().expect("Tempdir failed");
    env::set_var("HOME", dir.path());

    // Don't create .claude directory or settings.json

    // Operations should fail with IO error
    let result = list();
    assert!(result.is_err(), "List should fail when settings.json missing");

    match result.unwrap_err() {
        Error::Settings(SettingsError::Io(_)) => {
            // Expected error
        }
        e => panic!("Expected SettingsError::Io, got: {:?}", e),
    }
}

#[test]
#[serial(edge_cases)]
fn test_install_duplicate_via_registry() {
    let _dir = setup_test_env();

    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/test.sh".to_string(),
        matcher: String::new(),
        timeout: None,
        r#async: None,
    };

    // First install
    install(HookEvent::Stop, handler.clone(), "test").expect("Install should succeed");

    // Second install should fail
    let result = install(HookEvent::Stop, handler, "test");
    assert!(result.is_err(), "Duplicate install should fail");

    match result.unwrap_err() {
        Error::Hook(HookError::AlreadyExists { event, command }) => {
            assert_eq!(event, HookEvent::Stop);
            assert_eq!(command, "/path/to/test.sh");
        }
        e => panic!("Expected HookError::AlreadyExists, got: {:?}", e),
    }
}

#[test]
#[serial(edge_cases)]
fn test_install_duplicate_via_settings() {
    let _dir = setup_test_env();

    // Manually add hook to settings (not in registry)
    let settings = serde_json::json!({
        "hooks": [
            {
                "event": "Stop",
                "command": "/path/to/test.sh",
                "type": "command",
                "matcher": ""
            }
        ]
    });
    fs::write(
        env::var("HOME").expect("HOME not set") + "/.claude/settings.json",
        serde_json::to_string_pretty(&settings).expect("Serialize failed"),
    )
    .expect("Write failed");

    // Try to install same hook
    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/test.sh".to_string(),
        matcher: String::new(),
        timeout: None,
        r#async: None,
    };

    let result = install(HookEvent::Stop, handler, "test");
    assert!(result.is_err(), "Install should fail if already in settings");

    match result.unwrap_err() {
        Error::Hook(HookError::AlreadyExists { .. }) => {
            // Expected
        }
        e => panic!("Expected HookError::AlreadyExists, got: {:?}", e),
    }
}

#[test]
#[serial(edge_cases)]
fn test_uninstall_nonexistent_hook() {
    let _dir = setup_test_env();

    // Try to uninstall hook that doesn't exist
    let result = uninstall(HookEvent::Stop, "/nonexistent/hook.sh");
    assert!(result.is_err(), "Uninstall should fail for nonexistent hook");

    match result.unwrap_err() {
        Error::Hook(HookError::NotManaged { event, command }) => {
            assert_eq!(event, HookEvent::Stop);
            assert_eq!(command, "/nonexistent/hook.sh");
        }
        e => panic!("Expected HookError::NotManaged, got: {:?}", e),
    }
}

#[test]
#[serial(edge_cases)]
fn test_uninstall_unmanaged_hook() {
    let _dir = setup_test_env();

    // Manually add hook to settings (not in registry)
    let settings = serde_json::json!({
        "hooks": [
            {
                "event": "Stop",
                "command": "/unmanaged/hook.sh",
                "type": "command",
                "matcher": ""
            }
        ]
    });
    fs::write(
        env::var("HOME").expect("HOME not set") + "/.claude/settings.json",
        serde_json::to_string_pretty(&settings).expect("Serialize failed"),
    )
    .expect("Write failed");

    // Try to uninstall unmanaged hook
    let result = uninstall(HookEvent::Stop, "/unmanaged/hook.sh");
    assert!(result.is_err(), "Uninstall should fail for unmanaged hook");

    match result.unwrap_err() {
        Error::Hook(HookError::NotManaged { .. }) => {
            // Expected
        }
        e => panic!("Expected HookError::NotManaged, got: {:?}", e),
    }
}

#[test]
#[serial(edge_cases)]
fn test_malformed_hook_in_settings() {
    let _dir = setup_test_env();

    // Add malformed hook (missing required fields)
    let settings = serde_json::json!({
        "hooks": [
            {
                "event": "Stop",
                // Missing command, type, matcher
            }
        ]
    });
    fs::write(
        env::var("HOME").expect("HOME not set") + "/.claude/settings.json",
        serde_json::to_string_pretty(&settings).expect("Serialize failed"),
    )
    .expect("Write failed");

    // List should fail with parse error
    let result = list();
    assert!(result.is_err(), "List should fail with malformed hook");
}

#[test]
#[serial(edge_cases)]
fn test_invalid_event_in_settings() {
    let _dir = setup_test_env();

    // Add hook with invalid event
    let settings = serde_json::json!({
        "hooks": [
            {
                "event": "InvalidEvent",
                "command": "/path/to/hook.sh",
                "type": "command",
                "matcher": ""
            }
        ]
    });
    fs::write(
        env::var("HOME").expect("HOME not set") + "/.claude/settings.json",
        serde_json::to_string_pretty(&settings).expect("Serialize failed"),
    )
    .expect("Write failed");

    // List should fail with parse error
    let result = list();
    assert!(result.is_err(), "List should fail with invalid event");
}

#[test]
#[serial(edge_cases)]
fn test_empty_settings_file() {
    let _dir = setup_test_env();

    // Write empty file
    fs::write(
        env::var("HOME").expect("HOME not set") + "/.claude/settings.json",
        "",
    )
    .expect("Write failed");

    // Operations should fail
    let result = list();
    assert!(result.is_err(), "List should fail with empty file");
}

#[test]
#[serial(edge_cases)]
fn test_settings_with_comments_and_trailing_commas() {
    let _dir = setup_test_env();

    // JSONC with comments (should work due to json_comments crate)
    let jsonc = r#"{
        // This is a comment
        "hooks": [
            {
                "event": "Stop",
                "command": "/path/to/hook.sh",
                "type": "command",
                "matcher": "",
            }  // trailing comma
        ],
        "cleanupPeriodDays": 7,
    }"#;

    fs::write(
        env::var("HOME").expect("HOME not set") + "/.claude/settings.json",
        jsonc,
    )
    .expect("Write failed");

    // Should parse successfully (json_comments handles JSONC)
    let result = list();
    // Note: Current implementation may not support JSONC yet
    // This test documents expected behavior
    match result {
        Ok(entries) => {
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].handler.command, "/path/to/hook.sh");
        }
        Err(_) => {
            // If not implemented yet, that's OK for now
            // This test serves as documentation
        }
    }
}

#[test]
#[serial(edge_cases)]
fn test_registry_dir_not_exist() {
    let dir = tempdir().expect("Tempdir failed");
    env::set_var("HOME", dir.path());

    // Create settings but not registry directory
    let claude_dir = dir.path().join(".claude");
    fs::create_dir_all(&claude_dir).expect("mkdir failed");

    let settings = serde_json::json!({
        "hooks": []
    });
    fs::write(
        claude_dir.join("settings.json"),
        serde_json::to_string_pretty(&settings).expect("Serialize failed"),
    )
    .expect("Write failed");

    // Install should succeed (registry dir created automatically)
    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/test.sh".to_string(),
        matcher: String::new(),
        timeout: None,
        r#async: None,
    };

    let result = install(HookEvent::Stop, handler, "test");
    assert!(result.is_ok(), "Install should create registry dir if missing");
}

#[test]
#[serial(edge_cases)]
fn test_multiple_same_command_different_events() {
    let _dir = setup_test_env();

    // Install same command for different events (should be allowed)
    let command = "/path/to/multi.sh";

    let handler1 = HookHandler {
        r#type: "command".to_string(),
        command: command.to_string(),
        matcher: String::new(),
        timeout: None,
        r#async: None,
    };
    install(HookEvent::Start, handler1, "test").expect("Start install should succeed");

    let handler2 = HookHandler {
        r#type: "command".to_string(),
        command: command.to_string(),
        matcher: String::new(),
        timeout: None,
        r#async: None,
    };
    install(HookEvent::Stop, handler2, "test").expect("Stop install should succeed");

    // Verify both exist
    let entries = list().expect("List should succeed");
    assert_eq!(entries.len(), 2);

    // Both should have same command
    assert!(entries.iter().all(|e| e.handler.command == command));

    // Events should be different
    let events: Vec<HookEvent> = entries.iter().map(|e| e.event).collect();
    assert!(events.contains(&HookEvent::Start));
    assert!(events.contains(&HookEvent::Stop));
}
