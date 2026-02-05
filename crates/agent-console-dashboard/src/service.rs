//! Platform-specific system service management for the Agent Console daemon.
//!
//! Provides install, uninstall, and status commands for:
//! - macOS via launchd (LaunchAgents plist)
//! - Linux via systemd (user unit files)

use std::path::PathBuf;
use std::process::Command;

/// macOS launchd plist template with USERNAME placeholder.
const PLIST_TEMPLATE: &str = include_str!("../../../resources/com.agent-console.daemon.plist");

/// Linux systemd user unit file.
const SYSTEMD_UNIT: &str = include_str!("../../../resources/acd.service");

/// Service label used in launchd plist.
const LAUNCHD_LABEL: &str = "com.agent-console.daemon";

/// Systemd service name.
const SYSTEMD_SERVICE: &str = "acd.service";

/// Install the daemon as a system service.
pub fn install_service() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    {
        install_macos()
    }
    #[cfg(target_os = "linux")]
    {
        install_linux()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Err("Platform not supported. Only macOS (launchd) and Linux (systemd) are supported.".into())
    }
}

/// Uninstall the daemon system service.
pub fn uninstall_service() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    {
        uninstall_macos()
    }
    #[cfg(target_os = "linux")]
    {
        uninstall_linux()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Err("Platform not supported. Only macOS (launchd) and Linux (systemd) are supported.".into())
    }
}

/// Check daemon service status.
pub fn service_status() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    {
        status_macos()
    }
    #[cfg(target_os = "linux")]
    {
        status_linux()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Err("Platform not supported. Only macOS (launchd) and Linux (systemd) are supported.".into())
    }
}

/// Returns the macOS plist install path: ~/Library/LaunchAgents/com.agent-console.daemon.plist
pub fn macos_plist_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let home = dirs::home_dir().ok_or("could not determine home directory")?;
    Ok(home
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{LAUNCHD_LABEL}.plist")))
}

/// Returns the Linux systemd unit install path: ~/.config/systemd/user/acd.service
pub fn linux_unit_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let home = dirs::home_dir().ok_or("could not determine home directory")?;
    Ok(home
        .join(".config")
        .join("systemd")
        .join("user")
        .join(SYSTEMD_SERVICE))
}

/// Returns the plist content with USERNAME replaced by the current user.
pub fn render_plist() -> Result<String, Box<dyn std::error::Error>> {
    let username = get_username()?;
    Ok(PLIST_TEMPLATE.replace("USERNAME", &username))
}

/// Returns the systemd unit content.
pub fn rendered_unit() -> &'static str {
    SYSTEMD_UNIT
}

/// Returns the raw plist template content (before placeholder replacement).
pub fn plist_template() -> &'static str {
    PLIST_TEMPLATE
}

// --- macOS implementation ---

#[cfg(target_os = "macos")]
fn install_macos() -> Result<(), Box<dyn std::error::Error>> {
    let plist_path = macos_plist_path()?;
    let parent = plist_path
        .parent()
        .ok_or("could not determine plist parent directory")?;

    std::fs::create_dir_all(parent)?;

    let content = render_plist()?;
    std::fs::write(&plist_path, &content)?;

    println!("Wrote plist to {}", plist_path.display());

    let output = Command::new("launchctl")
        .args(["load", &plist_path.to_string_lossy()])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("launchctl load failed: {stderr}").into());
    }

    println!("Service installed and loaded via launchd");
    Ok(())
}

#[cfg(target_os = "macos")]
fn uninstall_macos() -> Result<(), Box<dyn std::error::Error>> {
    let uid = get_uid()?;

    let output = Command::new("launchctl")
        .args(["bootout", &format!("gui/{uid}/{LAUNCHD_LABEL}")])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Warning: launchctl bootout: {stderr}");
    }

    let plist_path = macos_plist_path()?;
    if plist_path.exists() {
        std::fs::remove_file(&plist_path)?;
        println!("Removed {}", plist_path.display());
    }

    println!("Service uninstalled");
    Ok(())
}

#[cfg(target_os = "macos")]
fn status_macos() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("launchctl")
        .args(["list"])
        .output()?;

    if !output.status.success() {
        return Err("launchctl list failed".into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let found: Vec<&str> = stdout
        .lines()
        .filter(|line| line.contains("com.agent-console"))
        .collect();

    if found.is_empty() {
        println!("Service is not loaded");
    } else {
        for line in &found {
            println!("{line}");
        }
    }
    Ok(())
}

// --- Linux implementation ---

#[cfg(target_os = "linux")]
fn install_linux() -> Result<(), Box<dyn std::error::Error>> {
    let unit_path = linux_unit_path()?;
    let parent = unit_path
        .parent()
        .ok_or("could not determine unit parent directory")?;

    std::fs::create_dir_all(parent)?;
    std::fs::write(&unit_path, SYSTEMD_UNIT)?;

    println!("Wrote unit file to {}", unit_path.display());

    let output = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("systemctl daemon-reload failed: {stderr}").into());
    }

    let output = Command::new("systemctl")
        .args(["--user", "enable", SYSTEMD_SERVICE])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("systemctl enable failed: {stderr}").into());
    }

    println!("Service installed and enabled via systemd");
    Ok(())
}

#[cfg(target_os = "linux")]
fn uninstall_linux() -> Result<(), Box<dyn std::error::Error>> {
    let _ = Command::new("systemctl")
        .args(["--user", "stop", "acd"])
        .output();

    let output = Command::new("systemctl")
        .args(["--user", "disable", "acd"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Warning: systemctl disable: {stderr}");
    }

    let unit_path = linux_unit_path()?;
    if unit_path.exists() {
        std::fs::remove_file(&unit_path)?;
        println!("Removed {}", unit_path.display());
    }

    let _ = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .output();

    println!("Service uninstalled");
    Ok(())
}

#[cfg(target_os = "linux")]
fn status_linux() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("systemctl")
        .args(["--user", "is-enabled", SYSTEMD_SERVICE])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    println!("Service status: {stdout}");
    Ok(())
}

// --- Helpers ---

fn get_username() -> Result<String, Box<dyn std::error::Error>> {
    std::env::var("USER")
        .or_else(|_| std::env::var("LOGNAME"))
        .map_err(|_| "could not determine username from USER or LOGNAME".into())
}

#[cfg(target_os = "macos")]
fn get_uid() -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("id").args(["-u"]).output()?;
    if !output.status.success() {
        return Err("failed to get user id".into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plist_template_contains_label() {
        assert!(
            PLIST_TEMPLATE.contains(LAUNCHD_LABEL),
            "plist template should contain the launchd label"
        );
    }

    #[test]
    fn test_plist_template_contains_username_placeholder() {
        assert!(
            PLIST_TEMPLATE.contains("USERNAME"),
            "plist template should contain USERNAME placeholder"
        );
    }

    #[test]
    fn test_systemd_unit_contains_exec_start() {
        assert!(
            SYSTEMD_UNIT.contains("ExecStart="),
            "systemd unit should contain ExecStart directive"
        );
    }

    #[test]
    fn test_systemd_unit_contains_acd_daemon() {
        assert!(
            SYSTEMD_UNIT.contains("acd daemon"),
            "systemd unit ExecStart should invoke 'acd daemon'"
        );
    }

    #[test]
    fn test_render_plist_replaces_username() {
        let rendered = render_plist().expect("render_plist should succeed");
        assert!(
            !rendered.contains("USERNAME"),
            "rendered plist should not contain USERNAME placeholder"
        );
        // Should contain the actual username
        let username = std::env::var("USER")
            .or_else(|_| std::env::var("LOGNAME"))
            .expect("USER or LOGNAME should be set");
        assert!(
            rendered.contains(&username),
            "rendered plist should contain the actual username"
        );
    }

    #[test]
    fn test_rendered_unit_is_static() {
        assert_eq!(
            rendered_unit(),
            SYSTEMD_UNIT,
            "rendered_unit should return the raw systemd unit"
        );
    }

    #[test]
    fn test_macos_plist_path_under_launch_agents() {
        let path = macos_plist_path().expect("macos_plist_path should succeed");
        let path_str = path.to_string_lossy();
        assert!(
            path_str.contains("Library/LaunchAgents"),
            "plist path should be under ~/Library/LaunchAgents, got: {path_str}"
        );
        assert!(
            path_str.ends_with(".plist"),
            "plist path should end with .plist"
        );
    }

    #[test]
    fn test_linux_unit_path_under_systemd_user() {
        let path = linux_unit_path().expect("linux_unit_path should succeed");
        let path_str = path.to_string_lossy();
        assert!(
            path_str.contains(".config/systemd/user"),
            "unit path should be under ~/.config/systemd/user, got: {path_str}"
        );
        assert!(
            path_str.ends_with(".service"),
            "unit path should end with .service"
        );
    }

    #[test]
    fn test_get_username_succeeds() {
        let username = get_username().expect("get_username should succeed");
        assert!(!username.is_empty(), "username should not be empty");
    }

    #[test]
    fn test_plist_template_is_valid_xml() {
        assert!(
            PLIST_TEMPLATE.contains("<?xml"),
            "plist should start with XML declaration"
        );
        assert!(
            PLIST_TEMPLATE.contains("</plist>"),
            "plist should have closing plist tag"
        );
    }

    #[test]
    fn test_systemd_unit_has_install_section() {
        assert!(
            SYSTEMD_UNIT.contains("[Install]"),
            "systemd unit should have [Install] section"
        );
    }

    #[test]
    fn test_systemd_unit_has_service_section() {
        assert!(
            SYSTEMD_UNIT.contains("[Service]"),
            "systemd unit should have [Service] section"
        );
    }

    #[test]
    fn test_constants_are_consistent() {
        assert_eq!(LAUNCHD_LABEL, "com.agent-console.daemon");
        assert_eq!(SYSTEMD_SERVICE, "acd.service");
    }
}
