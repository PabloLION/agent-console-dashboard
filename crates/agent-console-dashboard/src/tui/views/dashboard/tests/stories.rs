use super::*;

// --- Story 1 (acd-lht): Red error for missing CWD tests ---

#[test]
fn test_format_session_line_unknown_working_dir_shows_error_standard() {
    let mut session = Session::new(
        "error-test".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("unknown")),
    );
    session.status = Status::Working;
    let line = format_session_line(&session, 100, "<error>", false);

    // Should have 4 spans: work_dir (error), session ID, status, elapsed
    assert_eq!(line.spans.len(), 4);

    // The work_dir span (index 0) should contain "<error>" and be styled with red
    let work_dir_span = &line.spans[0];
    assert!(
        work_dir_span.content.contains("<error>"),
        "Expected '<error>' in work_dir span, got: '{}'",
        work_dir_span.content
    );
    assert_eq!(
        work_dir_span.style.fg,
        Some(error_color()),
        "Expected error color (red) for <error> span"
    );
}

#[test]
fn test_format_session_line_unknown_working_dir_shows_error_wide() {
    let mut session = Session::new(
        "error-wide-test".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("unknown")),
    );
    session.status = Status::Attention;
    let line = format_session_line(&session, 100, "<error>", false);

    // Should have 4 spans: work_dir (error), session ID, status, elapsed
    assert_eq!(line.spans.len(), 4);

    // The work_dir span (index 0) should contain "<error>" and be styled with red
    let work_dir_span = &line.spans[0];
    assert!(
        work_dir_span.content.contains("<error>"),
        "Expected '<error>' in work_dir span, got: '{}'",
        work_dir_span.content
    );
    assert_eq!(
        work_dir_span.style.fg,
        Some(error_color()),
        "Expected error color (red) for <error> span"
    );
}

#[test]
fn test_format_session_line_normal_path_unchanged() {
    let session = Session::new(
        "normal-path-test".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/home/user/project")),
    );
    let line = format_session_line(&session, 100, "project", false);

    // Should have 4 spans
    assert_eq!(line.spans.len(), 4);

    // The work_dir span (index 0) should contain the path, not "<error>"
    let work_dir_span = &line.spans[0];
    assert!(
        !work_dir_span.content.contains("<error>"),
        "Normal path should not display <error>, got: '{}'",
        work_dir_span.content
    );
    assert!(
        work_dir_span.content.contains("project"),
        "Expected path to contain 'project', got: '{}'",
        work_dir_span.content
    );
    // Should not be red
    assert_ne!(
        work_dir_span.style.fg,
        Some(error_color()),
        "Normal path should not use error color"
    );
}

// --- Story 2 (acd-r57): Column alignment tests ---

#[test]
fn test_column_alignment_standard_width() {
    let session = Session::new(
        "align-test".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/home/user/project")),
    );
    let line = format_session_line(&session, 100, "project", false);

    // Should have 4 spans: workdir, session ID, status, elapsed
    assert_eq!(line.spans.len(), 4);

    // Session ID (index 1) should be left-aligned with width 40
    let session_id_span = &line.spans[1];
    assert_eq!(
        session_id_span.content.len(),
        40,
        "Session ID should have width 40, got: '{}'",
        session_id_span.content
    );

    // Status (index 2) should be left-aligned with width 14
    let status_span = &line.spans[2];
    assert_eq!(
        status_span.content.len(),
        14,
        "Status should have width 14, got: '{}'",
        status_span.content
    );
    assert!(
        !status_span.content.starts_with(' '),
        "Status should be left-aligned, got: '{}'",
        status_span.content
    );

    // Elapsed (index 3) should be left-aligned with width 16
    let elapsed_span = &line.spans[3];
    assert_eq!(
        elapsed_span.content.len(),
        16,
        "Elapsed should have width 16, got: '{}'",
        elapsed_span.content
    );
    assert!(
        !elapsed_span.content.starts_with(' '),
        "Elapsed should be left-aligned, got: '{}'",
        elapsed_span.content
    );
}

#[test]
fn test_column_alignment_wide_width() {
    let session = Session::new(
        "wide-align-test".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/home/user/project")),
    );
    let line = format_session_line(&session, 120, "project", false);

    // Should have 4 spans: workdir, session ID, status, elapsed
    assert_eq!(line.spans.len(), 4);

    // Session ID (index 1) should be left-aligned with width 40
    let session_id_span = &line.spans[1];
    assert_eq!(session_id_span.content.len(), 40);

    // Status (index 2) should be left-aligned with width 14
    let status_span = &line.spans[2];
    assert_eq!(status_span.content.len(), 14);

    // Elapsed (index 3) should be left-aligned with width 16
    let elapsed_span = &line.spans[3];
    assert_eq!(elapsed_span.content.len(), 16);
}

#[test]
fn test_directory_column_expands_with_terminal_width() {
    let session = Session::new(
        "expand-test".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/tmp")),
    );

    // Test at standard width (100 cols)
    let line_100 = format_session_line(&session, 100, "tmp", false);
    let dir_span_100 = &line_100.spans[0];

    // Test at wider width (150 cols)
    let line_150 = format_session_line(&session, 150, "tmp", false);
    let dir_span_150 = &line_150.spans[0];

    // Directory column at 150 should be wider than at 100
    assert!(
        dir_span_150.content.len() > dir_span_100.content.len(),
        "Directory column should expand with terminal width: 100={}, 150={}",
        dir_span_100.content.len(),
        dir_span_150.content.len()
    );
}

// --- Story 3 (acd-8uw): Column headers tests ---

#[test]
fn test_header_narrow_mode_no_header() {
    let line = format_header_line(30);
    // Narrow mode should have no header
    assert_eq!(line.spans.len(), 0, "Narrow mode should have no header");
}

#[test]
fn test_header_standard_mode() {
    let line = format_header_line(60);
    // Standard mode: symbol space + Directory + Session ID + Status + Elapsed = 5 spans
    assert_eq!(
        line.spans.len(),
        5,
        "Standard mode should have 5 header spans"
    );

    // Verify header contains expected column titles
    let full_text = line
        .spans
        .iter()
        .map(|s| s.content.as_ref())
        .collect::<Vec<_>>()
        .join("");
    assert!(
        full_text.contains("Session ID"),
        "Header should contain 'Session ID'"
    );
    assert!(
        full_text.contains("Status"),
        "Header should contain 'Status'"
    );
    assert!(
        full_text.contains("Directory"),
        "Header should contain 'Directory'"
    );
    assert!(
        full_text.contains("Time Elapsed"),
        "Header should contain 'Time Elapsed'"
    );

    // Verify all spans use header style (cyan + bold)
    for span in &line.spans {
        assert_eq!(
            span.style.fg,
            Some(Color::Cyan),
            "Header span should use cyan color"
        );
        assert!(
            span.style.add_modifier.contains(Modifier::BOLD),
            "Header span should be bold"
        );
    }
}

#[test]
fn test_header_wide_mode_same_columns_wider_directory() {
    let line = format_header_line(100);
    // Wide mode: same 5 spans as standard (symbol space + Directory + Session ID + Status + Elapsed)
    assert_eq!(line.spans.len(), 5, "Wide mode should have 5 header spans");

    // Verify header contains expected column titles
    let full_text = line
        .spans
        .iter()
        .map(|s| s.content.as_ref())
        .collect::<Vec<_>>()
        .join("");
    assert!(
        full_text.contains("Session ID"),
        "Header should contain 'Session ID'"
    );
    assert!(
        full_text.contains("Status"),
        "Header should contain 'Status'"
    );
    assert!(
        full_text.contains("Directory"),
        "Header should contain 'Directory'"
    );
    assert!(
        full_text.contains("Time Elapsed"),
        "Header should contain 'Time Elapsed'"
    );

    // Wide directory header should be wider than standard
    let standard_line = format_header_line(60);
    let standard_dir = &standard_line.spans[1]; // Directory span
    let wide_dir = &line.spans[1]; // Directory span
    assert!(
        wide_dir.content.len() > standard_dir.content.len(),
        "Wide directory header should be wider: standard={}, wide={}",
        standard_dir.content.len(),
        wide_dir.content.len()
    );
}

#[test]
fn test_header_labels_are_left_aligned() {
    let line = format_header_line(60);

    // Directory (index 1): check left-aligned (starts with "D", not space)
    let dir_span = &line.spans[1];
    assert!(
        dir_span.content.starts_with('D'),
        "Directory header should be left-aligned, got: '{}'",
        dir_span.content
    );

    // Session ID (index 2): check left-aligned (starts with "S", not space)
    let id_span = &line.spans[2];
    assert!(
        id_span.content.starts_with('S'),
        "Session ID header should be left-aligned, got: '{}'",
        id_span.content
    );

    // Status (index 3): check left-aligned (starts with "S", not space)
    let status_span = &line.spans[3];
    assert!(
        status_span.content.starts_with('S'),
        "Status header should be left-aligned, got: '{}'",
        status_span.content
    );

    // Time Elapsed (index 4): check left-aligned (starts with "T", not space)
    let elapsed_span = &line.spans[4];
    assert!(
        elapsed_span.content.starts_with('T'),
        "Time Elapsed header should be left-aligned, got: '{}'",
        elapsed_span.content
    );
}

#[test]
fn test_header_alignment_matches_data() {
    // Verify that header columns align with data columns at standard width
    let header = format_header_line(100);
    let session = Session::new(
        "align-check".to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/tmp/test")),
    );
    let data_line = format_session_line(&session, 100, "test", false);

    // Header has 5 spans (padding + 4 columns), data has 4 spans (columns only).
    // The header "  " padding aligns with ratatui's highlight symbol space.
    assert_eq!(header.spans.len(), 5, "Header should have 5 spans");
    assert_eq!(data_line.spans.len(), 4, "Data should have 4 spans");

    // Verify column widths match (header offset +1 for padding span)
    // Directory: header[1] == data[0]
    assert_eq!(
        header.spans[1].content.len(),
        data_line.spans[0].content.len()
    );

    // Session ID: header[2] == data[1], both fixed 40
    assert_eq!(header.spans[2].content.len(), 40);
    assert_eq!(data_line.spans[1].content.len(), 40);

    // Status: header[3] == data[2], both fixed 14
    assert_eq!(header.spans[3].content.len(), 14);
    assert_eq!(data_line.spans[2].content.len(), 14);

    // Time Elapsed: header[4] == data[3], both fixed 16
    assert_eq!(header.spans[4].content.len(), 16);
    assert_eq!(data_line.spans[3].content.len(), 16);
}
