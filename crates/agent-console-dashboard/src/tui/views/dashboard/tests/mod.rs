pub(crate) use super::*;
pub(crate) use crate::{AgentType, Session};
pub(crate) use std::path::PathBuf;

mod buffer;
mod disambiguation;
mod rendering;
mod stories;
mod unit;

pub(crate) fn make_session(id: &str, status: Status) -> Session {
    let mut s = Session::new(
        id.to_string(),
        AgentType::ClaudeCode,
        Some(PathBuf::from("/home/user/project")),
    );
    s.status = status;
    s
}

pub(crate) fn make_test_session(id: &str, working_dir: Option<PathBuf>) -> Session {
    Session::new(id.to_string(), AgentType::ClaudeCode, working_dir)
}
