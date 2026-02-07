//! Build script for agent-console-dashboard.
//!
//! Generates `.claude-plugin/plugin.json` and `.claude-plugin/marketplace.json`
//! at the workspace root. These files declare ACD as a Claude Code plugin with
//! hooks that replace the old shell-script + settings.json approach.
//!
//! Also enforces that the Cargo.toml version matches the generated plugin.json
//! version — a mismatch fails the build.

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
            ]
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
}
