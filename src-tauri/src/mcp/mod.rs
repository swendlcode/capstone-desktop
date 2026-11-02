//! Capstone's agent-facing surface.
//!
//! The same two routes Keystone uses, and for the same reason: a change made
//! while the window is open has to appear in it.
//!
//! - **Through the app**, over the loopback bridge, so a saved template and a
//!   queued file land in the open window.
//! - **Straight at the files on disk** when nothing answers the probe.
//!
//! The fallback is narrower here than in Keystone, and that is a property of
//! the app rather than an omission. Capstone keeps **templates and settings
//! on disk** but the **queue only in memory** — there is no queue file to
//! read, and the processing pool and its progress events live in the app. So
//! with Capstone closed an agent can read and write templates and settings,
//! and nothing else. Those tools say so through [`McpContext::require_live`]
//! rather than returning an empty queue, which would read as "you have no
//! files" instead of "the app is not running".

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::RwLock;
use serde_json::Value;

use crate::error::{CapstoneError, Result};
use crate::models::{Settings, Template};

mod catalog;
mod stdio;
mod tools;

pub use catalog::{tool_list, TOOL_NAMES};
pub use stdio::serve_stdio;

const IDENTIFIER: &str = "app.capstone.desktop";

/// What only exists while the app is running.
pub struct Live {
    pub app: tauri::AppHandle,
}

pub struct McpContext {
    /// The app's own `Arc` when live, so a write is the write the window sees.
    pub templates: Arc<RwLock<Vec<Template>>>,
    pub settings: Arc<RwLock<Settings>>,
    pub data_dir: PathBuf,
    pub live: Option<Live>,
}

impl McpContext {
    /// Inside the running app. The handle comes in separately because
    /// `AppState` does not hold one — the queue runner is handed it per call.
    pub fn from_state(state: &crate::state::AppState, app: tauri::AppHandle) -> Self {
        Self {
            templates: state.templates.clone(),
            settings: state.settings.clone(),
            data_dir: state.data_dir.clone(),
            live: Some(Live { app }),
        }
    }

    /// Standalone: the templates and settings the app would have loaded.
    pub fn standalone() -> Result<Self> {
        let data_dir = data_dir()?;
        if !data_dir.exists() {
            return Err(CapstoneError::NotFound(format!(
                "no Capstone data at {}; open Capstone once to create it",
                data_dir.display()
            )));
        }
        Ok(Self {
            templates: Arc::new(RwLock::new(crate::templates::load(&data_dir))),
            settings: Arc::new(RwLock::new(crate::settings::load(&data_dir))),
            data_dir,
            live: None,
        })
    }

    pub fn require_live(&self, what: &str) -> Result<&Live> {
        self.live.as_ref().ok_or_else(|| {
            CapstoneError::Other(format!(
                "{what} needs Capstone to be running: the queue and the processing \
                 pool live in the app. Open it and try again"
            ))
        })
    }

    /// Persist templates. Best effort, like the app's own: the change is
    /// already live in memory and a failed disk write should not make the
    /// call look like it did nothing.
    pub fn save_templates(&self) {
        let templates = self.templates.read().clone();
        if let Err(e) = crate::templates::save(&self.data_dir, &templates) {
            tracing::warn!("templates could not be saved: {e}");
        }
    }

    pub fn save_settings(&self) {
        let settings = self.settings.read().clone();
        if let Err(e) = crate::settings::save(&self.data_dir, &settings) {
            tracing::warn!("settings could not be saved: {e}");
        }
    }
}

pub fn dispatch(ctx: &McpContext, tool: &str, args: &Value) -> Result<Value> {
    tools::dispatch(ctx, tool, args)
}

/// Where Tauri would have put the app's data, worked out without Tauri.
pub fn data_dir() -> Result<PathBuf> {
    let base: Option<PathBuf> = {
        #[cfg(target_os = "macos")]
        {
            std::env::var_os("HOME")
                .map(|h| PathBuf::from(h).join("Library").join("Application Support"))
        }
        #[cfg(target_os = "windows")]
        {
            std::env::var_os("APPDATA").map(PathBuf::from)
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::var_os("HOME")
                        .map(|h| PathBuf::from(h).join(".local").join("share"))
                })
        }
    };

    base.map(|b| b.join(IDENTIFIER))
        .ok_or_else(|| CapstoneError::Other("could not work out the data directory".into()))
}
