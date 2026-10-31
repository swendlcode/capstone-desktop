//! One struct of shared collaborators, built once in `setup` and handed to
//! every command as `State<'_, AppState>`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use parking_lot::{Mutex, RwLock};
use tauri::{AppHandle, Manager};

use crate::error::{CapstoneError, Result};
use crate::models::{QueuedFile, Settings, Template};

pub struct AppState {
    /// A Vec and not a map: the table shows the queue in the order files were
    /// added, and nothing needs random access by id often enough to pay for a
    /// second index.
    pub queue: Arc<Mutex<Vec<QueuedFile>>>,

    pub templates: Arc<RwLock<Vec<Template>>>,
    pub settings: Arc<RwLock<Settings>>,

    /// Raised by `cancel_processing` and cleared when a run starts. The runner
    /// checks it at file boundaries, so a cancel never leaves a half-written
    /// output file on disk for the user to find later.
    pub cancel: Arc<AtomicBool>,

    /// Held for the length of a run so a second `start_processing` is a no-op.
    /// Two pools over one queue would process some files twice and race on
    /// their output paths.
    pub running: Arc<AtomicBool>,

    pub data_dir: PathBuf,
}

impl AppState {
    pub fn init(app: AppHandle) -> Result<Self> {
        let data_dir = app
            .path()
            .app_data_dir()
            .unwrap_or_else(|_| std::env::temp_dir().join("capstone"));
        std::fs::create_dir_all(&data_dir)?;

        let settings = crate::settings::load(&data_dir);
        let templates = crate::templates::load(&data_dir);

        Ok(Self {
            queue: Arc::new(Mutex::new(Vec::new())),
            templates: Arc::new(RwLock::new(templates)),
            settings: Arc::new(RwLock::new(settings)),
            cancel: Arc::new(AtomicBool::new(false)),
            running: Arc::new(AtomicBool::new(false)),
            data_dir,
        })
    }

    /// Best effort on purpose. A settings write that fails should not fail the
    /// command that changed them, because the change is already live in memory
    /// and the user would rather keep working than be told about a disk.
    pub fn save_settings(&self) {
        let settings = self.settings.read().clone();
        if let Err(e) = crate::settings::save(&self.data_dir, &settings) {
            tracing::warn!("could not save settings: {}", e);
        }
    }

    pub fn save_templates(&self) {
        let templates = self.templates.read().clone();
        if let Err(e) = crate::templates::save(&self.data_dir, &templates) {
            tracing::warn!("could not save templates: {}", e);
        }
    }

    /// The template a run will use. Falls back to the stock Festival rather
    /// than erroring: an `active_template_id` pointing at a template the user
    /// has since deleted should not stop them processing anything.
    pub fn active_template(&self) -> Template {
        let id = self.settings.read().active_template_id.clone();
        let templates = self.templates.read();
        templates
            .iter()
            .find(|t| t.id == id)
            .cloned()
            .unwrap_or_else(crate::templates::festival)
    }

    pub fn template_by_id(&self, id: &str) -> Result<Template> {
        self.templates
            .read()
            .iter()
            .find(|t| t.id == id)
            .cloned()
            .ok_or_else(|| CapstoneError::NotFound(format!("template {}", id)))
    }

    pub fn queued_file(&self, id: &str) -> Result<QueuedFile> {
        self.queue
            .lock()
            .iter()
            .find(|f| f.id == id)
            .cloned()
            .ok_or_else(|| CapstoneError::NotFound(format!("queued file {}", id)))
    }

    /// Applies `edit` to the matching row and hands back the updated copy, so a
    /// caller can emit `capstone://file-updated` without taking the lock twice.
    pub fn update_file<F>(&self, id: &str, edit: F) -> Option<QueuedFile>
    where
        F: FnOnce(&mut QueuedFile),
    {
        let mut queue = self.queue.lock();
        let file = queue.iter_mut().find(|f| f.id == id)?;
        edit(file);
        Some(file.clone())
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}
