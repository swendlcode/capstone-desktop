use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, State};

use crate::audio::decode;
use crate::error::{CapstoneError, Result};
use crate::jobs;
use crate::models::{Analysis, FileStatus, QueuedFile};
use crate::state::AppState;

/// How deep a dropped folder is walked. Sample libraries nest, but not without
/// limit, and an unbounded walk over a mounted volume is how a drop turns into
/// a hang.
const MAX_DEPTH: usize = 12;

/// Expands folders, keeps the audio files, and returns only the rows it
/// actually added -- the frontend appends them to the table it already has
/// rather than replacing it, so a drop of one file does not redraw a queue of
/// three hundred.
///
/// Analysis of the new rows is kicked off before returning, so the Clips,
/// Dynamics, Volume and Pitch columns fill in while the user is still dragging
/// more files in. The rows come back `pending` and reach `analyzed` over
/// `capstone://file-updated`.
#[tauri::command]
pub async fn add_paths(
    paths: Vec<String>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<QueuedFile>> {
    let queue = state.queue.clone();
    let added = tokio::task::spawn_blocking(move || {
        let found = expand(&paths);
        let mut queue = queue.lock();
        let mut added = Vec::new();
        for path in found {
            let path_string = path.to_string_lossy().to_string();
            // The same file dropped twice is one row. Matching on the path and
            // not on content is the right call for a file-in/file-out tool:
            // two identical files in different folders are two jobs.
            if queue.iter().any(|f| f.path == path_string) {
                continue;
            }
            let file = QueuedFile {
                id: uuid::Uuid::new_v4().to_string(),
                filename: path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path_string.clone()),
                path: path_string,
                analysis: None,
                status: FileStatus::Pending,
                result: None,
                error: None,
            };
            queue.push(file.clone());
            added.push(file);
        }
        added
    })
    .await
    .map_err(|e| CapstoneError::Other(e.to_string()))?;

    jobs::analysis::start(app, added.iter().map(|f| f.id.clone()).collect());

    Ok(added)
}

/// Walks every folder given and keeps the audio files, in a stable order.
/// Sorting matters more than it looks: without it the table's order follows
/// whatever the filesystem happened to hand back, which differs between runs
/// on the same folder.
fn expand(paths: &[String]) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for raw in paths {
        let path = Path::new(raw);
        if path.is_dir() {
            for entry in walkdir::WalkDir::new(path)
                .max_depth(MAX_DEPTH)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() && decode::is_audio_file(entry.path()) {
                    found.push(entry.into_path());
                }
            }
        } else if path.is_file() && decode::is_audio_file(path) {
            found.push(path.to_path_buf());
        }
    }
    found.sort();
    found.dedup();
    found
}

/// Re-analyses one row on demand. `add_paths` already analyses what it queues,
/// so this is the retry path: a row that errored, or one the user wants read
/// again after replacing the file on disk.
///
/// The status transitions and the error text live in `jobs::analysis`, shared
/// with the background batch, rather than being written out twice here.
#[tauri::command]
pub async fn analyze_file(id: String, app: AppHandle) -> Result<Analysis> {
    tokio::task::spawn_blocking(move || {
        let binding = app.state::<AppState>();
        jobs::analysis::run_file(&app, binding.inner(), &id)
    })
    .await
    .map_err(|e| CapstoneError::Other(e.to_string()))?
}

#[tauri::command]
pub async fn get_queue(state: State<'_, AppState>) -> Result<Vec<QueuedFile>> {
    Ok(state.queue.lock().clone())
}

#[tauri::command]
pub async fn remove_from_queue(id: String, state: State<'_, AppState>) -> Result<()> {
    state.queue.lock().retain(|f| f.id != id);
    Ok(())
}

#[tauri::command]
pub async fn clear_queue(state: State<'_, AppState>) -> Result<()> {
    state.queue.lock().clear();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> PathBuf {
        let root = std::env::temp_dir()
            .join("capstone-tests")
            .join(format!("expand-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("Tech House")).unwrap();
        for name in ["b.wav", "a.aiff", "notes.txt", "cover.jpg"] {
            std::fs::write(root.join(name), b"x").unwrap();
        }
        std::fs::write(root.join("Tech House").join("c.flac"), b"x").unwrap();
        root
    }

    #[test]
    fn a_dropped_folder_is_walked_and_filtered_to_audio() {
        let root = tree();
        let found = expand(&[root.to_string_lossy().to_string()]);
        let names: Vec<String> = found
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        assert!(names.contains(&"a.aiff".to_string()));
        assert!(names.contains(&"b.wav".to_string()));
        assert!(names.contains(&"c.flac".to_string()));
        assert!(!names.contains(&"notes.txt".to_string()));
        assert!(!names.contains(&"cover.jpg".to_string()));
        assert_eq!(found.len(), 3);
    }

    /// Without the sort the table's order follows whatever the filesystem
    /// happened to hand back, which differs between runs on the same folder.
    #[test]
    fn the_order_is_stable() {
        let root = tree();
        let once = expand(&[root.to_string_lossy().to_string()]);
        let twice = expand(&[root.to_string_lossy().to_string()]);
        assert_eq!(once, twice);
        assert!(once.windows(2).all(|p| p[0] <= p[1]));
    }

    /// A folder dropped together with a file inside it is one job, not two.
    #[test]
    fn a_file_named_twice_is_listed_once() {
        let root = tree();
        let found = expand(&[
            root.to_string_lossy().to_string(),
            root.join("b.wav").to_string_lossy().to_string(),
            root.join("b.wav").to_string_lossy().to_string(),
        ]);
        assert_eq!(found.len(), 3);
    }

    #[test]
    fn a_path_that_does_not_exist_is_skipped_rather_than_failing() {
        let found = expand(&["/no/such/place/track.wav".to_string()]);
        assert!(found.is_empty());
    }
}
