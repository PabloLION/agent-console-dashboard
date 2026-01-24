//! Internal client module for the Agent Console Dashboard.
//!
//! This module provides client connection functionality with auto-start
//! capability for the daemon. When a client attempts to connect and finds
//! the daemon not running, it will automatically spawn the daemon process
//! in the background and retry the connection with exponential backoff.
//!
//! # Internal Use Only
//!
//! This module is intentionally **not exported** from the library root.
//! It is used internally by:
//! - The TUI dashboard (`agent-console tui`)
//! - CLI commands (`agent-console set/list/subscribe`)
//!
//! External tools (shell hooks, scripts) should use the CLI commands
//! rather than importing this module directly.
//!
//! # Features
//!
//! - **Auto-Start**: Automatically spawns the daemon if not running
//! - **Exponential Backoff**: Retries connection with increasing delays
//! - **Race-Safe**: Multiple simultaneous clients won't spawn duplicate daemons
//!
//! # Usage (internal)
//!
//! ```ignore
//! use crate::client::connect_with_auto_start;
//! use std::path::Path;
//!
//! async fn example() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     let client = connect_with_auto_start(Path::new("/tmp/agent-console.sock")).await?;
//!     // Use client for communication with daemon
//!     Ok(())
//! }
//! ```

pub mod connection;

pub use connection::{connect_with_auto_start, Client, ClientError};

use std::error::Error;

/// Result type alias for client operations.
///
/// Uses `Send + Sync` bounds on the error type to allow the result
/// to be safely passed across thread boundaries, which is essential
/// for async operations in multi-threaded runtimes.
pub type ClientResult<T> = Result<T, Box<dyn Error + Send + Sync>>;
