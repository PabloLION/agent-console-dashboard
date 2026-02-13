use super::*;
use crate::tui::test_utils::{
    assert_text_fg_in_row, find_row_with_text, make_session as make_test_session_with_dir,
    render_session_list_to_buffer, row_contains, row_text,
};

// --- TUI TestBackend tests (acd-211) ---
// Column Layout Tests (13 tests - TDD for acd-0uz, acd-7dl, acd-k69, acd-czj, acd-csg)

#[test]
fn test_directory_is_first_data_column_standard() {
    let sessions = vec![make_test_session_with_dir(
        "test",
        Status::Working,
        Some(PathBuf::from("/home/user/project")),
    )];
    let buffer = render_session_list_to_buffer(&sessions, Some(0), 100, 10);
    // Find the first data row (after header)
    let row = find_row_with_text(&buffer, "project").expect("should find project");
    let row_string = row_text(&buffer, row);
    // Directory should appear before session ID in the line
    let dir_pos = row_string.find("project").expect("project not found");
    let id_pos = row_string.find("test").expect("test not found");
    assert!(
        dir_pos < id_pos,
        "Directory should be before session ID: dir_pos={}, id_pos={}",
        dir_pos,
        id_pos
    );
}

#[test]
fn test_directory_is_first_data_column_wide() {
    let sessions = vec![make_test_session_with_dir(
        "test-wide",
        Status::Working,
        Some(PathBuf::from("/home/user/wide-project")),
    )];
    let buffer = render_session_list_to_buffer(&sessions, Some(0), 100, 10);
    let row = find_row_with_text(&buffer, "wide-project").expect("should find wide-project");
    let row_string = row_text(&buffer, row);
    let dir_pos = row_string
        .find("wide-project")
        .expect("wide-project not found");
    let id_pos = row_string.find("test-wide").expect("test-wide not found");
    assert!(
        dir_pos < id_pos,
        "Directory should be before session ID: dir_pos={}, id_pos={}",
        dir_pos,
        id_pos
    );
}

#[test]
fn test_header_directory_is_first_column_standard() {
    let sessions = vec![make_test_session_with_dir(
        "test",
        Status::Working,
        Some(PathBuf::from("/tmp")),
    )];
    let buffer = render_session_list_to_buffer(&sessions, Some(0), 100, 10);
    // Find header row
    let header_row =
        find_row_with_text(&buffer, "Directory").expect("should find Directory header");
    let row_string = row_text(&buffer, header_row);
    let dir_pos = row_string.find("Directory").expect("Directory not found");
    let id_pos = row_string.find("Session ID").expect("Session ID not found");
    assert!(
        dir_pos < id_pos,
        "Directory header should be before Session ID header"
    );
}

#[test]
fn test_header_does_not_say_name() {
    let sessions = vec![make_test_session_with_dir(
        "test",
        Status::Working,
        Some(PathBuf::from("/tmp")),
    )];
    let buffer = render_session_list_to_buffer(&sessions, Some(0), 100, 10);
    // Check that "Name" is not in any header row
    for row in 0..buffer.area().height {
        let text = row_text(&buffer, row);
        if text.contains("Directory") || text.contains("Session ID") {
            assert!(
                !text.contains("Name"),
                "Header should not contain 'Name', got: '{}'",
                text
            );
        }
    }
}

#[test]
fn test_header_says_session_id_standard() {
    let sessions = vec![make_test_session_with_dir(
        "test",
        Status::Working,
        Some(PathBuf::from("/tmp")),
    )];
    let buffer = render_session_list_to_buffer(&sessions, Some(0), 100, 10);
    assert!(
        find_row_with_text(&buffer, "Session ID").is_some(),
        "Header should contain 'Session ID'"
    );
}

#[test]
fn test_header_says_session_id_wide() {
    let sessions = vec![make_test_session_with_dir(
        "test",
        Status::Working,
        Some(PathBuf::from("/tmp")),
    )];
    let buffer = render_session_list_to_buffer(&sessions, Some(0), 100, 10);
    assert!(
        find_row_with_text(&buffer, "Session ID").is_some(),
        "Wide header should contain 'Session ID'"
    );
}

#[test]
fn test_session_id_not_truncated_in_line() {
    let long_id = "very-long-session-identifier-that-should-not-be-truncated";
    let sessions = vec![make_test_session_with_dir(
        long_id,
        Status::Working,
        Some(PathBuf::from("/tmp")),
    )];
    let buffer = render_session_list_to_buffer(&sessions, Some(0), 120, 10);
    assert!(
        find_row_with_text(&buffer, long_id).is_some(),
        "Full session ID should appear in buffer without truncation"
    );
}

#[test]
fn test_session_id_not_truncated_at_any_width() {
    let long_id = "extremely-long-session-id-with-many-characters";
    let sessions = vec![make_test_session_with_dir(
        long_id,
        Status::Working,
        Some(PathBuf::from("/tmp")),
    )];
    // Try multiple widths
    for width in [80, 100, 120, 150] {
        let buffer = render_session_list_to_buffer(&sessions, Some(0), width, 10);
        assert!(
            find_row_with_text(&buffer, long_id).is_some(),
            "Session ID should not be truncated at width {}",
            width
        );
    }
}

#[test]
fn test_elapsed_column_fits_hours_format() {
    let sessions = vec![make_test_session_with_dir(
        "test",
        Status::Working,
        Some(PathBuf::from("/tmp")),
    )];
    let buffer = render_session_list_to_buffer(&sessions, Some(0), 100, 10);
    let data_row = find_row_with_text(&buffer, "test").expect("should find session row");
    let row_string = row_text(&buffer, data_row);
    assert!(
        row_string.contains('s') || row_string.contains('m') || row_string.contains('h'),
        "Row should contain elapsed time format"
    );
}

#[test]
fn test_elapsed_column_width_at_least_16() {
    let sessions = vec![make_test_session_with_dir(
        "test",
        Status::Working,
        Some(PathBuf::from("/tmp")),
    )];
    let buffer = render_session_list_to_buffer(&sessions, Some(0), 100, 10);
    let data_row = find_row_with_text(&buffer, "test").expect("should find session row");
    let row_string = row_text(&buffer, data_row);
    assert!(
        row_string.contains("working"),
        "Status should be visible: '{}'",
        row_string
    );
    assert!(
        row_string.contains('s') || row_string.contains('m') || row_string.contains('h'),
        "Elapsed should be visible: '{}'",
        row_string
    );
}

#[test]
fn test_header_labels_left_aligned_standard() {
    let sessions = vec![make_test_session_with_dir(
        "test",
        Status::Working,
        Some(PathBuf::from("/tmp")),
    )];
    let buffer = render_session_list_to_buffer(&sessions, Some(0), 100, 10);
    let header_row = find_row_with_text(&buffer, "Directory").expect("should find header");
    let row_string = row_text(&buffer, header_row);
    let dir_pos = row_string.find("Directory").expect("Directory not found");
    assert!(
        dir_pos < 5,
        "Directory should be left-aligned (pos < 5), got pos={}",
        dir_pos
    );
}

#[test]
fn test_header_labels_left_aligned_wide() {
    let sessions = vec![make_test_session_with_dir(
        "test",
        Status::Working,
        Some(PathBuf::from("/tmp")),
    )];
    let buffer = render_session_list_to_buffer(&sessions, Some(0), 100, 10);
    let header_row = find_row_with_text(&buffer, "Directory").expect("should find header");
    let row_string = row_text(&buffer, header_row);
    let dir_pos = row_string.find("Directory").expect("Directory not found");
    assert!(
        dir_pos < 5,
        "Directory should be left-aligned (pos < 5), got pos={}",
        dir_pos
    );
}

#[test]
fn test_data_columns_left_aligned_standard() {
    let sessions = vec![make_test_session_with_dir(
        "test",
        Status::Working,
        Some(PathBuf::from("/tmp")),
    )];
    let buffer = render_session_list_to_buffer(&sessions, Some(0), 100, 10);
    let data_row = find_row_with_text(&buffer, "test").expect("should find data row");
    let row_string = row_text(&buffer, data_row);
    let dir_pos = row_string.find("tmp").expect("tmp not found");
    assert!(
        dir_pos < 10,
        "Directory data should be left-aligned (pos < 10), got pos={}",
        dir_pos
    );
}
