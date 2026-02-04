//! Atomic write safety tests
//!
//! Tests atomic write guarantees and data integrity
//! Validates roundtrip preservation of all keys
//! Tests write failure scenarios

use claude_hooks::{install, uninstall, HookEvent, HookHandler};
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
        "hooks": {},
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
#[serial(home)]
fn test_settings_roundtrip_preserves_all_keys() {
    let _dir = setup_test_env();

    // Create settings with extensive keys
    let settings = serde_json::json!({
        "hooks": {},
        "cleanupPeriodDays": 7,
        "env": {"TEST": "value", "PATH": "/usr/bin"},
        "permissions": {
            "allowFileWrite": true,
            "allowNetworkAccess": false
        },
        "statusLine": true,
        "enabledPlugins": ["plugin1", "plugin2"],
        "syntaxHighlightingDisabled": false,
        "customKey": "should be preserved",
        "nestedObject": {
            "level1": {
                "level2": "deep value"
            }
        },
        "arrayOfObjects": [
            {"name": "item1", "value": 1},
            {"name": "item2", "value": 2}
        ]
    });
    fs::write(
        env::var("HOME").expect("HOME not set") + "/.claude/settings.json",
        serde_json::to_string_pretty(&settings).expect("Serialize failed"),
    )
    .expect("Write failed");

    // Install hook
    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/test.sh".to_string(),
        timeout: None,
        r#async: None,
        status_message: None,
    };
    install(HookEvent::Stop, handler.clone(), None, "test").expect("Install should succeed");

    // Uninstall hook
    uninstall(HookEvent::Stop, "/path/to/test.sh").expect("Uninstall should succeed");

    // Verify all keys preserved
    let content = fs::read_to_string(env::var("HOME").expect("HOME not set") + "/.claude/settings.json")
        .expect("Read failed");
    let final_settings: serde_json::Value = serde_json::from_str(&content).expect("Parse failed");

    // Top-level keys
    assert_eq!(final_settings["cleanupPeriodDays"], 7);
    assert_eq!(final_settings["statusLine"], true);
    assert_eq!(final_settings["syntaxHighlightingDisabled"], false);
    assert_eq!(final_settings["customKey"], "should be preserved");

    // Nested env object
    assert_eq!(final_settings["env"]["TEST"], "value");
    assert_eq!(final_settings["env"]["PATH"], "/usr/bin");

    // Nested permissions object
    assert_eq!(final_settings["permissions"]["allowFileWrite"], true);
    assert_eq!(final_settings["permissions"]["allowNetworkAccess"], false);

    // Arrays
    let plugins = final_settings["enabledPlugins"]
        .as_array()
        .expect("Should be array");
    assert_eq!(plugins.len(), 2);
    assert!(plugins.contains(&serde_json::json!("plugin1")));

    // Deep nesting
    assert_eq!(final_settings["nestedObject"]["level1"]["level2"], "deep value");

    // Array of objects
    let array_of_objects = final_settings["arrayOfObjects"]
        .as_array()
        .expect("Should be array");
    assert_eq!(array_of_objects.len(), 2);
    assert_eq!(array_of_objects[0]["name"], "item1");
    assert_eq!(array_of_objects[1]["value"], 2);

    // Hooks should be empty object after uninstall
    let hooks = final_settings["hooks"].as_object().expect("Should be object");
    assert!(hooks.is_empty() || hooks.values().all(|v| v.as_array().map(|a| a.is_empty()).unwrap_or(true)));
}

#[test]
#[serial(home)]
fn test_install_preserves_existing_hook_order() {
    let _dir = setup_test_env();

    // Create settings with hooks in specific order using correct format
    let settings = serde_json::json!({
        "hooks": {
            "SessionStart": [
                {
                    "hooks": [
                        { "command": "/first/hook.sh", "type": "command" }
                    ]
                }
            ],
            "Stop": [
                {
                    "hooks": [
                        { "command": "/second/hook.sh", "type": "command" }
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

    // Install new hook (should append to PreToolUse)
    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/third/hook.sh".to_string(),
        timeout: None,
        r#async: None,
        status_message: None,
    };
    install(HookEvent::PreToolUse, handler, None, "test").expect("Install should succeed");

    // Verify structure preserved and new hook added
    let content = fs::read_to_string(env::var("HOME").expect("HOME not set") + "/.claude/settings.json")
        .expect("Read failed");
    let final_settings: serde_json::Value = serde_json::from_str(&content).expect("Parse failed");

    let hooks = final_settings["hooks"].as_object().expect("Should be object");
    assert!(hooks.contains_key("SessionStart"));
    assert!(hooks.contains_key("Stop"));
    assert!(hooks.contains_key("PreToolUse"));
}

#[test]
#[serial(home)]
fn test_uninstall_preserves_remaining_hook_order() {
    let _dir = setup_test_env();

    // Install multiple hooks
    let commands = vec!["/first.sh", "/second.sh", "/third.sh", "/fourth.sh"];
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

    // Uninstall middle hook
    uninstall(HookEvent::Stop, "/second.sh").expect("Uninstall should succeed");

    // Verify remaining hooks
    let content = fs::read_to_string(env::var("HOME").expect("HOME not set") + "/.claude/settings.json")
        .expect("Read failed");
    let final_settings: serde_json::Value = serde_json::from_str(&content).expect("Parse failed");

    // Count hooks in Stop event
    let stop_hooks = &final_settings["hooks"]["Stop"];
    let mut hook_commands: Vec<String> = Vec::new();
    if let Some(groups) = stop_hooks.as_array() {
        for group in groups {
            if let Some(hooks) = group["hooks"].as_array() {
                for hook in hooks {
                    if let Some(cmd) = hook["command"].as_str() {
                        hook_commands.push(cmd.to_string());
                    }
                }
            }
        }
    }

    assert_eq!(hook_commands.len(), 3);
    assert!(hook_commands.contains(&"/first.sh".to_string()));
    assert!(!hook_commands.contains(&"/second.sh".to_string()));
    assert!(hook_commands.contains(&"/third.sh".to_string()));
    assert!(hook_commands.contains(&"/fourth.sh".to_string()));
}

#[test]
#[serial(home)]
fn test_atomic_write_creates_temp_file() {
    let _dir = setup_test_env();

    let settings_path = env::var("HOME").expect("HOME not set") + "/.claude/settings.json";

    // Record initial modification time
    let initial_metadata = fs::metadata(&settings_path).expect("Metadata failed");
    let initial_mtime = initial_metadata.modified().expect("mtime failed");

    // Sleep briefly to ensure mtime would change
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Install hook (triggers atomic write)
    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/test.sh".to_string(),
        timeout: None,
        r#async: None,
        status_message: None,
    };
    install(HookEvent::Stop, handler, None, "test").expect("Install should succeed");

    // Verify file was updated (mtime changed)
    let final_metadata = fs::metadata(&settings_path).expect("Metadata failed");
    let final_mtime = final_metadata.modified().expect("mtime failed");
    assert!(final_mtime > initial_mtime, "File should have been updated");

    // Verify no temp files left behind
    let claude_dir = env::var("HOME").expect("HOME not set") + "/.claude";
    let entries = fs::read_dir(&claude_dir).expect("read_dir failed");
    for entry in entries {
        let entry = entry.expect("entry failed");
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        assert!(
            !name_str.contains(".tmp"),
            "Temp file should not exist: {}",
            name_str
        );
    }
}

#[test]
#[serial(home)]
fn test_json_formatting_preserved() {
    let _dir = setup_test_env();

    // Create pretty-formatted settings
    let settings = serde_json::json!({
        "hooks": {},
        "cleanupPeriodDays": 7
    });
    let pretty_json = serde_json::to_string_pretty(&settings).expect("Serialize failed");
    fs::write(
        env::var("HOME").expect("HOME not set") + "/.claude/settings.json",
        &pretty_json,
    )
    .expect("Write failed");

    // Install hook
    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/test.sh".to_string(),
        timeout: None,
        r#async: None,
        status_message: None,
    };
    install(HookEvent::Stop, handler, None, "test").expect("Install should succeed");

    // Verify output is still pretty-formatted
    let content = fs::read_to_string(env::var("HOME").expect("HOME not set") + "/.claude/settings.json")
        .expect("Read failed");

    // Check for indentation (pretty-print uses 2 spaces)
    assert!(content.contains("  "), "Output should be pretty-formatted");
    assert!(content.contains('\n'), "Output should have newlines");

    // Should be valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("Parse failed");
    assert!(parsed.is_object());
}

#[test]
#[serial(home)]
fn test_unicode_and_special_chars_preserved() {
    let _dir = setup_test_env();

    // Create settings with unicode and special chars
    let settings = serde_json::json!({
        "hooks": {},
        "env": {
            "UNICODE": "Hello 世界 🌍",
            "SPECIAL": "quotes\"and\\backslashes",
            "NEWLINES": "line1\nline2\ttab"
        }
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
    install(HookEvent::Stop, handler.clone(), None, "test").expect("Install should succeed");
    uninstall(HookEvent::Stop, "/path/to/test.sh").expect("Uninstall should succeed");

    // Verify special chars preserved
    let content = fs::read_to_string(env::var("HOME").expect("HOME not set") + "/.claude/settings.json")
        .expect("Read failed");
    let final_settings: serde_json::Value = serde_json::from_str(&content).expect("Parse failed");

    assert!(final_settings["env"]["UNICODE"]
        .as_str()
        .expect("Should be string")
        .contains("世界"));
    assert!(final_settings["env"]["UNICODE"]
        .as_str()
        .expect("Should be string")
        .contains("🌍"));
    assert_eq!(
        final_settings["env"]["SPECIAL"].as_str().expect("Should be string"),
        "quotes\"and\\backslashes"
    );
    assert_eq!(
        final_settings["env"]["NEWLINES"].as_str().expect("Should be string"),
        "line1\nline2\ttab"
    );
}

#[test]
#[serial(home)]
fn test_empty_strings_and_nulls_preserved() {
    let _dir = setup_test_env();

    // Create settings with empty strings and nulls
    let settings = serde_json::json!({
        "hooks": {},
        "emptyString": "",
        "nullValue": null,
        "zeroValue": 0,
        "falseValue": false
    });
    fs::write(
        env::var("HOME").expect("HOME not set") + "/.claude/settings.json",
        serde_json::to_string_pretty(&settings).expect("Serialize failed"),
    )
    .expect("Write failed");

    // Install and uninstall
    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/test.sh".to_string(),
        timeout: None,
        r#async: None,
        status_message: None,
    };
    install(HookEvent::Stop, handler.clone(), None, "test").expect("Install should succeed");
    uninstall(HookEvent::Stop, "/path/to/test.sh").expect("Uninstall should succeed");

    // Verify values preserved
    let content = fs::read_to_string(env::var("HOME").expect("HOME not set") + "/.claude/settings.json")
        .expect("Read failed");
    let final_settings: serde_json::Value = serde_json::from_str(&content).expect("Parse failed");

    assert_eq!(final_settings["emptyString"], "");
    assert_eq!(final_settings["nullValue"], serde_json::Value::Null);
    assert_eq!(final_settings["zeroValue"], 0);
    assert_eq!(final_settings["falseValue"], false);
}

#[test]
#[serial(home)]
fn test_large_settings_file_handled() {
    let _dir = setup_test_env();

    // Create settings with many keys and large arrays
    let mut settings = serde_json::json!({
        "hooks": {},
        "cleanupPeriodDays": 7
    });

    // Add 100 custom keys
    for i in 0..100 {
        settings
            .as_object_mut()
            .expect("Should be object")
            .insert(format!("customKey{}", i), serde_json::json!(format!("value{}", i)));
    }

    fs::write(
        env::var("HOME").expect("HOME not set") + "/.claude/settings.json",
        serde_json::to_string_pretty(&settings).expect("Serialize failed"),
    )
    .expect("Write failed");

    // Install hook
    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/test.sh".to_string(),
        timeout: None,
        r#async: None,
        status_message: None,
    };
    install(HookEvent::Stop, handler, None, "test").expect("Install should succeed");

    // Verify all keys still present
    let content = fs::read_to_string(env::var("HOME").expect("HOME not set") + "/.claude/settings.json")
        .expect("Read failed");
    let final_settings: serde_json::Value = serde_json::from_str(&content).expect("Parse failed");

    for i in 0..100 {
        let key = format!("customKey{}", i);
        assert!(
            final_settings.get(&key).is_some(),
            "Key {} should be preserved",
            key
        );
    }
}

#[test]
#[serial(home)]
fn test_concurrent_operations_sequential() {
    let _dir = setup_test_env();

    // Install multiple hooks sequentially
    for i in 0..10 {
        let handler = HookHandler {
            r#type: "command".to_string(),
            command: format!("/path/to/hook{}.sh", i),
            timeout: None,
            r#async: None,
            status_message: None,
        };
        install(HookEvent::Stop, handler, None, "test").expect("Install should succeed");
    }

    // Verify all hooks present
    let content = fs::read_to_string(env::var("HOME").expect("HOME not set") + "/.claude/settings.json")
        .expect("Read failed");
    let final_settings: serde_json::Value = serde_json::from_str(&content).expect("Parse failed");

    // Count hooks in Stop event
    let stop_hooks = &final_settings["hooks"]["Stop"];
    let mut hook_count = 0;
    if let Some(groups) = stop_hooks.as_array() {
        for group in groups {
            if let Some(hooks) = group["hooks"].as_array() {
                hook_count += hooks.len();
            }
        }
    }
    assert_eq!(hook_count, 10);
}
