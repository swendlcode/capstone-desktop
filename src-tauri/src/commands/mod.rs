//! Thin Tauri handlers, one file per domain.
//!
//! A command validates its input, hands the work to `core` code -- `jobs`,
//! `audio`, `templates`, `settings` -- and returns. No DSP and no business
//! logic lives in this layer, and anything that blocks goes through
//! `tokio::task::spawn_blocking` so it cannot stall the IPC bridge.

pub mod cover_commands;
pub mod dialog_commands;
pub mod process_commands;
pub mod queue_commands;
pub mod settings_commands;
pub mod template_commands;
