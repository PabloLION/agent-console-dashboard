//! Serialization roundtrip tests for health and dump types.

use crate::*;

#[test]
fn test_health_status_serialization_roundtrip() {
    let health = HealthStatus {
        uptime_seconds: 9240,
        sessions: SessionCounts {
            active: 3,
            closed: 1,
        },
        connections: 2,
        memory_mb: Some(2.1),
        socket_path: "/tmp/acd.sock".to_string(),
    };

    let json = serde_json::to_string(&health).expect("failed to serialize HealthStatus");
    let parsed: HealthStatus =
        serde_json::from_str(&json).expect("failed to deserialize HealthStatus");

    assert_eq!(parsed.uptime_seconds, 9240);
    assert_eq!(parsed.sessions.active, 3);
    assert_eq!(parsed.sessions.closed, 1);
    assert_eq!(parsed.connections, 2);
    assert_eq!(parsed.memory_mb, Some(2.1));
    assert_eq!(parsed.socket_path, "/tmp/acd.sock");
}

#[test]
fn test_health_status_memory_none() {
    let health = HealthStatus {
        uptime_seconds: 0,
        sessions: SessionCounts {
            active: 0,
            closed: 0,
        },
        connections: 0,
        memory_mb: None,
        socket_path: "/tmp/test.sock".to_string(),
    };

    let json = serde_json::to_string(&health).expect("failed to serialize HealthStatus");
    assert!(json.contains("\"memory_mb\":null"));

    let parsed: HealthStatus =
        serde_json::from_str(&json).expect("failed to deserialize HealthStatus");
    assert!(parsed.memory_mb.is_none());
}

#[test]
fn test_daemon_dump_serialization_roundtrip() {
    let dump = DaemonDump {
        uptime_seconds: 3600,
        socket_path: "/tmp/test.sock".to_string(),
        sessions: vec![DumpSession {
            session_id: "session-1".to_string(),
            status: "working".to_string(),
            working_dir: Some("/home/user/project".to_string()),
            elapsed_seconds: 120,
            closed: false,
        }],
        session_counts: SessionCounts {
            active: 1,
            closed: 0,
        },
        connections: 2,
    };

    let json = serde_json::to_string(&dump).expect("failed to serialize DaemonDump");
    let parsed: DaemonDump =
        serde_json::from_str(&json).expect("failed to deserialize DaemonDump");
    assert_eq!(parsed, dump);
}

#[test]
fn test_dump_session_serialization() {
    let entry = DumpSession {
        session_id: "snap-1".to_string(),
        status: "attention".to_string(),
        working_dir: Some("/tmp/work".to_string()),
        elapsed_seconds: 45,
        closed: true,
    };

    let json = serde_json::to_string(&entry).expect("failed to serialize DumpSession");
    let parsed: DumpSession =
        serde_json::from_str(&json).expect("failed to deserialize DumpSession");
    assert_eq!(parsed, entry);
}

#[test]
fn test_daemon_dump_empty_sessions() {
    let dump = DaemonDump {
        uptime_seconds: 0,
        socket_path: "/tmp/empty.sock".to_string(),
        sessions: vec![],
        session_counts: SessionCounts {
            active: 0,
            closed: 0,
        },
        connections: 0,
    };

    let json = serde_json::to_string(&dump).expect("failed to serialize DaemonDump");
    let parsed: DaemonDump =
        serde_json::from_str(&json).expect("failed to deserialize DaemonDump");
    assert_eq!(parsed.sessions.len(), 0);
    assert_eq!(parsed.session_counts.active, 0);
}

#[test]
fn test_daemon_dump_multiple_sessions() {
    let dump = DaemonDump {
        uptime_seconds: 7200,
        socket_path: "/tmp/multi.sock".to_string(),
        sessions: vec![
            DumpSession {
                session_id: "s1".to_string(),
                status: "working".to_string(),
                working_dir: Some("/project-a".to_string()),
                elapsed_seconds: 60,
                closed: false,
            },
            DumpSession {
                session_id: "s2".to_string(),
                status: "closed".to_string(),
                working_dir: Some("/project-b".to_string()),
                elapsed_seconds: 300,
                closed: true,
            },
            DumpSession {
                session_id: "s3".to_string(),
                status: "question".to_string(),
                working_dir: Some("/project-c".to_string()),
                elapsed_seconds: 10,
                closed: false,
            },
        ],
        session_counts: SessionCounts {
            active: 2,
            closed: 1,
        },
        connections: 3,
    };

    let json = serde_json::to_string(&dump).expect("failed to serialize DaemonDump");
    let parsed: DaemonDump =
        serde_json::from_str(&json).expect("failed to deserialize DaemonDump");
    assert_eq!(parsed.sessions.len(), 3);
    assert_eq!(parsed.session_counts.active, 2);
    assert_eq!(parsed.session_counts.closed, 1);
    assert_eq!(parsed.connections, 3);
}
