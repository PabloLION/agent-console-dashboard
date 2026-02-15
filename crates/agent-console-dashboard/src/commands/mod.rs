//! Command implementations for the ACD CLI.
//!
//! This module contains all command handler functions, organized by domain:
//! - `daemon` - Daemon lifecycle commands (start, stop)
//! - `hook` - Claude Code hook integration
//! - `install` - Hook installation/uninstallation
//! - `ipc` - IPC commands (update, status, dump)

pub(crate) mod daemon;
pub(crate) mod hook;
pub(crate) mod install;
pub(crate) mod ipc;

pub(crate) use daemon::*;
pub(crate) use hook::*;
pub(crate) use install::*;
pub(crate) use ipc::*;
