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
mod integration_tests;
