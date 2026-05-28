// src-tauri/src/errors.rs
// Unified error handling for Tauri commands

use std::fmt;

/// Command result type used across all Tauri command handlers.
pub type CommandResult<T> = Result<T, String>;

/// Helper to convert any error into a CommandResult string.
pub fn command_error<E: fmt::Display>(prefix: &str, error: E) -> String {
    format!("{prefix}: {error}")
}
