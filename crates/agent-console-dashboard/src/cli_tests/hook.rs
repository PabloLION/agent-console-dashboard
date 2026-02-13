//! Hook validation tests.

use crate::commands::hook::{validate_hook_input, HookInput};

#[test]
fn test_validate_hook_input_valid() {
    let input = HookInput {
        session_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        cwd: "/home/user/project".to_string(),
    };
    let warnings = validate_hook_input(&input);
    assert!(warnings.is_empty(), "valid input should have no warnings");
}

#[test]
fn test_validate_hook_input_invalid_session_id_length() {
    let input = HookInput {
        session_id: "short".to_string(),
        cwd: "/home/user/project".to_string(),
    };
    let warnings = validate_hook_input(&input);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("session_id length is 5"));
    assert!(warnings[0].contains("(expected 36)"));
}

#[test]
fn test_validate_hook_input_invalid_session_id_chars() {
    let input = HookInput {
        session_id: "550e8400-e29b-41d4-a716-44665544000G".to_string(),
        cwd: "/home/user/project".to_string(),
    };
    let warnings = validate_hook_input(&input);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("session_id contains invalid characters"));
}

#[test]
fn test_validate_hook_input_empty_cwd() {
    let input = HookInput {
        session_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        cwd: "".to_string(),
    };
    let warnings = validate_hook_input(&input);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("cwd is empty"));
}

#[test]
fn test_validate_hook_input_relative_cwd() {
    let input = HookInput {
        session_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        cwd: "relative/path".to_string(),
    };
    let warnings = validate_hook_input(&input);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("cwd is not an absolute path"));
    assert!(warnings[0].contains("relative/path"));
}

#[test]
fn test_validate_hook_input_multiple_invalid_fields() {
    let input = HookInput {
        session_id: "short".to_string(),
        cwd: "relative".to_string(),
    };
    let warnings = validate_hook_input(&input);
    assert_eq!(warnings.len(), 2);
    assert!(warnings.iter().any(|w| w.contains("session_id")));
    assert!(warnings.iter().any(|w| w.contains("cwd")));
}

#[test]
fn test_validate_hook_input_uppercase_hex_valid() {
    let input = HookInput {
        session_id: "550E8400-E29B-41D4-A716-446655440000".to_string(),
        cwd: "/home/user/project".to_string(),
    };
    let warnings = validate_hook_input(&input);
    assert!(warnings.is_empty(), "uppercase hex should be valid");
}

#[test]
fn test_validate_hook_input_all_dashes_weird_but_passes() {
    let input = HookInput {
        session_id: "------------------------------------".to_string(),
        cwd: "/home/user/project".to_string(),
    };
    let warnings = validate_hook_input(&input);
    assert!(warnings.is_empty(), "36 dashes passes charset validation");
}

#[test]
fn test_validate_hook_input_cwd_with_spaces_valid() {
    let input = HookInput {
        session_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        cwd: "/home/user/my project".to_string(),
    };
    let warnings = validate_hook_input(&input);
    assert!(warnings.is_empty(), "absolute path with spaces is valid");
}
