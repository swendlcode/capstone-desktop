//! Stock templates, template persistence, and the shared-template file.
//!
//! Stock templates are code, not data: they are rebuilt on every launch and
//! merged over whatever is on disk. That way a shipped tweak to a stock
//! template reaches existing installs, and a user who edited one by hand in the
//! JSON file cannot leave themselves with a setting the app no longer
//! understands. Anything the user made carries `is_stock: false`.
//!
//! The shipped set lives in [`catalog`]; [`share`] is the file format that lets
//! a template be posted and opened by somebody else.

use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::models::Template;

pub mod catalog;
pub mod share;

pub use catalog::{
    archive, club, default_bands, festival, stock, stream, vinyl_rip, ARCHIVE_ID, CLUB_ID,
    FESTIVAL_ID, STREAM_ID, VINYL_ID,
};
pub use share::{SharedTemplate, SHARE_FORMAT_VERSION};

const FILE_NAME: &str = "templates.json";

pub fn path(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE_NAME)
}

/// Never fails, for the same reason settings load does not: the stock
/// templates alone are a working app.
///
/// Everything read off disk goes through [`share::clamped`]. The import path is
/// not the only way a template with an impossible setting can get into
/// `templates.json` — a user can edit the file, and a shared one can be dropped
/// into it by hand — so the bound belongs here rather than only at the door.
pub fn load(data_dir: &Path) -> Vec<Template> {
    let file = path(data_dir);
    let stored: Vec<Template> = if file.exists() {
        match std::fs::read_to_string(&file).map(|s| serde_json::from_str::<Vec<Template>>(&s)) {
            Ok(Ok(list)) => list,
            Ok(Err(e)) => {
                tracing::warn!("templates.json is malformed, using stock only: {}", e);
                Vec::new()
            }
            Err(e) => {
                tracing::warn!("could not read templates.json, using stock only: {}", e);
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    let mut out = stock();
    let stock_ids: Vec<String> = out.iter().map(|t| t.id.clone()).collect();
    // Stock first and user templates after, dropping any stored copy of a
    // stock id. A stale stock template on disk shadowing the shipped one is how
    // a fixed default would fail to reach anybody who had already launched.
    out.extend(
        stored
            .into_iter()
            .filter(|t| !t.is_stock && !stock_ids.contains(&t.id))
            .map(share::clamped),
    );
    out
}

/// Writes only what the user owns. Stock templates are code and would be
/// re-merged on load anyway, so storing them would just be a second source of
/// truth to drift.
pub fn save(data_dir: &Path, templates: &[Template]) -> Result<()> {
    std::fs::create_dir_all(data_dir)?;
    let user: Vec<&Template> = templates.iter().filter(|t| !t.is_stock).collect();
    let json = serde_json::to_string_pretty(&user)?;
    std::fs::write(path(data_dir), json)?;
    Ok(())
}

#[cfg(test)]
#[path = "templates/tests.rs"]
mod tests;
