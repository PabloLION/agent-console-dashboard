//! Install/uninstall command implementations.
//!
//! Handles installation and removal of ACD hooks into ~/.claude/settings.json.

use std::process::ExitCode;

/// Returns the complete list of ACD hooks to install.
///
/// Each entry: (event, command, timeout, matcher).
/// This is the single source of truth for which hooks ACD registers.
pub(crate) fn acd_hook_definitions(
) -> Vec<(claude_hooks::HookEvent, &'static str, Option<String>)> {
    use claude_hooks::HookEvent;
    vec![
        (HookEvent::SessionStart, "acd claude-hook attention", None),
        (HookEvent::UserPromptSubmit, "acd claude-hook working", None),
        (HookEvent::Stop, "acd claude-hook attention", None),
        (HookEvent::SessionEnd, "acd claude-hook closed", None),
        (
            HookEvent::Notification,
            "acd claude-hook question",
            Some("elicitation_dialog".to_string()),
        ),
        (
            HookEvent::Notification,
            "acd claude-hook attention",
            Some("permission_prompt".to_string()),
        ),
        // PreToolUse bridges the gap when Claude resumes after permission_prompt
        // or elicitation_dialog. Without it, status stays "attention" while
        // Claude is actively working.
        (HookEvent::PreToolUse, "acd claude-hook working", None),
        // Experiment (acd-ws6): PostToolUse removed to test if PreToolUse alone
        // provides accurate status transitions. Restore when experiment concludes.
        // (HookEvent::PostToolUse, "acd claude-hook working", None),
    ]
}

/// Check if `acd` binary is reachable in PATH.
fn acd_in_path() -> bool {
    std::process::Command::new("which")
        .arg("acd")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Ensure ~/.claude/settings.json exists (create with `{}` if missing).
fn ensure_settings_file() -> std::result::Result<(), String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    let claude_dir = std::path::Path::new(&home).join(".claude");
    let settings_path = claude_dir.join("settings.json");

    if !settings_path.exists() {
        std::fs::create_dir_all(&claude_dir)
            .map_err(|e| format!("failed to create ~/.claude/: {}", e))?;
        std::fs::write(&settings_path, "{}\n")
            .map_err(|e| format!("failed to create settings.json: {}", e))?;
        println!("  Created ~/.claude/settings.json");
    }
    Ok(())
}

/// Install all ACD hooks into ~/.claude/settings.json.
pub(crate) fn run_install_command() -> ExitCode {
    // 1. Check PATH
    if !acd_in_path() {
        eprintln!("Warning: 'acd' not found in PATH");
        eprintln!("  Hooks will fail silently until acd is in PATH.");
        eprintln!("  Fix: cargo install --path crates/agent-console-dashboard");
        eprintln!();
    }

    // 2. Ensure settings.json exists
    if let Err(e) = ensure_settings_file() {
        eprintln!("Error: {}", e);
        return ExitCode::FAILURE;
    }

    // 3. Install each hook
    let definitions = acd_hook_definitions();
    let mut installed = 0u32;
    let mut skipped = 0u32;
    let mut errors = Vec::new();

    for (event, command, matcher) in &definitions {
        let handler = claude_hooks::HookHandler {
            r#type: "command".to_string(),
            command: command.to_string(),
            timeout: Some(10),
            r#async: None,
            status_message: None,
        };

        match claude_hooks::install(*event, handler, matcher.clone(), "acd") {
            Ok(()) => {
                installed += 1;
                let matcher_str = matcher
                    .as_ref()
                    .map(|m| format!(" ({})", m))
                    .unwrap_or_default();
                println!("  Installed: {:?}{} -> {}", event, matcher_str, command);
            }
            Err(claude_hooks::Error::Hook(claude_hooks::HookError::AlreadyExists { .. })) => {
                skipped += 1;
            }
            Err(e) => {
                errors.push(format!("{:?} -> {}: {}", event, command, e));
            }
        }
    }

    // 4. Summary
    println!();
    println!(
        "Hooks: {} installed, {} already present, {} errors",
        installed,
        skipped,
        errors.len()
    );

    if !errors.is_empty() {
        eprintln!();
        for err in &errors {
            eprintln!("  Error: {}", err);
        }
        return ExitCode::FAILURE;
    }

    if installed > 0 {
        println!();
        println!("You may need to restart Claude Code for hooks to take effect.");
    }

    ExitCode::SUCCESS
}

/// Remove all ACD-managed hooks from ~/.claude/settings.json.
pub(crate) fn run_uninstall_command() -> ExitCode {
    let definitions = acd_hook_definitions();
    let mut removed = 0u32;
    let mut skipped = 0u32;
    let mut errors = Vec::new();

    for (event, command, _matcher) in &definitions {
        match claude_hooks::uninstall(*event, command) {
            Ok(()) => {
                removed += 1;
                println!("  Removed: {:?} -> {}", event, command);
            }
            Err(claude_hooks::Error::Hook(claude_hooks::HookError::NotManaged { .. })) => {
                skipped += 1;
            }
            Err(e) => {
                errors.push(format!("{:?} -> {}: {}", event, command, e));
            }
        }
    }

    println!();
    println!(
        "Hooks: {} removed, {} not managed, {} errors",
        removed,
        skipped,
        errors.len()
    );

    if !errors.is_empty() {
        eprintln!();
        for err in &errors {
            eprintln!("  Error: {}", err);
        }
        return ExitCode::FAILURE;
    }

    if removed > 0 {
        println!();
        println!("You may need to restart Claude Code for changes to take effect.");
    }

    ExitCode::SUCCESS
}
