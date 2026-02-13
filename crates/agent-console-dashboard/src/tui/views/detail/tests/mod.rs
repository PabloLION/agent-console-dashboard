pub(crate) use super::*;
pub(crate) use crate::{AgentType, StateTransition};
pub(crate) use std::path::PathBuf;
pub(crate) use std::time::Duration;

mod rendering;
mod unit;

pub(crate) fn make_session(id: &str) -> Session {
    Session::new(
        id.to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/home/user/project-a")),
    )
}
