use super::*;

impl App {
    /// Applies a daemon update message (full `SessionSnapshot`) to the session list.
    ///
    /// `elapsed_seconds` is the time since the session entered its current
    /// status, as reported by the daemon. We backdate `session.since` by
    /// subtracting this duration from `Instant::now()` so elapsed time
    /// displays correctly even though `Instant` cannot survive IPC.
    pub(super) fn apply_update(&mut self, info: &crate::SessionSnapshot) {
        let status: Status = info.status.parse().unwrap_or(Status::Working);
        let backdated_since = Instant::now()
            .checked_sub(Duration::from_secs(info.elapsed_seconds))
            .unwrap_or_else(Instant::now);
        let backdated_activity = Instant::now()
            .checked_sub(Duration::from_secs(info.idle_seconds))
            .unwrap_or_else(Instant::now);
        let working_dir = info.working_dir.as_ref().map(PathBuf::from);

        if let Some(session) = self
            .sessions
            .iter_mut()
            .find(|s| s.session_id == info.session_id)
        {
            // Update working_dir from daemon if Some
            if working_dir.is_some() {
                session.working_dir = working_dir.clone();
            }
            if session.status != status {
                session.history.push(crate::StateTransition {
                    timestamp: Instant::now(),
                    from: session.status,
                    to: status,
                    duration: session.since.elapsed(),
                });
                session.status = status;
                session.since = backdated_since;
            }
            session.last_activity = backdated_activity;
            session.closed = info.closed;
            session.priority = info.priority;
        } else {
            let mut session = Session::new(
                info.session_id.clone(),
                AgentType::ClaudeCode,
                working_dir.clone(),
            );
            session.status = status;
            session.since = backdated_since;
            session.last_activity = backdated_activity;
            session.closed = info.closed;
            session.priority = info.priority;
            // Reconstruct history from wire StatusChange entries
            let now_secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let entries = &info.history;
            for i in 0..entries.len() {
                let to = entries[i]
                    .status
                    .parse::<Status>()
                    .unwrap_or(Status::Working);
                let from = if i > 0 {
                    entries[i - 1]
                        .status
                        .parse::<Status>()
                        .unwrap_or(Status::Working)
                } else {
                    Status::Working
                };
                let duration = if i > 0 {
                    Duration::from_secs(entries[i].at_secs.saturating_sub(entries[i - 1].at_secs))
                } else {
                    Duration::from_secs(0)
                };
                // Approximate Instant from unix timestamp
                let secs_ago = now_secs.saturating_sub(entries[i].at_secs);
                let timestamp = Instant::now()
                    .checked_sub(Duration::from_secs(secs_ago))
                    .unwrap_or_else(Instant::now);
                session.history.push(crate::StateTransition {
                    timestamp,
                    from,
                    to,
                    duration,
                });
            }
            self.sessions.push(session);
            if self.selected_index.is_none() {
                self.selected_index = Some(0);
            }
        }

        // Sort sessions: status group → priority (desc) → elapsed (desc)
        self.sessions.sort_by(|a, b| {
            use std::cmp::Reverse;

            // Determine status group for sorting
            let a_group = if a.closed {
                3u8 // Closed sessions: group 3
            } else if a.is_inactive(crate::INACTIVE_SESSION_THRESHOLD) {
                2u8 // Inactive sessions: group 2
            } else {
                a.status.status_group()
            };

            let b_group = if b.closed {
                3u8
            } else if b.is_inactive(crate::INACTIVE_SESSION_THRESHOLD) {
                2u8
            } else {
                b.status.status_group()
            };

            let a_elapsed = a.since.elapsed().as_secs();
            let b_elapsed = b.since.elapsed().as_secs();

            // Sort by: group (asc) → priority (desc) → elapsed (desc)
            (a_group, Reverse(a.priority), Reverse(a_elapsed)).cmp(&(
                b_group,
                Reverse(b.priority),
                Reverse(b_elapsed),
            ))
        });
    }
}
