//! Tests for the SessionStore module.
//!
//! Tests are organized into categories:
//! - `basic`: Core CRUD operations
//! - `lifecycle_*`: Session lifecycle methods
//!   - `lifecycle_create`: create_session tests
//!   - `lifecycle_get_or_create`: get_or_create_session tests
//!   - `lifecycle_update`: update_session tests
//!   - `lifecycle_close`: close_session and remove_session tests
//! - `concurrent`: Concurrent access and thread-safety
//! - `subscriber`: Broadcast channel and notifications

mod basic;
mod closed;
mod concurrent;
mod lifecycle_close;
mod lifecycle_create;
mod lifecycle_get_or_create;
mod lifecycle_update;
mod inactive;
mod subscriber;

use super::SessionStore;
use crate::{AgentType, Session};
use std::path::PathBuf;

/// Helper function to create a test session with the given ID.
pub(super) fn create_test_session(id: &str) -> Session {
    Session::new(
        id.to_string(),
        AgentType::ClaudeCode,
        PathBuf::from(format!("/home/user/{}", id)),
    )
}
