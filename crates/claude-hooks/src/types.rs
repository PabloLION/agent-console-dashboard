//! Core domain types for claude-hooks
//!
//! This module defines the types that model Claude Code hooks, including
//! HookEvent, HookHandler, RegistryEntry, and ListEntry.

use serde::{Deserialize, Serialize};

/// Claude Code hook events
///
/// Matches Claude's event names exactly when serialized.
/// See: <https://docs.anthropic.com/en/docs/claude-code/hooks>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HookEvent {
    /// Session begins or resumes
    SessionStart,
    /// User submits prompt
    UserPromptSubmit,
    /// Before tool execution
    PreToolUse,
    /// Permission dialog shown
    PermissionRequest,
    /// After tool succeeds
    PostToolUse,
    /// After tool fails
    PostToolUseFailure,
    /// Notification sent
    Notification,
    /// Subagent spawned
    SubagentStart,
    /// Subagent finishes
    SubagentStop,
    /// Claude finishes response
    Stop,
    /// Before compaction
    PreCompact,
    /// Session terminates
    SessionEnd,
}

/// Hook handler configuration (matches Claude's settings.json structure)
///
/// This is the innermost handler object inside a matcher group's `hooks` array.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookHandler {
    /// Handler type: "command", "prompt", or "agent"
    #[serde(rename = "type")]
    pub r#type: String,
    /// Full command string with arguments (for type="command")
    pub command: String,
    /// Optional timeout in seconds (default 600)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,
    /// Optional async flag (only for PostToolUse/PostToolUseFailure)
    #[serde(skip_serializing_if = "Option::is_none", rename = "async")]
    pub r#async: Option<bool>,
    /// Optional custom spinner message
    #[serde(skip_serializing_if = "Option::is_none", rename = "statusMessage")]
    pub status_message: Option<String>,
}

/// Matcher group in Claude Code hooks structure
///
/// Each event has an array of matcher groups. Each group has an optional
/// `matcher` regex and a `hooks` array of handlers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatcherGroup {
    /// Optional regex matcher to filter when hooks run
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matcher: Option<String>,
    /// Array of hook handlers
    pub hooks: Vec<HookHandler>,
}

/// Registry entry (internal representation with metadata)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryEntry {
    // Identity fields (composite key - D22)
    /// Hook event
    pub event: HookEvent,
    /// Optional matcher regex (None for hooks without matcher)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matcher: Option<String>,
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
        let json = r#""SessionStart""#;
        let event: HookEvent = serde_json::from_str(json).expect("deserialization failed");
        assert_eq!(event, HookEvent::SessionStart);
    }

    #[test]
    fn test_all_hook_events_serialize() {
        let events = vec![
            (HookEvent::SessionStart, r#""SessionStart""#),
            (HookEvent::UserPromptSubmit, r#""UserPromptSubmit""#),
            (HookEvent::PreToolUse, r#""PreToolUse""#),
            (HookEvent::PermissionRequest, r#""PermissionRequest""#),
            (HookEvent::PostToolUse, r#""PostToolUse""#),
            (HookEvent::PostToolUseFailure, r#""PostToolUseFailure""#),
            (HookEvent::Notification, r#""Notification""#),
            (HookEvent::SubagentStart, r#""SubagentStart""#),
            (HookEvent::SubagentStop, r#""SubagentStop""#),
            (HookEvent::Stop, r#""Stop""#),
            (HookEvent::PreCompact, r#""PreCompact""#),
            (HookEvent::SessionEnd, r#""SessionEnd""#),
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
            timeout: Some(600),
            r#async: None,
            status_message: None,
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
            timeout: Some(300),
            r#async: Some(true),
            status_message: Some("Running validation...".to_string()),
        };
        let json = serde_json::to_string(&handler_full).expect("serialization failed");
        let deserialized: HookHandler =
            serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(handler_full, deserialized);

        // Test with all optional fields absent
        let handler_minimal = HookHandler {
            r#type: "command".to_string(),
            command: "/path/to/script.sh".to_string(),
            timeout: None,
            r#async: None,
            status_message: None,
        };
        let json = serde_json::to_string(&handler_minimal).expect("serialization failed");
        let deserialized: HookHandler =
            serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(handler_minimal, deserialized);
    }

    #[test]
    fn test_matcher_group_roundtrip() {
        let group = MatcherGroup {
            matcher: Some("Bash".to_string()),
            hooks: vec![HookHandler {
                r#type: "command".to_string(),
                command: "/path/to/script.sh".to_string(),
                timeout: Some(10),
                r#async: None,
                status_message: None,
            }],
        };
        let json = serde_json::to_string(&group).expect("serialization failed");
        let deserialized: MatcherGroup =
            serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(group, deserialized);
    }

    #[test]
    fn test_matcher_group_without_matcher() {
        let group = MatcherGroup {
            matcher: None,
            hooks: vec![HookHandler {
                r#type: "command".to_string(),
                command: "/path/to/script.sh".to_string(),
                timeout: None,
                r#async: None,
                status_message: None,
            }],
        };
        let json = serde_json::to_string(&group).expect("serialization failed");
        assert!(
            !json.contains("matcher"),
            "matcher should be omitted when None"
        );
        let deserialized: MatcherGroup =
            serde_json::from_str(&json).expect("deserialization failed");
        assert_eq!(group, deserialized);
    }

    #[test]
    fn test_registry_entry_roundtrip() {
        let entry = RegistryEntry {
            event: HookEvent::Stop,
            matcher: None,
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

        // Should match same event and command
        assert!(entry.matches(HookEvent::Stop, "/path/to/stop.sh"));

        // Should not match different command
        assert!(!entry.matches(HookEvent::Stop, "/different/path"));

        // Should not match different event
        assert!(!entry.matches(HookEvent::SessionStart, "/path/to/stop.sh"));

        // Should not match both different
        assert!(!entry.matches(HookEvent::SessionStart, "/different/path"));
    }
}
