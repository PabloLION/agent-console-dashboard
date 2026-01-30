//! Logging initialization for the Agent Console daemon.
//!
//! Configures the `tracing` subscriber with level filtering via the `ACD_LOG`
//! environment variable. Falls back to `info` level when the variable is unset.
//!
//! # Usage
//!
//! ```bash
//! # Default (info level)
//! acd daemon
//!
//! # Debug level
//! ACD_LOG=debug acd daemon
//!
//! # Module-specific filtering
//! ACD_LOG=agent_console=debug,warn acd daemon
//! ```

use tracing_subscriber::{fmt, EnvFilter};

/// Initialize the tracing subscriber.
///
/// Reads the `ACD_LOG` environment variable for filter directives.
/// Falls back to `info` level when the variable is unset or invalid.
///
/// Output is written to stderr, which works for both foreground mode
/// (visible in terminal) and is the standard convention for log output.
///
/// # Panics
///
/// Panics if a global subscriber has already been set (should only be
/// called once, at daemon startup).
pub fn init() {
    let filter = EnvFilter::try_from_env("ACD_LOG")
        .unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();
}

#[cfg(test)]
mod tests {
    use tracing_subscriber::EnvFilter;

    #[test]
    fn env_filter_parses_valid_directives() {
        // Verify common filter strings parse without error
        let directives = ["info", "debug", "warn", "error", "trace"];
        for d in directives {
            let filter = EnvFilter::try_new(d);
            assert!(filter.is_ok(), "failed to parse directive: {}", d);
        }
    }

    #[test]
    fn env_filter_parses_module_directive() {
        let filter = EnvFilter::try_new("agent_console=debug,warn");
        assert!(filter.is_ok());
    }
}
