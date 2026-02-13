use super::*;
use serde_json::json;
use serial_test::serial;

#[test]
fn test_settings_path() {
    let path = settings_path();
    assert!(path.ends_with(".claude/settings.json"));
}

#[test]
fn test_add_hook_creates_structure() {
    let settings = json!({
        "cleanupPeriodDays": 7
    });

    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/stop.sh".to_string(),
        timeout: Some(600),
        r#async: None,
        status_message: None,
    };

    let result = add_hook(settings, HookEvent::Stop, handler, None);

    // Check hooks object was created
    let hooks = result.get("hooks").expect("hooks should exist");
    assert!(hooks.is_object(), "hooks should be object");

    // Check Stop array was created
    let stop_array = hooks.get("Stop").expect("Stop should exist");
    assert!(stop_array.is_array(), "Stop should be array");

    // Check matcher group structure
    let groups = stop_array.as_array().expect("should be array");
    assert_eq!(groups.len(), 1);

    let group = &groups[0];
    assert!(group.get("matcher").is_none(), "matcher should be None");

    let inner_hooks = group.get("hooks").expect("hooks should exist");
    let inner_arr = inner_hooks.as_array().expect("should be array");
    assert_eq!(inner_arr.len(), 1);
    assert_eq!(
        inner_arr[0].get("command").expect("cmd").as_str(),
        Some("/path/to/stop.sh")
    );
}

#[test]
fn test_add_hook_with_matcher() {
    let settings = json!({
        "hooks": {}
    });

    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/pre-bash.sh".to_string(),
        timeout: Some(10),
        r#async: None,
        status_message: None,
    };

    let result = add_hook(
        settings,
        HookEvent::PreToolUse,
        handler,
        Some("Bash".to_string()),
    );

    let hooks = result.get("hooks").expect("hooks");
    let pre_tool_use = hooks.get("PreToolUse").expect("PreToolUse");
    let groups = pre_tool_use.as_array().expect("array");

    assert_eq!(groups.len(), 1);
    assert_eq!(
        groups[0].get("matcher").expect("matcher").as_str(),
        Some("Bash")
    );
}

#[test]
fn test_add_hook_to_existing_event() {
    let settings = json!({
        "hooks": {
            "Stop": [
                {
                    "hooks": [
                        { "type": "command", "command": "/existing/hook.sh" }
                    ]
                }
            ]
        }
    });

    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/new/hook.sh".to_string(),
        timeout: None,
        r#async: None,
        status_message: None,
    };

    let result = add_hook(settings, HookEvent::Stop, handler, None);

    let stop_array = result
        .get("hooks")
        .unwrap()
        .get("Stop")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(stop_array.len(), 2, "should have 2 matcher groups");
}

#[test]
fn test_remove_hook_exact_match() {
    let settings = json!({
        "hooks": {
            "Stop": [
                {
                    "hooks": [
                        { "type": "command", "command": "/path/to/stop.sh" }
                    ]
                }
            ],
            "SessionStart": [
                {
                    "hooks": [
                        { "type": "command", "command": "/path/to/start.sh" }
                    ]
                }
            ]
        }
    });

    let result = remove_hook(settings, HookEvent::Stop, "/path/to/stop.sh");

    let stop_array = result
        .get("hooks")
        .unwrap()
        .get("Stop")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(stop_array.len(), 0, "Stop array should be empty");

    // SessionStart should be preserved
    let start_array = result
        .get("hooks")
        .unwrap()
        .get("SessionStart")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(start_array.len(), 1, "SessionStart should be preserved");
}

#[test]
fn test_remove_hook_preserves_other_groups() {
    let settings = json!({
        "hooks": {
            "Stop": [
                {
                    "hooks": [
                        { "type": "command", "command": "/path/to/stop.sh" }
                    ]
                },
                {
                    "hooks": [
                        { "type": "command", "command": "/different/hook.sh" }
                    ]
                }
            ]
        }
    });

    let result = remove_hook(settings, HookEvent::Stop, "/path/to/stop.sh");

    let stop_array = result
        .get("hooks")
        .unwrap()
        .get("Stop")
        .unwrap()
        .as_array()
        .unwrap();
    assert_eq!(stop_array.len(), 1, "should have 1 remaining group");

    let remaining = &stop_array[0].get("hooks").unwrap().as_array().unwrap()[0];
    assert_eq!(
        remaining.get("command").unwrap().as_str(),
        Some("/different/hook.sh")
    );
}

#[test]
fn test_remove_hook_no_hooks_object() {
    let settings = json!({
        "cleanupPeriodDays": 7
    });

    let result = remove_hook(settings.clone(), HookEvent::Stop, "/any/path");
    assert_eq!(result, settings, "should return unchanged if no hooks");
}

#[test]
fn test_list_hooks_empty() {
    let settings = json!({
        "hooks": {}
    });

    let result = list_hooks(&settings);
    assert!(result.is_empty());
}

#[test]
fn test_list_hooks_multiple() {
    let settings = json!({
        "hooks": {
            "Stop": [
                {
                    "hooks": [
                        { "type": "command", "command": "/stop.sh", "timeout": 15 }
                    ]
                }
            ],
            "PreToolUse": [
                {
                    "matcher": "Bash",
                    "hooks": [
                        { "type": "command", "command": "/pre-bash.sh" }
                    ]
                }
            ]
        }
    });

    let result = list_hooks(&settings);
    assert_eq!(result.len(), 2);

    // Find Stop hook
    let stop = result.iter().find(|(e, _, _)| *e == HookEvent::Stop);
    assert!(stop.is_some());
    let (_, matcher, handler) = stop.unwrap();
    assert!(matcher.is_none());
    assert_eq!(handler.command, "/stop.sh");
    assert_eq!(handler.timeout, Some(15));

    // Find PreToolUse hook
    let pre = result.iter().find(|(e, _, _)| *e == HookEvent::PreToolUse);
    assert!(pre.is_some());
    let (_, matcher, handler) = pre.unwrap();
    assert_eq!(matcher.as_deref(), Some("Bash"));
    assert_eq!(handler.command, "/pre-bash.sh");
}

#[test]
fn test_roundtrip_preserves_non_hook_keys() {
    let settings = json!({
        "hooks": {},
        "cleanupPeriodDays": 7,
        "env": {"TEST": "value"},
        "permissions": {},
        "statusLine": true,
        "enabledPlugins": ["plugin1"],
        "syntaxHighlightingDisabled": false
    });

    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/test.sh".to_string(),
        timeout: None,
        r#async: None,
        status_message: None,
    };

    let result = add_hook(settings, HookEvent::Stop, handler, None);

    assert_eq!(result.get("cleanupPeriodDays").expect("should exist"), 7);
    assert!(result.get("env").is_some());
    assert!(result.get("permissions").is_some());
    assert!(result.get("statusLine").is_some());
    assert!(result.get("enabledPlugins").is_some());
    assert!(result.get("syntaxHighlightingDisabled").is_some());
}

#[test]
#[serial(home)]
fn test_read_valid_settings() {
    use std::io::Write;
    use tempfile::tempdir;

    let dir = tempdir().expect("tempdir creation failed");
    std::env::set_var("HOME", dir.path());

    let claude_dir = dir.path().join(".claude");
    fs::create_dir(&claude_dir).expect("mkdir failed");

    let settings = json!({
        "hooks": {
            "Stop": [{ "hooks": [{ "type": "command", "command": "/test.sh" }] }]
        },
        "cleanupPeriodDays": 7
    });
    let settings_file = claude_dir.join("settings.json");
    let mut file = fs::File::create(&settings_file).expect("file creation failed");
    file.write_all(
        serde_json::to_string_pretty(&settings)
            .expect("serialization failed")
            .as_bytes(),
    )
    .expect("write failed");

    let result = read_settings().expect("read_settings failed");
    assert_eq!(result.get("cleanupPeriodDays").expect("should exist"), 7);
    assert!(result.get("hooks").is_some());
}

#[test]
fn test_timestamp_format() {
    use regex::Regex;

    let timestamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let re = Regex::new(r"^\d{8}-\d{6}$").expect("regex creation failed");
    assert!(
        re.is_match(&timestamp),
        "Timestamp should match format yyyyMMdd-hhmmss, got: {}",
        timestamp
    );
}
