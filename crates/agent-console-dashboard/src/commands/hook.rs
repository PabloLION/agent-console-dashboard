//! Claude Code hook command implementations.
//!
//! Handles the `claude-hook` command that receives JSON from stdin and communicates
//! with the daemon to update session status.

use agent_console_dashboard::{
    client::connect_with_lazy_start, IpcCommand, IpcCommandKind, Status, IPC_VERSION,
};
use std::process::ExitCode;

/// JSON payload from Claude Code hook stdin.
///
/// Only fields we need are declared; unknown fields are silently ignored
/// so future Claude Code versions don't break us.
#[derive(serde::Deserialize)]
pub(crate) struct HookInput {
    pub session_id: String,
    pub cwd: String,
}

/// Validates HookInput fields. Returns warnings for invalid fields.
/// Does not reject input — Claude Code should not be blocked by validation.
pub(crate) fn validate_hook_input(input: &HookInput) -> Vec<String> {
    let mut warnings = Vec::new();

    // session_id: 36 chars, hex + dashes only
    // TODO(acd-rhr): Consider full UUID v4 validation
    if input.session_id.len() != 36 {
        warnings.push(format!(
            "session_id length is {} (expected 36): {}",
            input.session_id.len(),
            input.session_id
        ));
    } else if !input
        .session_id
        .chars()
        .all(|c| c.is_ascii_hexdigit() || c == '-')
    {
        warnings.push(format!(
            "session_id contains invalid characters: {}",
            input.session_id
        ));
    }

    // cwd: non-empty absolute path
    // TODO(acd-8vx): Consider validating path exists
    if input.cwd.is_empty() {
        warnings.push("cwd is empty".to_string());
    } else if !input.cwd.starts_with('/') {
        warnings.push(format!("cwd is not an absolute path: {}", input.cwd));
    }

    warnings
}

/// Connects to daemon via lazy-start (spawning if needed), sends SET command as JSON.
///
/// Exit codes per Claude Code hook spec:
/// - 0: success (outputs `{"continue": true}` on stdout)
///
/// This function never returns a non-zero exit code after stdin parsing
/// succeeds -- hook failures are reported via systemMessage to avoid blocking
/// Claude Code.
pub(crate) async fn run_claude_hook_async(
    socket: &std::path::Path,
    status: Status,
    input: &HookInput,
) -> ExitCode {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let warnings = validate_hook_input(input);
    for w in &warnings {
        eprintln!("acd claude-hook: warning: {}", w);
    }

    let client = match connect_with_lazy_start(socket).await {
        Ok(c) => c,
        Err(e) => {
            let json = serde_json::json!({
                "continue": true,
                "systemMessage": format!(
                    "acd daemon not reachable ({}), session {} not tracked",
                    e, input.session_id
                ),
            });
            println!("{}", json);
            return ExitCode::SUCCESS;
        }
    };

    let stream = client.into_stream();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let cmd = IpcCommand {
        version: IPC_VERSION,
        cmd: IpcCommandKind::Set.to_string(),
        session_id: Some(input.session_id.clone()),
        status: Some(status.to_string()),
        working_dir: Some(input.cwd.clone()),
        confirmed: None,
        priority: None,
    };
    let cmd_json = serde_json::to_string(&cmd).expect("failed to serialize SET command");
    let cmd_line = format!("{}\n", cmd_json);

    if writer.write_all(cmd_line.as_bytes()).await.is_err() || writer.flush().await.is_err() {
        let json = serde_json::json!({
            "continue": true,
            "systemMessage": format!(
                "acd daemon: failed to send command, session {} not tracked",
                input.session_id
            ),
        });
        println!("{}", json);
        return ExitCode::SUCCESS;
    }

    let mut line = String::new();
    if reader.read_line(&mut line).await.is_err() {
        let json = serde_json::json!({
            "continue": true,
            "systemMessage": format!(
                "acd daemon: no response, session {} not tracked",
                input.session_id
            ),
        });
        println!("{}", json);
        return ExitCode::SUCCESS;
    }

    match serde_json::from_str::<agent_console_dashboard::IpcResponse>(line.trim()) {
        Ok(resp) if resp.ok => {
            println!(r#"{{"continue": true}}"#);
        }
        Ok(resp) => {
            let err = resp.error.unwrap_or_else(|| "unknown error".to_string());
            let json = serde_json::json!({
                "continue": true,
                "systemMessage": format!(
                    "acd daemon error: {}, session {} not tracked",
                    err, input.session_id
                ),
            });
            println!("{}", json);
        }
        Err(_) => {
            let json = serde_json::json!({
                "continue": true,
                "systemMessage": format!(
                    "acd daemon: invalid response, session {} not tracked",
                    input.session_id
                ),
            });
            println!("{}", json);
        }
    }
    ExitCode::SUCCESS
}
