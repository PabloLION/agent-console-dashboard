//! Programmatic management of Claude Code hooks
//!
//! This crate provides a library API to install, uninstall, and list hooks
//! in Claude Code's settings.json with atomic safety guarantees and
//! ownership tracking.
//!
//! # Platform Support
//!
//! - macOS: Supported
//! - Linux: Supported
//! - Windows: Not supported in v0.1 (deferred to v0.2+)
//!
//! # Scope Limitations (v0.1)
//!
//! - User scope only (`~/.claude/settings.json`)
//! - No multi-scope support (user/project/local)
//! - Library-only (no CLI binary)
//!
//! # Examples
//!
//! ```ignore
//! use claude_hooks::{HookEvent, HookHandler, install};
//!
//! let handler = HookHandler {
//!     r#type: "command".to_string(),
//!     command: "/path/to/stop.sh".to_string(),
//!     timeout: Some(600),
//!     r#async: None,
//!     status_message: None,
//! };
//!
//! install(HookEvent::Stop, handler, None, "acd")?;
//! ```

#![warn(missing_docs)]

mod error;
mod registry;
mod settings;
mod types;

// Re-export all public types
pub use error::{Error, HookError, RegistryError, Result, SettingsError};
pub use types::{HookEvent, HookHandler, ListEntry, MatcherGroup, RegistryEntry, RegistryMetadata};

/// Install a hook for the specified event.
///
/// # Arguments
/// * `event` - Hook event (Stop, PreToolUse, etc.)
/// * `handler` - Hook handler configuration (command, timeout, etc.)
/// * `matcher` - Optional matcher regex (e.g., "Bash" for PreToolUse hooks)
/// * `installed_by` - Free-form string identifying installer (e.g., "acd")
///
/// # Errors
/// * `HookError::AlreadyExists` - Hook already exists (in registry or settings)
/// * `SettingsError` - Failed to read or write settings.json
/// * `RegistryError` - Failed to read registry (write failure is logged but not returned)
///
/// # Example
/// ```ignore
/// use claude_hooks::{HookEvent, HookHandler, install};
///
/// let handler = HookHandler {
///     r#type: "command".to_string(),
///     command: "/path/to/stop.sh".to_string(),
///     timeout: Some(600),
///     r#async: None,
///     status_message: None,
/// };
///
/// install(HookEvent::Stop, handler, None, "acd")?;
/// ```
pub fn install(
    event: HookEvent,
    handler: HookHandler,
    matcher: Option<String>,
    installed_by: &str,
) -> Result<()> {
    use chrono::Local;

    // 1. Read registry
    let registry_entries = registry::read_registry()?;

    // 2. Check if hook exists in registry
    if registry_entries
        .iter()
        .any(|e| e.matches(event, &handler.command))
    {
        return Err(HookError::AlreadyExists {
            event,
            command: handler.command.clone(),
        }
        .into());
    }

    // 3. Read settings
    let settings_value = settings::read_settings()?;

    // 4. Check if hook exists in settings.json using list_hooks
    let existing_hooks = settings::list_hooks(&settings_value);
    for (hook_event, _, hook_handler) in &existing_hooks {
        if *hook_event == event && hook_handler.command == handler.command {
            return Err(HookError::AlreadyExists {
                event,
                command: handler.command.clone(),
            }
            .into());
        }
    }

    // 5. Add hook to settings
    let updated_settings =
        settings::add_hook(settings_value, event, handler.clone(), matcher.clone());

    // 6. Write settings atomically
    settings::write_settings_atomic(updated_settings)?;

    // 7. Create registry entry
    let timestamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let entry = RegistryEntry {
        event,
        matcher,
        r#type: handler.r#type.clone(),
        command: handler.command.clone(),
        timeout: handler.timeout,
        r#async: handler.r#async,
        scope: "user".to_string(),
        enabled: true,
        added_at: timestamp,
        installed_by: installed_by.to_string(),
        description: None,
        reason: None,
        optional: None,
    };

    // 8. Add entry to registry
    let updated_registry = registry::add_entry(registry_entries, entry);

    // 9. Write registry (log warning on failure, don't fail operation)
    if let Err(e) = registry::write_registry(updated_registry) {
        log::warn!(
            "Failed to write registry after successful settings write: {}",
            e
        );
        log::warn!("Hook installed but not tracked. Remove manually from settings.json if needed.");
    }

    Ok(())
}

/// Uninstall a hook for the specified event and command.
///
/// Only removes hooks installed via this crate (matched via registry).
///
/// # Arguments
/// * `event` - Hook event
/// * `command` - Exact command string
///
/// # Errors
/// * `HookError::NotManaged` - Hook not found in registry (not managed by us)
/// * `SettingsError` - Failed to read or write settings.json
/// * `RegistryError` - Failed to read registry (write failure is logged but not returned)
///
/// # Example
/// ```ignore
/// use claude_hooks::{HookEvent, uninstall};
///
/// uninstall(HookEvent::Stop, "/path/to/stop.sh")?;
/// ```
pub fn uninstall(event: HookEvent, command: &str) -> Result<()> {
    // 1. Read registry
    let registry_entries = registry::read_registry()?;

    // 2. Check if hook exists in registry
    if !registry_entries.iter().any(|e| e.matches(event, command)) {
        return Err(HookError::NotManaged {
            event,
            command: command.to_string(),
        }
        .into());
    }

    // 3. Read settings
    let settings_value = settings::read_settings()?;

    // 4. Check if hook exists in settings.json using list_hooks
    let existing_hooks = settings::list_hooks(&settings_value);
    let hook_in_settings = existing_hooks
        .iter()
        .any(|(e, _, h)| *e == event && h.command == command);

    if !hook_in_settings {
        log::warn!(
            "Hook in registry but not in settings.json: {:?} - {}",
            event,
            command
        );
        log::warn!("Removing from registry anyway (user may have manually deleted)");
    }

    // 5. Remove hook from settings (if exists)
    let updated_settings = settings::remove_hook(settings_value, event, command);

    // 6. Write settings atomically
    settings::write_settings_atomic(updated_settings)?;

    // 7. Remove entry from registry
    let updated_registry = registry::remove_entry(registry_entries, event, command);

    // 8. Write registry (log warning on failure, don't fail operation)
    if let Err(e) = registry::write_registry(updated_registry) {
        log::warn!(
            "Failed to write registry after successful settings write: {}",
            e
        );
        log::warn!("Hook removed but registry dirty. May show as managed until registry fixed.");
    }

    Ok(())
}

/// List all hooks from settings.json with management status.
///
/// Returns all hooks (managed and unmanaged). Managed hooks include metadata.
///
/// # Errors
/// * `SettingsError` - Failed to read or parse settings.json
/// * `RegistryError` - Failed to read or parse registry
///
/// # Example
/// ```ignore
/// use claude_hooks::list;
///
/// for entry in list()? {
///     if entry.managed {
///         println!("Managed: {:?} - {}", entry.event, entry.handler.command);
///     } else {
///         println!("Unmanaged: {:?} - {}", entry.event, entry.handler.command);
///     }
/// }
/// ```
pub fn list() -> Result<Vec<ListEntry>> {
    // 1. Read registry
    let registry_entries = registry::read_registry()?;

    // 2. Read settings
    let settings_value = settings::read_settings()?;

    // 3. Parse hooks from settings.json using list_hooks
    let hooks = settings::list_hooks(&settings_value);

    let mut results = Vec::new();

    for (event, _matcher, handler) in hooks {
        // Check if hook exists in registry
        let registry_entry = registry_entries
            .iter()
            .find(|e| e.matches(event, &handler.command));

        let (managed, metadata) = if let Some(entry) = registry_entry {
            let metadata = RegistryMetadata {
                added_at: entry.added_at.clone(),
                installed_by: entry.installed_by.clone(),
                description: entry.description.clone(),
                reason: entry.reason.clone(),
                optional: entry.optional,
            };
            (true, Some(metadata))
        } else {
            (false, None)
        };

        results.push(ListEntry {
            event,
            handler,
            managed,
            metadata,
        });
    }

    Ok(results)
}

#[cfg(test)]
mod integration_tests {
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
}
