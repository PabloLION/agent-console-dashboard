use super::*;

// --- Story 4 (acd-9ul): Basename disambiguation tests ---

#[test]
fn test_compute_directory_display_names_unique_basenames() {
    let sessions = vec![
        Session::new(
            "s1".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/home/user/project-a")),
        ),
        Session::new(
            "s2".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/home/user/project-b")),
        ),
    ];
    let display_names = compute_directory_display_names(&sessions);
    assert_eq!(display_names.get("s1"), Some(&"project-a".to_string()));
    assert_eq!(display_names.get("s2"), Some(&"project-b".to_string()));
}

#[test]
fn test_compute_directory_display_names_duplicate_basenames() {
    let sessions = vec![
        Session::new(
            "s1".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/home/user/project")),
        ),
        Session::new(
            "s2".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/work/client/project")),
        ),
    ];
    let display_names = compute_directory_display_names(&sessions);
    // Both should have parent/basename format since basename "project" is duplicated
    assert_eq!(display_names.get("s1"), Some(&"user/project".to_string()));
    assert_eq!(display_names.get("s2"), Some(&"client/project".to_string()));
}

#[test]
fn test_compute_directory_display_names_mixed() {
    let sessions = vec![
        Session::new(
            "s1".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/home/user/project")),
        ),
        Session::new(
            "s2".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/work/client/project")),
        ),
        Session::new(
            "s3".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/tmp/unique-name")),
        ),
    ];
    let display_names = compute_directory_display_names(&sessions);
    // s1 and s2 have duplicate basename, need disambiguation
    assert_eq!(display_names.get("s1"), Some(&"user/project".to_string()));
    assert_eq!(display_names.get("s2"), Some(&"client/project".to_string()));
    // s3 is unique
    assert_eq!(display_names.get("s3"), Some(&"unique-name".to_string()));
}

#[test]
fn test_compute_directory_display_names_unknown_paths() {
    let sessions = vec![
        Session::new("s1".to_string(), AgentType::ClaudeCode, None),
        Session::new(
            "s2".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/home/user/project")),
        ),
    ];
    let display_names = compute_directory_display_names(&sessions);
    // Unknown path should map to <error>
    assert_eq!(display_names.get("s1"), Some(&"<error>".to_string()));
    // Normal path should show basename
    assert_eq!(display_names.get("s2"), Some(&"project".to_string()));
}

#[test]
fn test_compute_directory_display_names_root_path() {
    let sessions = vec![Session::new(
        "s1".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/")),
    )];
    let display_names = compute_directory_display_names(&sessions);
    // Root path has no file_name(), should fall back to <error>
    assert_eq!(display_names.get("s1"), Some(&"<error>".to_string()));
}

#[test]
fn test_compute_directory_display_names_three_duplicate_basenames() {
    let sessions = vec![
        Session::new(
            "s1".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/home/user/project")),
        ),
        Session::new(
            "s2".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/work/client/project")),
        ),
        Session::new(
            "s3".to_string(),
            AgentType::ClaudeCode,
            Some(PathBuf::from("/opt/build/project")),
        ),
    ];
    let display_names = compute_directory_display_names(&sessions);
    // All three should show parent/basename since "project" appears 3 times
    assert_eq!(display_names.get("s1"), Some(&"user/project".to_string()));
    assert_eq!(display_names.get("s2"), Some(&"client/project".to_string()));
    assert_eq!(display_names.get("s3"), Some(&"build/project".to_string()));
}

#[test]
fn test_disambiguation_parent_collision() {
    // Same basename AND same immediate parent
    let s1 = make_test_session("s1", Some(PathBuf::from("/home/alice/project")));
    let s2 = make_test_session("s2", Some(PathBuf::from("/work/alice/project")));
    let sessions = vec![s1, s2];
    let names = compute_directory_display_names(&sessions);

    // Verify they are different
    assert_ne!(
        names.get("s1"),
        names.get("s2"),
        "Colliding parent/basename should be disambiguated"
    );

    // Verify exact output values with grandparent level
    assert_eq!(
        names.get("s1"),
        Some(&"home/alice/project".to_string()),
        "s1 should show home/alice/project"
    );
    assert_eq!(
        names.get("s2"),
        Some(&"work/alice/project".to_string()),
        "s2 should show work/alice/project"
    );
}

#[test]
fn test_disambiguation_three_level_collision() {
    // Three sessions sharing basename AND 2 parent levels
    let s1 = make_test_session("s1", Some(PathBuf::from("/a/shared/parent/project")));
    let s2 = make_test_session("s2", Some(PathBuf::from("/b/shared/parent/project")));
    let sessions = vec![s1, s2];
    let names = compute_directory_display_names(&sessions);

    // Should disambiguate with 3-level path
    assert_eq!(
        names.get("s1"),
        Some(&"a/shared/parent/project".to_string()),
        "s1 should show a/shared/parent/project"
    );
    assert_eq!(
        names.get("s2"),
        Some(&"b/shared/parent/project".to_string()),
        "s2 should show b/shared/parent/project"
    );
}

#[test]
fn test_disambiguation_single_component_path() {
    // Path with only root + one component (no parent to add)
    let s1 = make_test_session("s1", Some(PathBuf::from("/project")));
    let s2 = make_test_session("s2", Some(PathBuf::from("/other")));
    let sessions = vec![s1, s2];
    let names = compute_directory_display_names(&sessions);

    // Different basenames, no collision - should show basenames
    assert_eq!(
        names.get("s1"),
        Some(&"project".to_string()),
        "s1 should show project"
    );
    assert_eq!(
        names.get("s2"),
        Some(&"other".to_string()),
        "s2 should show other"
    );
}

#[test]
fn test_disambiguation_identical_paths() {
    // Two sessions with exact same path - must handle gracefully
    let s1 = make_test_session("s1", Some(PathBuf::from("/home/user/project")));
    let s2 = make_test_session("s2", Some(PathBuf::from("/home/user/project")));
    let sessions = vec![s1, s2];
    let names = compute_directory_display_names(&sessions);

    // Both should show the same display name (fall back to full path)
    let name1 = names.get("s1").expect("s1 should have a display name");
    let name2 = names.get("s2").expect("s2 should have a display name");
    assert_eq!(
        name1, name2,
        "Identical paths should produce identical display names"
    );
    // Should be the full path as fallback
    assert_eq!(
        name1, "/home/user/project",
        "Identical paths should fall back to full path"
    );
}

#[test]
fn test_disambiguation_none_mixed_with_collisions() {
    // None session should not interfere with real path disambiguation
    let s1 = make_test_session("s1", None);
    let s2 = make_test_session("s2", Some(PathBuf::from("/work/project")));
    let s3 = make_test_session("s3", Some(PathBuf::from("/home/project")));
    let sessions = vec![s1, s2, s3];
    let names = compute_directory_display_names(&sessions);

    // s1 (None) should show <error>
    assert_eq!(
        names.get("s1"),
        Some(&"<error>".to_string()),
        "None path should show <error>"
    );

    // s2 and s3 share basename "project" and should be disambiguated
    assert_eq!(
        names.get("s2"),
        Some(&"work/project".to_string()),
        "s2 should show work/project"
    );
    assert_eq!(
        names.get("s3"),
        Some(&"home/project".to_string()),
        "s3 should show home/project"
    );
}
