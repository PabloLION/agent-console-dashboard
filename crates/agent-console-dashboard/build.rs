//! Build script for agent-console-dashboard.
//!
//! Generates `.claude-plugin/plugin.json` and `.claude-plugin/marketplace.json`
//! at the workspace root. These files declare ACD as a Claude Code plugin with
//! hooks that replace the old shell-script + settings.json approach.
//!
//! Also enforces that the Cargo.toml version matches the generated plugin.json
//! version — a mismatch fails the build.
//!
//! Installs git hooks (pre-commit, pre-push) by symlinking from `.git/hooks/`
//! to `scripts/`. Set `NO_INSTALL_HOOKS=1` to skip.

use std::fs;
use std::path::Path;

fn main() {
    let cargo_version = env!("CARGO_PKG_VERSION");

    // Workspace root is two levels up from the crate directory
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let workspace_root = Path::new(&manifest_dir)
        .parent()
        .expect("crate should have parent")
        .parent()
        .expect("crates/ should have parent");

    let plugin_dir = workspace_root.join(".claude-plugin");
    fs::create_dir_all(&plugin_dir).expect("failed to create .claude-plugin directory");

    // Generate plugin.json
    let plugin_json = serde_json::json!({
        "name": "agent-console-dashboard",
        "version": cargo_version,
        "description": "Real-time TUI dashboard for monitoring Claude Code sessions",
        "author": {
            "name": "Pablo LION"
        },
        "hooks": {
            "Stop": [
                {
                    "matcher": "",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "acd claude-hook attention",
                            "timeout": 10
                        }
                    ]
                }
            ],
            "SessionStart": [
                {
                    "matcher": "",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "acd claude-hook attention",
                            "timeout": 10
                        }
                    ]
                }
            ],
            "UserPromptSubmit": [
                {
                    "matcher": "",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "acd claude-hook working",
                            "timeout": 10
                        }
                    ]
                }
            ],
            "SessionEnd": [
                {
                    "matcher": "",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "acd claude-hook closed",
                            "timeout": 10
                        }
                    ]
                }
            ],
            "Notification": [
                {
                    "matcher": "elicitation_dialog",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "acd claude-hook question",
                            "timeout": 10
                        }
                    ]
                },
                {
                    "matcher": "permission_prompt",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "acd claude-hook attention",
                            "timeout": 10
                        }
                    ]
                }
            ],
            "PreToolUse": [
                {
                    "matcher": "",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "acd claude-hook working",
                            "timeout": 10
                        }
                    ]
                }
            ],
            "PreCompact": [
                {
                    "matcher": "",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "acd claude-hook working",
                            "timeout": 10
                        }
                    ]
                }
            ]
            // Experiment (acd-ws6): PostToolUse commented out to test if
            // PreToolUse alone provides accurate status transitions.
            // Uncomment when experiment concludes.
            /* "PostToolUse": [
                {
                    "matcher": "",
                    "hooks": [
                        {
                            "type": "command",
                            "command": "acd claude-hook working",
                            "timeout": 10
                        }
                    ]
                }
            ] */
        }
    });

    let plugin_path = plugin_dir.join("plugin.json");
    let plugin_content =
        serde_json::to_string_pretty(&plugin_json).expect("failed to serialize plugin.json");
    fs::write(&plugin_path, &plugin_content).expect("failed to write plugin.json");

    // Generate marketplace.json
    let marketplace_json = serde_json::json!({
        "plugins": [
            {
                "name": "agent-console-dashboard",
                "path": "./"
            }
        ]
    });

    let marketplace_path = plugin_dir.join("marketplace.json");
    let marketplace_content = serde_json::to_string_pretty(&marketplace_json)
        .expect("failed to serialize marketplace.json");
    fs::write(&marketplace_path, &marketplace_content).expect("failed to write marketplace.json");

    // Version sync check: read back generated plugin.json and verify version matches
    let generated: serde_json::Value =
        serde_json::from_str(&plugin_content).expect("failed to parse generated plugin.json");
    let plugin_version = generated["version"]
        .as_str()
        .expect("plugin.json should have a version string");

    assert_eq!(
        plugin_version, cargo_version,
        "plugin.json version ({}) does not match Cargo.toml version ({})",
        plugin_version, cargo_version
    );

    // Re-run if Cargo.toml changes (version bump)
    println!("cargo:rerun-if-changed=Cargo.toml");

    // Install git hooks
    install_git_hooks(workspace_root);
}

/// Install git hooks by creating symlinks from `.git/hooks/` to `scripts/`.
///
/// Skipped when `NO_INSTALL_HOOKS=1` is set (for CI or special environments).
/// Warns but does not overwrite hooks that exist and aren't our symlinks.
fn install_git_hooks(workspace_root: &Path) {
    if std::env::var("NO_INSTALL_HOOKS").as_deref() == Ok("1") {
        return;
    }

    let git_hooks_dir = workspace_root.join(".git/hooks");
    if !git_hooks_dir.is_dir() {
        // Not a git repo (e.g., extracted tarball), skip silently
        return;
    }

    let hooks = &[
        ("pre-commit", "../../scripts/pre-commit.sh"),
        ("pre-push", "../../scripts/pre-push.sh"),
    ];

    for (hook_name, relative_target) in hooks {
        let hook_path = git_hooks_dir.join(hook_name);
        let target = Path::new(relative_target);

        // Check if the script source exists
        let script_path = workspace_root.join(format!("scripts/{hook_name}.sh"));
        if !script_path.exists() {
            println!("cargo:warning=git hook script not found: scripts/{hook_name}.sh, skipping");
            continue;
        }

        // Re-run if hook scripts change
        println!("cargo:rerun-if-changed=scripts/{hook_name}.sh");

        // If hook already exists, check what it is
        if hook_path.exists() || hook_path.symlink_metadata().is_ok() {
            match hook_path.read_link() {
                Ok(existing_target) if existing_target == target => {
                    // Already points to the right place
                    continue;
                }
                Ok(_) => {
                    // Symlink to something else — replace it
                    let _ = fs::remove_file(&hook_path);
                }
                Err(_) => {
                    // Not a symlink (regular file) — don't overwrite
                    println!(
                        "cargo:warning=.git/hooks/{hook_name} exists and is not our symlink, \
                         skipping. Remove it manually to enable auto-installed hooks."
                    );
                    continue;
                }
            }
        }

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, &hook_path).unwrap_or_else(|e| {
                println!("cargo:warning=failed to create {hook_name} hook symlink: {e}");
            });
        }

        #[cfg(not(unix))]
        {
            println!(
                "cargo:warning=git hook auto-install not supported on this platform, \
                 run: ln -sf {relative_target} .git/hooks/{hook_name}"
            );
        }
    }
}
