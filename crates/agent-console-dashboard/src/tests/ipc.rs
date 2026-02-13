//! Tests for IPC types and SessionMetadata.

use crate::*;

#[test]
fn test_session_metadata_default() {
    let metadata = SessionMetadata::default();
    assert!(metadata.working_dir.is_none());
    assert!(metadata.session_id.is_none());
    assert!(metadata.agent_type.is_none());
}

#[test]
fn test_session_metadata_new() {
    let metadata = SessionMetadata::new();
    assert!(metadata.working_dir.is_none());
    assert!(metadata.session_id.is_none());
    assert!(metadata.agent_type.is_none());
}

#[test]
fn test_session_metadata_with_working_dir() {
    let metadata = SessionMetadata::with_working_dir(PathBuf::from("/home/user/project"));
    assert_eq!(
        metadata.working_dir,
        Some(PathBuf::from("/home/user/project"))
    );
    assert!(metadata.session_id.is_none());
    assert!(metadata.agent_type.is_none());
}

#[test]
fn test_session_metadata_builder_pattern() {
    let metadata = SessionMetadata::new()
        .working_dir(PathBuf::from("/tmp/test"))
        .session_id("session-123".to_string())
        .agent_type(AgentType::ClaudeCode);

    assert_eq!(metadata.working_dir, Some(PathBuf::from("/tmp/test")));
    assert_eq!(metadata.session_id, Some("session-123".to_string()));
    assert_eq!(metadata.agent_type, Some(AgentType::ClaudeCode));
}

#[test]
fn test_session_metadata_clone() {
    let metadata = SessionMetadata::new()
        .working_dir(PathBuf::from("/clone/test"))
        .session_id("clone-session".to_string());

    let cloned = metadata.clone();
    assert_eq!(cloned.working_dir, metadata.working_dir);
    assert_eq!(cloned.session_id, metadata.session_id);
    assert_eq!(cloned.agent_type, metadata.agent_type);
}

#[test]
fn test_session_metadata_debug_format() {
    let metadata = SessionMetadata::new()
        .working_dir(PathBuf::from("/debug/path"))
        .session_id("debug-session".to_string());

    let debug_str = format!("{:?}", metadata);
    assert!(debug_str.contains("SessionMetadata"));
    assert!(debug_str.contains("/debug/path"));
    assert!(debug_str.contains("debug-session"));
}

#[test]
fn test_session_metadata_partial_fields() {
    // Test with only working_dir
    let metadata1 = SessionMetadata {
        working_dir: Some(PathBuf::from("/only/working/dir")),
        session_id: None,
        agent_type: None,
    };
    assert!(metadata1.working_dir.is_some());
    assert!(metadata1.session_id.is_none());

    // Test with only session_id
    let metadata2 = SessionMetadata {
        working_dir: None,
        session_id: Some("only-session-id".to_string()),
        agent_type: None,
    };
    assert!(metadata2.working_dir.is_none());
    assert!(metadata2.session_id.is_some());

    // Test with only agent_type
    let metadata3 = SessionMetadata {
        working_dir: None,
        session_id: None,
        agent_type: Some(AgentType::ClaudeCode),
    };
    assert!(metadata3.working_dir.is_none());
    assert!(metadata3.agent_type.is_some());
}

#[test]
fn test_session_update_new() {
    let update = SessionUpdate::new("session-1".to_string(), Status::Working, 120);
    assert_eq!(update.session_id, "session-1");
    assert_eq!(update.status, Status::Working);
    assert_eq!(update.elapsed_seconds, 120);
}

#[test]
fn test_session_update_clone() {
    let update = SessionUpdate::new("clone-test".to_string(), Status::Attention, 60);
    let cloned = update.clone();
    assert_eq!(cloned.session_id, update.session_id);
    assert_eq!(cloned.status, update.status);
    assert_eq!(cloned.elapsed_seconds, update.elapsed_seconds);
}

#[test]
fn test_session_update_equality() {
    let update1 = SessionUpdate::new("eq-test".to_string(), Status::Question, 30);
    let update2 = SessionUpdate::new("eq-test".to_string(), Status::Question, 30);
    let update3 = SessionUpdate::new("eq-test".to_string(), Status::Working, 30);
    assert_eq!(update1, update2);
    assert_ne!(update1, update3);
}

#[test]
fn test_session_update_debug_format() {
    let update = SessionUpdate::new("debug-test".to_string(), Status::Closed, 45);
    let debug_str = format!("{:?}", update);
    assert!(debug_str.contains("debug-test"));
    assert!(debug_str.contains("Closed"));
    assert!(debug_str.contains("45"));
}

#[test]
fn test_session_update_all_statuses() {
    for status in [
        Status::Working,
        Status::Attention,
        Status::Question,
        Status::Closed,
    ] {
        let update = SessionUpdate::new("status-test".to_string(), status, 0);
        assert_eq!(update.status, status);
    }
}
