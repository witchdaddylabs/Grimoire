// src-tauri/src/errors.rs
// Unified error handling for Tauri commands

/// Command result type used across all Tauri command handlers.
pub type CommandResult<T> = Result<T, String>;
