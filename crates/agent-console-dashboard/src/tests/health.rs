//! Tests for health status and diagnostics types.

use crate::*;

#[test]
fn test_format_uptime_minutes_only() {
    assert_eq!(format_uptime(0), "0m");
    assert_eq!(format_uptime(59), "0m");
    assert_eq!(format_uptime(60), "1m");
    assert_eq!(format_uptime(600), "10m");
    assert_eq!(format_uptime(3599), "59m");
}

#[test]
fn test_format_uptime_hours_and_minutes() {
    assert_eq!(format_uptime(3600), "1h 0m");
    assert_eq!(format_uptime(3660), "1h 1m");
    assert_eq!(format_uptime(9240), "2h 34m");
    assert_eq!(format_uptime(86400), "24h 0m");
}

#[test]
fn test_session_counts_equality() {
    let a = SessionCounts {
        active: 3,
        closed: 1,
    };
    let b = SessionCounts {
        active: 3,
        closed: 1,
    };
    let c = SessionCounts {
        active: 2,
        closed: 1,
    };
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn test_get_memory_usage_mb_returns_value() {
    // Best-effort test: on most systems this should return Some
    let mem = get_memory_usage_mb();
    // We just verify it doesn't panic; the value may be None in some CI environments
    if let Some(mb) = mem {
        assert!(mb > 0.0, "memory usage should be positive");
    }
}
