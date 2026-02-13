//! Health status and diagnostics types for the daemon.

/// Session count breakdown for health status reporting.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct SessionCounts {
    /// Count of active (non-closed) sessions.
    pub active: usize,
    /// Count of closed sessions.
    pub closed: usize,
}

/// Health status response from the daemon STATUS command.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct HealthStatus {
    /// Daemon uptime in seconds.
    pub uptime_seconds: u64,
    /// Session count breakdown.
    pub sessions: SessionCounts,
    /// Count of active connections to the daemon.
    pub connections: usize,
    /// Process memory usage in MB (None if unavailable).
    pub memory_mb: Option<f64>,
    /// Path to the Unix domain socket.
    pub socket_path: String,
}

/// Full daemon state dump for diagnostics.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct DaemonDump {
    /// Daemon uptime in seconds.
    pub uptime_seconds: u64,
    /// Path to the Unix domain socket.
    pub socket_path: String,
    /// Snapshot of all sessions.
    pub sessions: Vec<DumpSession>,
    /// Session count breakdown.
    pub session_counts: SessionCounts,
    /// Count of active connections to the daemon.
    pub connections: usize,
}

/// Summary of a single session for dump output.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct DumpSession {
    /// Unique session identifier.
    pub session_id: String,
    /// Current session status as string.
    pub status: String,
    /// Working directory for this session.
    pub working_dir: Option<String>,
    /// Elapsed seconds in the current status.
    pub elapsed_seconds: u64,
    /// Whether session has been closed.
    pub closed: bool,
}

/// Formats a duration in seconds to a human-readable string.
///
/// Returns "Xh Ym" for durations >= 1 hour, "Xm" otherwise.
pub fn format_uptime(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

/// Queries the current process memory usage via sysinfo.
///
/// Returns the RSS in megabytes, or None if the process cannot be found.
pub fn get_memory_usage_mb() -> Option<f64> {
    use sysinfo::{Pid, System};

    let pid = Pid::from_u32(std::process::id());
    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
    sys.process(pid)
        .map(|proc_info| proc_info.memory() as f64 / 1024.0 / 1024.0)
}
