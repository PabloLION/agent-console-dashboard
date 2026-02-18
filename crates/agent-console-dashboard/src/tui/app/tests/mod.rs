pub(crate) use super::*;
pub(crate) use crate::AgentType;

mod basic;
mod interaction;

pub(crate) fn make_app_with_sessions(count: usize) -> App {
    let mut app = App::new(PathBuf::from("/tmp/test.sock"), None);
    for i in 0..count {
        app.sessions.push(Session::new(
            format!("session-{}", i),
            AgentType::ClaudeCode,
            Some(PathBuf::from(format!("/home/user/project-{}", i))),
        ));
    }
    app.init_selection();
    app
}
