//! Cover art for one queued file.

/// The file's embedded artwork as a `data:` URI, or `null` when it has none.
///
/// `spawn_blocking` because the first call for a path opens the file and
/// decodes a full-size picture; on the IPC thread that would stall every other
/// command behind it. Repeat calls come back from the cache in `crate::cover`
/// and are cheap, which is what makes it safe for the table to ask per row.
#[tauri::command]
pub async fn get_cover_art(path: String) -> Option<String> {
    tokio::task::spawn_blocking(move || crate::cover::cover_art(&path))
        .await
        .unwrap_or_else(|e| {
            tracing::warn!("cover art task failed: {}", e);
            None
        })
}
