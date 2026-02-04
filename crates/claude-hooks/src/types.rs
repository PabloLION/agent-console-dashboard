//! Core domain types for claude-hooks
//!
//! This module defines the types that model Claude Code hooks, including
//! HookEvent, HookHandler, RegistryEntry, and ListEntry.

use serde::{Deserialize, Serialize};

/// Claude Code hook events
///
/// Matches Claude's event names exactly when serialized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HookEvent {
    /// Agent starts
    Start,
    /// Agent stops
    Stop,
    /// Before prompt input
    BeforePrompt,
    /// After prompt input
    AfterPrompt,
    /// Before tool use
    BeforeToolUse,
    /// After tool use
    AfterToolUse,
    /// Before edit
    BeforeEdit,
    /// After edit
    AfterEdit,
    /// Before revert
    BeforeRevert,
    /// After revert
    AfterRevert,
    /// Before run
    BeforeRun,
    /// After run
    AfterRun,
}

/// Hook handler configuration (matches Claude's settings.json structure)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookHandler {
    /// Handler type (e.g., "command" in v0.1)
    #[serde(rename = "type")]
    pub r#type: String,
    /// Full command string with arguments
    pub command: String,
    /// Matcher string (empty for global hooks)
    pub matcher: String,
    /// Optional timeout in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,
    /// Optional async flag
    #[serde(skip_serializing_if = "Option::is_none", rename = "async")]
    pub r#async: Option<bool>,
}

/// Registry entry (internal representation with metadata)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryEntry {
    // Identity fields (composite key - D22)
    /// Hook event
    pub event: HookEvent,
    /// Matcher string
    pub matcher: String,
    /// Handler type
    #[serde(rename = "type")]
    pub r#type: String,
    /// Command string
    pub command: String,

    // Configuration fields (not part of identity)
    /// Optional timeout in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,
    /// Optional async flag
    #[serde(skip_serializing_if = "Option::is_none", rename = "async")]
    pub r#async: Option<bool>,

    // Metadata fields
    /// Scope (e.g., "user" in v0.1)
    pub scope: String,
    /// Whether hook is enabled
    pub enabled: bool,
    /// Timestamp when hook was added (yyyyMMdd-hhmmss)
    pub added_at: String,
    /// Free-form string identifying installer (D24)
    pub installed_by: String,
    /// Optional description of what the hook does
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional reason why the hook was added
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Optional flag for whether hook is optional
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
}

impl RegistryEntry {
    /// Check if this entry matches the given event and command (composite key)
    ///
    /// This implements the composite key matching logic from D22.
    pub fn matches(&self, event: HookEvent, command: &str) -> bool {
        self.event == event && self.command == command
    }
}

/// Entry returned by list() function
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListEntry {
    /// Hook event
    pub event: HookEvent,
    /// Hook handler configuration
    pub handler: HookHandler,
    /// True if we installed this hook
    pub managed: bool,
    /// Present if managed, contains registry metadata
    pub metadata: Option<RegistryMetadata>,
}

/// Subset of registry metadata for list output
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryMetadata {
    /// Timestamp when hook was added
    pub added_at: String,
    /// Free-form string identifying installer
    pub installed_by: String,
    /// Optional description
    pub description: Option<String>,
    /// Optional reason
    pub reason: Option<String>,
    /// Optional flag
    pub optional: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_event_serialization() {
        let event = HookEvent::Stop;
        let json = serde_json::to_string(&event).expect("serialization failed");
        assert_eq!(json, r#""Stop""#);
    }

    #[test]
    fn test_hook_event_deserialization() {
        let json = r#""Start""#;
        let event: HookEvent = serde_json::from_str(json).expect("deserialization failed");
        assert_eq!(event, HookEvent::Start);
    }

    #[test]
    fn test_all_hook_events_serialize() {
        let events = vec![
            (HookEvent::Start, r#""Start""#),
            (HookEvent::Stop, r#""Stop""#),
            (HookEvent::BeforePrompt, r#""BeforePrompt""#),
            (HookEvent::AfterPrompt, r#""AfterPrompt""#),
            (HookEvent::BeforeToolUse, r#""BeforeToolUse""#),
            (HookEvent::AfterToolUse, r#""AfterToolUse""#),
            (HookEvent::BeforeEdit, r#""BeforeEdit""#),
            (HookEvent::AfterEdit, r#""AfterEdit""#),
            (HookEvent::BeforeRevert, r#""BeforeRevert""#),
            (HookEvent::AfterRevert, r#""AfterRevert""#),
            (HookEvent::BeforeRun, r#""BeforeRun""#),
            (HookEvent::AfterRun, r#""AfterRun""#),
        ];

        for (event, expected) in events {
            let json = serde_json::to_string(&event).expect("serialization failed");
            assert_eq!(json, expected, "Event {:?} serialized incorrectly", event);
        }
    }

    #[test]
    fn test_hook_handler_roundtrip() {
        let handler = HookHandler {
            r#type: "command".to_string(),
            command: "/path/to/stop.sh".to_string(),
            matcher: String::new(),
            timeout: Some(600),
            r#async: None,
        };
        let json = serde_json::to_string(&handler).expect("serialization failed");
        let deserialized: HookHandler =
            serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(handler, deserialized);
    }

    #[test]
    fn test_hook_handler_optional_fields() {
        // Test with all optional fields present
        let handler_full = HookHandler {
            r#type: "command".to_string(),
            command: "/path/to/script.sh".to_string(),
            matcher: String::new(),
            timeout: Some(300),
            r#async: Some(true),
        };
        let json = serde_json::to_string(&handler_full).expect("serialization failed");
        let deserialized: HookHandler =
            serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(handler_full, deserialized);

        // Test with all optional fields absent
        let handler_minimal = HookHandler {
            r#type: "command".to_string(),
            command: "/path/to/script.sh".to_string(),
            matcher: String::new(),
            timeout: None,
            r#async: None,
        };
        let json = serde_json::to_string(&handler_minimal).expect("serialization failed");
        let deserialized: HookHandler =
            serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(handler_minimal, deserialized);
    }

    #[test]
    fn test_registry_entry_roundtrip() {
        let entry = RegistryEntry {
            event: HookEvent::Stop,
            matcher: String::new(),
            r#type: "command".to_string(),
            command: "/path/to/stop.sh".to_string(),
            timeout: Some(600),
            r#async: None,
            scope: "user".to_string(),
            enabled: true,
            added_at: "20260203-143022".to_string(),
            installed_by: "acd".to_string(),
            description: Some("Test hook".to_string()),
            reason: Some("Testing".to_string()),
            optional: Some(false),
        };
        let json = serde_json::to_string(&entry).expect("serialization failed");
        let deserialized: RegistryEntry =
            serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(entry, deserialized);
    }

    #[test]
    fn test_registry_entry_matches() {
        let entry = RegistryEntry {
            event: HookEvent::Stop,
            matcher: String::new(),
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

        // Should match same event and command
        assert!(entry.matches(HookEvent::Stop, "/path/to/stop.sh"));

        // Should not match different command
        assert!(!entry.matches(HookEvent::Stop, "/different/path"));

        // Should not match different event
        assert!(!entry.matches(HookEvent::Start, "/path/to/stop.sh"));

        // Should not match both different
        assert!(!entry.matches(HookEvent::Start, "/different/path"));
    }
}
