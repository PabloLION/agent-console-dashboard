//! Registry file I/O and manipulation for claude-hooks
//!
//! This module manages the local registry in XDG data directory to track
//! hooks installed by this crate. The registry uses JSONC format (JSON with
//! comments) and lives in `$XDG_DATA_HOME/claude-hooks/registry.jsonc`.

use crate::error::{RegistryError, Result};
use crate::types::{HookEvent, RegistryEntry};
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Registry file schema
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Registry {
    schema_version: u32,
    agent_name: String,
    hooks: Vec<RegistryEntry>,
}

/// Returns the path to the registry file
///
/// Uses XDG data directory conventions:
/// - macOS: `~/Library/Application Support/claude-hooks/registry.jsonc`
/// - Linux: `~/.local/share/claude-hooks/registry.jsonc` (or `$XDG_DATA_HOME/claude-hooks/registry.jsonc`)
///
/// # Panics
///
/// Panics if XDG data directory cannot be determined (should never happen on supported platforms)
pub fn registry_path() -> PathBuf {
    let data_dir = dirs::data_dir().expect("Failed to determine XDG data directory");

    data_dir.join("claude-hooks").join("registry.jsonc")
}

/// Read registry file and parse as Vec<RegistryEntry>
///
/// Returns empty vector if registry file doesn't exist yet (first run).
///
/// # Errors
///
/// - `RegistryError::Io` if file read fails
/// - `RegistryError::Parse` if JSONC parsing fails
pub fn read_registry() -> Result<Vec<RegistryEntry>> {
    let path = registry_path();

    if !path.exists() {
        // Registry file doesn't exist yet (first run)
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&path).map_err(RegistryError::Io)?;

    // Strip comments and parse as JSON
    let stripped = json_comments::StripComments::new(content.as_bytes());
    let stripped_bytes: Vec<u8> = std::io::Read::bytes(stripped)
        .collect::<std::io::Result<Vec<u8>>>()
        .map_err(|e| RegistryError::Parse(e.to_string()))?;

    let stripped_str = String::from_utf8(stripped_bytes)
        .map_err(|e| RegistryError::Parse(format!("Invalid UTF-8: {}", e)))?;

    let registry: Registry =
        serde_json::from_str(&stripped_str).map_err(|e| RegistryError::Parse(e.to_string()))?;

    Ok(registry.hooks)
}

/// Write registry file atomically
///
/// Creates directory if missing. Uses atomic rename pattern to ensure
/// consistency even on failure.
///
/// # Errors
///
/// - `RegistryError::Write` if directory creation fails
/// - `RegistryError::Write` if temp file write fails
/// - `RegistryError::Write` if fsync fails
/// - `RegistryError::Write` if rename fails
pub fn write_registry(entries: Vec<RegistryEntry>) -> Result<()> {
    let path = registry_path();

    // Create directory if missing
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| RegistryError::Write(format!("Failed to create directory: {}", e)))?;
    }

    let registry = Registry {
        schema_version: 1,
        agent_name: "claude-code".to_string(),
        hooks: entries,
    };

    // Write with atomic rename pattern
    let timestamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let temp_path = path.with_file_name(format!("registry.jsonc.tmp.{}", timestamp));

    let json = serde_json::to_string_pretty(&registry)
        .map_err(|e| RegistryError::Write(format!("Failed to serialize: {}", e)))?;

    // Add header comment
    let content = format!("// claude-hooks registry\n{}", json);

    fs::write(&temp_path, content)
        .map_err(|e| RegistryError::Write(format!("Failed to write temp file: {}", e)))?;

    // Fsync
    let file = fs::File::open(&temp_path)
        .map_err(|e| RegistryError::Write(format!("Failed to open temp file for fsync: {}", e)))?;
    file.sync_all()
        .map_err(|e| RegistryError::Write(format!("Failed to fsync: {}", e)))?;

    // Atomic rename
    fs::rename(&temp_path, &path).map_err(|e| {
        RegistryError::Write(format!(
            "Failed to rename {} to {}: {}",
            temp_path.display(),
            path.display(),
            e
        ))
    })?;

    Ok(())
}

/// Add entry to registry (pure function, no I/O)
///
/// Appends the entry to the vector and returns the new vector.
pub fn add_entry(mut entries: Vec<RegistryEntry>, entry: RegistryEntry) -> Vec<RegistryEntry> {
    entries.push(entry);
    entries
}

/// Remove entry from registry by exact match (pure function, no I/O)
///
/// Removes all entries that match the given event and command.
/// Uses the composite key matching logic from RegistryEntry::matches().
pub fn remove_entry(
    mut entries: Vec<RegistryEntry>,
    event: HookEvent,
    command: &str,
) -> Vec<RegistryEntry> {
    entries.retain(|entry| !entry.matches(event, command));
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::HookEvent;
    use serial_test::serial;
    use std::env;
    use tempfile::tempdir;

    /// Setup isolated test environment with HOME pointing to temp directory
    fn setup_test_env() -> tempfile::TempDir {
        let dir = tempdir().expect("failed to create temp dir");
        env::set_var("HOME", dir.path());
        dir
    }

    #[test]
    #[serial(home)]
    fn test_registry_path() {
        let _dir = setup_test_env();
        let path = registry_path();
        assert!(
            path.to_string_lossy().contains("claude-hooks"),
            "Path should contain 'claude-hooks'"
        );
        assert!(
            path.to_string_lossy().ends_with("registry.jsonc"),
            "Path should end with 'registry.jsonc'"
        );
    }

    #[test]
    fn test_add_entry() {
        let entries = Vec::new();
        let entry = RegistryEntry {
            event: HookEvent::Stop,
            matcher: None,
            r#type: "command".to_string(),
            command: "/path/to/stop.sh".to_string(),
            timeout: None,
            r#async: None,
            scope: "user".to_string(),
            enabled: true,
            added_at: "20260203-143022".to_string(),
            installed_by: "acd".to_string(),
            description: None,
            reason: None,
            optional: None,
        };

        let result = add_entry(entries, entry.clone());
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].command, "/path/to/stop.sh");
    }

    #[test]
    fn test_remove_entry() {
        let entry1 = RegistryEntry {
            event: HookEvent::Stop,
            matcher: None,
            r#type: "command".to_string(),
            command: "/path/to/stop.sh".to_string(),
            timeout: None,
            r#async: None,
            scope: "user".to_string(),
            enabled: true,
            added_at: "20260203-143022".to_string(),
            installed_by: "acd".to_string(),
            description: None,
            reason: None,
            optional: None,
        };

        let entry2 = RegistryEntry {
            event: HookEvent::SessionStart,
            command: "/path/to/start.sh".to_string(),
            ..entry1.clone()
        };

        let entries = vec![entry1, entry2];
        let result = remove_entry(entries, HookEvent::Stop, "/path/to/stop.sh");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].event, HookEvent::SessionStart);
    }

    #[test]
    fn test_remove_entry_multiple_matches() {
        let entry1 = RegistryEntry {
            event: HookEvent::Stop,
            matcher: None,
            r#type: "command".to_string(),
            command: "/path/to/stop.sh".to_string(),
            timeout: None,
            r#async: None,
            scope: "user".to_string(),
            enabled: true,
            added_at: "20260203-143022".to_string(),
            installed_by: "acd".to_string(),
            description: None,
            reason: None,
            optional: None,
        };

        let entry2 = entry1.clone();
        let entry3 = RegistryEntry {
            event: HookEvent::SessionStart,
            command: "/path/to/start.sh".to_string(),
            ..entry1.clone()
        };

        let entries = vec![entry1, entry2, entry3];
        let result = remove_entry(entries, HookEvent::Stop, "/path/to/stop.sh");

        // Should remove both Stop entries
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].event, HookEvent::SessionStart);
    }

    #[test]
    fn test_jsonc_parsing_with_comments() {
        let jsonc = r#"
        {
          // This is a comment
          "schema_version": 1,
          "agent_name": "claude-code",
          "hooks": []
        }
        "#;

        let stripped = json_comments::StripComments::new(jsonc.as_bytes());
        let stripped_bytes: Vec<u8> = std::io::Read::bytes(stripped)
            .collect::<std::io::Result<Vec<u8>>>()
            .expect("Failed to read stripped content");

        let stripped_str = String::from_utf8(stripped_bytes).expect("Invalid UTF-8");

        let registry: Registry = serde_json::from_str(&stripped_str).expect("Failed to parse JSON");
        assert_eq!(registry.schema_version, 1);
        assert_eq!(registry.agent_name, "claude-code");
    }

    #[test]
    #[serial(home)]
    fn test_write_and_read_registry() {
        let _dir = setup_test_env();

        let entries = vec![RegistryEntry {
            event: HookEvent::Stop,
            matcher: None,
            r#type: "command".to_string(),
            command: "/path/to/test-stop.sh".to_string(),
            timeout: Some(600),
            r#async: None,
            scope: "user".to_string(),
            enabled: true,
            added_at: "20260203-143022".to_string(),
            installed_by: "test".to_string(),
            description: Some("Test hook".to_string()),
            reason: Some("Testing".to_string()),
            optional: Some(false),
        }];

        write_registry(entries.clone()).expect("write should succeed");

        let read_entries = read_registry().expect("read should succeed");
        assert_eq!(read_entries.len(), 1);
        assert_eq!(read_entries[0].command, "/path/to/test-stop.sh");
        assert_eq!(read_entries[0].event, HookEvent::Stop);
    }

    #[test]
    #[serial(home)]
    fn test_read_nonexistent_registry() {
        let _dir = setup_test_env();
        // Fresh HOME, no registry file exists
        let result = read_registry();
        assert!(result.is_ok());
        assert_eq!(
            result.expect("should be ok").len(),
            0,
            "should return empty vec"
        );
    }

    #[test]
    #[serial(home)]
    fn test_registry_roundtrip_preserves_metadata() {
        let _dir = setup_test_env();

        let original_entries = vec![
            RegistryEntry {
                event: HookEvent::Stop,
                matcher: None,
                r#type: "command".to_string(),
                command: "/path/to/test-stop-roundtrip.sh".to_string(),
                timeout: Some(600),
                r#async: Some(false),
                scope: "user".to_string(),
                enabled: true,
                added_at: "20260203-143022".to_string(),
                installed_by: "test".to_string(),
                description: Some("Stop hook".to_string()),
                reason: Some("For testing".to_string()),
                optional: Some(false),
            },
            RegistryEntry {
                event: HookEvent::SessionStart,
                matcher: None,
                r#type: "command".to_string(),
                command: "/path/to/test-start-roundtrip.sh".to_string(),
                timeout: None,
                r#async: None,
                scope: "user".to_string(),
                enabled: true,
                added_at: "20260203-143023".to_string(),
                installed_by: "test".to_string(),
                description: None,
                reason: None,
                optional: None,
            },
        ];

        write_registry(original_entries.clone()).expect("write failed");
        let read_entries = read_registry().expect("read failed");

        assert_eq!(read_entries.len(), 2);
        assert_eq!(read_entries[0].event, HookEvent::Stop);
        assert_eq!(read_entries[0].command, "/path/to/test-stop-roundtrip.sh");
        assert_eq!(read_entries[0].timeout, Some(600));
        assert_eq!(read_entries[0].description, Some("Stop hook".to_string()));
        assert_eq!(read_entries[1].event, HookEvent::SessionStart);
        assert_eq!(read_entries[1].command, "/path/to/test-start-roundtrip.sh");
        assert_eq!(read_entries[1].timeout, None);
    }
}
