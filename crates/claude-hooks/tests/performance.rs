//! Performance tests
//!
//! Tests operation timing against target metrics
//! Validates that operations complete within acceptable time bounds

use claude_hooks::{install, list, uninstall, HookEvent, HookHandler};
use serial_test::serial;
use std::env;
use std::fs;
use std::time::Instant;
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
#[serial(performance)]
fn test_install_performance() {
    let _dir = setup_test_env();

    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/stop.sh".to_string(),
        matcher: String::new(),
        timeout: Some(600),
        r#async: None,
    };

    let start = Instant::now();
    install(HookEvent::Stop, handler, "test").expect("Install should succeed");
    let duration = start.elapsed();

    println!("Install took: {}ms", duration.as_millis());
    assert!(
        duration.as_millis() < 100,
        "Install took {}ms (target: <100ms)",
        duration.as_millis()
    );
}

#[test]
#[serial(performance)]
fn test_uninstall_performance() {
    let _dir = setup_test_env();

    // Install first
    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/stop.sh".to_string(),
        matcher: String::new(),
        timeout: Some(600),
        r#async: None,
    };
    install(HookEvent::Stop, handler, "test").expect("Install should succeed");

    // Measure uninstall
    let start = Instant::now();
    uninstall(HookEvent::Stop, "/path/to/stop.sh").expect("Uninstall should succeed");
    let duration = start.elapsed();

    println!("Uninstall took: {}ms", duration.as_millis());
    assert!(
        duration.as_millis() < 100,
        "Uninstall took {}ms (target: <100ms)",
        duration.as_millis()
    );
}

#[test]
#[serial(performance)]
fn test_list_performance_with_10_hooks() {
    let _dir = setup_test_env();

    // Install 10 hooks
    for i in 0..10 {
        let handler = HookHandler {
            r#type: "command".to_string(),
            command: format!("/path/to/hook{}.sh", i),
            matcher: String::new(),
            timeout: None,
            r#async: None,
        };
        install(HookEvent::Stop, handler, "test").expect("Install should succeed");
    }

    // Measure list
    let start = Instant::now();
    let entries = list().expect("List should succeed");
    let duration = start.elapsed();

    assert_eq!(entries.len(), 10);
    println!("List (10 hooks) took: {}ms", duration.as_millis());
    assert!(
        duration.as_millis() < 50,
        "List took {}ms (target: <50ms)",
        duration.as_millis()
    );
}

#[test]
#[serial(performance)]
fn test_list_performance_empty() {
    let _dir = setup_test_env();

    // Measure list with empty hooks
    let start = Instant::now();
    let entries = list().expect("List should succeed");
    let duration = start.elapsed();

    assert_eq!(entries.len(), 0);
    println!("List (0 hooks) took: {}ms", duration.as_millis());
    assert!(
        duration.as_millis() < 50,
        "List took {}ms (target: <50ms)",
        duration.as_millis()
    );
}

#[test]
#[serial(performance)]
fn test_install_100_hooks_sequentially() {
    let _dir = setup_test_env();

    let start = Instant::now();

    // Install 100 hooks
    for i in 0..100 {
        let handler = HookHandler {
            r#type: "command".to_string(),
            command: format!("/path/to/hook{}.sh", i),
            matcher: String::new(),
            timeout: None,
            r#async: None,
        };
        install(HookEvent::Stop, handler, "test").expect("Install should succeed");
    }

    let duration = start.elapsed();
    let avg_ms = duration.as_millis() / 100;

    println!(
        "100 sequential installs took: {}ms (avg: {}ms/install)",
        duration.as_millis(),
        avg_ms
    );

    // Average should be well under 100ms per install
    assert!(
        avg_ms < 100,
        "Average install took {}ms (target: <100ms)",
        avg_ms
    );
}

#[test]
#[serial(performance)]
fn test_uninstall_100_hooks_sequentially() {
    let _dir = setup_test_env();

    // Install 100 hooks first
    for i in 0..100 {
        let handler = HookHandler {
            r#type: "command".to_string(),
            command: format!("/path/to/hook{}.sh", i),
            matcher: String::new(),
            timeout: None,
            r#async: None,
        };
        install(HookEvent::Stop, handler, "test").expect("Install should succeed");
    }

    let start = Instant::now();

    // Uninstall all hooks
    for i in 0..100 {
        let command = format!("/path/to/hook{}.sh", i);
        uninstall(HookEvent::Stop, &command).expect("Uninstall should succeed");
    }

    let duration = start.elapsed();
    let avg_ms = duration.as_millis() / 100;

    println!(
        "100 sequential uninstalls took: {}ms (avg: {}ms/uninstall)",
        duration.as_millis(),
        avg_ms
    );

    // Average should be well under 100ms per uninstall
    assert!(
        avg_ms < 100,
        "Average uninstall took {}ms (target: <100ms)",
        avg_ms
    );
}

#[test]
#[serial(performance)]
fn test_list_performance_with_100_hooks() {
    let _dir = setup_test_env();

    // Install 100 hooks
    for i in 0..100 {
        let handler = HookHandler {
            r#type: "command".to_string(),
            command: format!("/path/to/hook{}.sh", i),
            matcher: String::new(),
            timeout: None,
            r#async: None,
        };
        install(HookEvent::Stop, handler, "test").expect("Install should succeed");
    }

    // Measure list
    let start = Instant::now();
    let entries = list().expect("List should succeed");
    let duration = start.elapsed();

    assert_eq!(entries.len(), 100);
    println!("List (100 hooks) took: {}ms", duration.as_millis());

    // List should scale well even with 100 hooks
    // More lenient than 50ms target for large lists
    assert!(
        duration.as_millis() < 200,
        "List took {}ms (target: <200ms for 100 hooks)",
        duration.as_millis()
    );
}

#[test]
#[serial(performance)]
fn test_mixed_operations_performance() {
    let _dir = setup_test_env();

    let start = Instant::now();

    // Mixed operations: install, list, uninstall
    for i in 0..20 {
        // Install
        let handler = HookHandler {
            r#type: "command".to_string(),
            command: format!("/path/to/hook{}.sh", i),
            matcher: String::new(),
            timeout: None,
            r#async: None,
        };
        install(HookEvent::Stop, handler, "test").expect("Install should succeed");

        // List
        let entries = list().expect("List should succeed");
        assert_eq!(entries.len(), i + 1);
    }

    // Uninstall half
    for i in 0..10 {
        let command = format!("/path/to/hook{}.sh", i);
        uninstall(HookEvent::Stop, &command).expect("Uninstall should succeed");
    }

    // Final list
    let entries = list().expect("List should succeed");
    assert_eq!(entries.len(), 10);

    let duration = start.elapsed();
    println!(
        "Mixed operations (20 install+list, 10 uninstall, 1 list) took: {}ms",
        duration.as_millis()
    );

    // Total should be reasonable (20*100 + 10*100 + overhead = ~3000ms budget)
    assert!(
        duration.as_millis() < 5000,
        "Mixed operations took {}ms (target: <5000ms)",
        duration.as_millis()
    );
}

#[test]
#[serial(performance)]
fn test_install_with_large_settings_file() {
    let _dir = setup_test_env();

    // Create settings with many keys to simulate large file
    let mut settings = serde_json::json!({
        "hooks": [],
        "cleanupPeriodDays": 7
    });

    // Add 500 custom keys
    for i in 0..500 {
        settings
            .as_object_mut()
            .expect("Should be object")
            .insert(
                format!("customKey{}", i),
                serde_json::json!(format!("value{}", i)),
            );
    }

    fs::write(
        env::var("HOME").expect("HOME not set") + "/.claude/settings.json",
        serde_json::to_string_pretty(&settings).expect("Serialize failed"),
    )
    .expect("Write failed");

    // Measure install with large file
    let handler = HookHandler {
        r#type: "command".to_string(),
        command: "/path/to/test.sh".to_string(),
        matcher: String::new(),
        timeout: None,
        r#async: None,
    };

    let start = Instant::now();
    install(HookEvent::Stop, handler, "test").expect("Install should succeed");
    let duration = start.elapsed();

    println!(
        "Install with large settings file took: {}ms",
        duration.as_millis()
    );
    // More lenient for large files
    assert!(
        duration.as_millis() < 200,
        "Install with large file took {}ms (target: <200ms)",
        duration.as_millis()
    );
}

#[test]
#[serial(performance)]
fn test_list_with_mixed_managed_unmanaged() {
    let _dir = setup_test_env();

    // Create settings with some unmanaged hooks
    let settings = serde_json::json!({
        "hooks": [
            {
                "event": "Start",
                "command": "/unmanaged/hook1.sh",
                "type": "command",
                "matcher": ""
            },
            {
                "event": "Stop",
                "command": "/unmanaged/hook2.sh",
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

    // Add managed hooks
    for i in 0..8 {
        let handler = HookHandler {
            r#type: "command".to_string(),
            command: format!("/managed/hook{}.sh", i),
            matcher: String::new(),
            timeout: None,
            r#async: None,
        };
        install(HookEvent::Stop, handler, "test").expect("Install should succeed");
    }

    // Measure list (10 total: 2 unmanaged, 8 managed)
    let start = Instant::now();
    let entries = list().expect("List should succeed");
    let duration = start.elapsed();

    assert_eq!(entries.len(), 10);
    let managed_count = entries.iter().filter(|e| e.managed).count();
    assert_eq!(managed_count, 8);

    println!(
        "List with mixed managed/unmanaged took: {}ms",
        duration.as_millis()
    );
    assert!(
        duration.as_millis() < 50,
        "List took {}ms (target: <50ms)",
        duration.as_millis()
    );
}
