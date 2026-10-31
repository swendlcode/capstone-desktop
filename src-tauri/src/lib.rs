//! Capstone: batch loudness, clip repair, warmth and pitch correction for
//! finished audio files.
//!
//! This file is the module list and the builder chain, and deliberately nothing
//! else. Every command the frontend can call is registered in one flat
//! `generate_handler!` below -- a missing line there is the usual cause of a
//! "command not found" at runtime, and a flat list is what keeps that
//! checkable at a glance.

pub mod audio;
pub mod commands;
pub mod error;
pub mod jobs;
pub mod models;
pub mod settings;
pub mod state;
pub mod templates;

use tauri::Manager;

use crate::state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "capstone_lib=info".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let state = AppState::init(app.handle().clone())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::queue_commands::add_paths,
            commands::queue_commands::analyze_file,
            commands::queue_commands::get_queue,
            commands::queue_commands::remove_from_queue,
            commands::queue_commands::clear_queue,
            commands::process_commands::start_processing,
            commands::process_commands::cancel_processing,
            commands::template_commands::get_templates,
            commands::template_commands::save_template,
            commands::template_commands::duplicate_template,
            commands::template_commands::delete_template,
            commands::settings_commands::get_settings,
            commands::settings_commands::update_settings,
            commands::dialog_commands::open_output_folder,
            commands::dialog_commands::pick_files,
            commands::dialog_commands::pick_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
