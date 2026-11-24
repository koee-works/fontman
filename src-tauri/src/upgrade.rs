//! Keeping the user's data safe from one version to the next.
//!
//! Everything the app keeps lives in the app data folder, named after the
//! bundle identifier, which installers and the updater never touch: the
//! library index (folders, collections, tags, activations), the Google key's
//! fallback file, watch and tag settings, imported themes, and `ui-settings.json`,
//! where the interface's own settings are mirrored from the webview. Keep the
//! identifier fixed forever, or a new version starts with an empty library.
//!
//! When the version changes, a copy of that data is made before anything is
//! opened, so a database migration that goes wrong can be undone by hand.

use serde::Serialize;
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

const VERSION_FILE: &str = "version.txt";
/// The interface's settings. (`settings.json` is the folder watcher's.)
const SETTINGS_FILE: &str = "ui-settings.json";
const BACKUPS: &str = "backups";
/// Backups kept; older ones are deleted.
const KEEP: usize = 5;
/// Small files worth keeping. Caches (the Google catalog, previews,
/// extracted collection faces) are rebuilt on demand and are left out.
const DATA_FILES: &[&str] = &[
    "index.sqlite",
    "index.sqlite-wal",
    "index.sqlite-shm",
    SETTINGS_FILE,
    "settings.json",
    "google-tags.json",
];

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeInfo {
    /// The version that ran before, when this launch is the first of a new
    /// one. "earlier" when an older build didn't record its version.
    pub previous: Option<String>,
    /// Where this launch's backup went, if one was made.
    pub backup: Option<PathBuf>,
    pub backups_dir: PathBuf,
}

/// Run before opening the index: back up the data if the version changed.
/// Never fails the launch; a backup that can't be made is logged.
pub fn prepare(app_data: &Path, version: &str) -> UpgradeInfo {
    let backups_dir = app_data.join(BACKUPS);
    let recorded = std::fs::read_to_string(app_data.join(VERSION_FILE)).ok().map(|s| s.trim().to_string());
    let has_data = app_data.join("index.sqlite").exists();

    let previous = match recorded {
        Some(v) if v == version => None,
        Some(v) => Some(v),
        // Data with no version file: written by a build from before this
        // file existed.
        None if has_data => Some("earlier".to_string()),
        None => None,
    };

    let mut backup = None;
    if let Some(prev) = &previous {
        match back_up(app_data, &backups_dir, prev) {
            Ok(dir) => backup = Some(dir),
            Err(e) => tracing::warn!(error = %e, "couldn't back up data before the update"),
        }
        prune(&backups_dir, KEEP);
    }
    UpgradeInfo { previous, backup, backups_dir }
}

/// Record that this version has started successfully.
pub fn record(app_data: &Path, version: &str) {
    let _ = std::fs::write(app_data.join(VERSION_FILE), version);
}

fn back_up(app_data: &Path, backups: &Path, previous: &str) -> std::io::Result<PathBuf> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let safe: String = previous.chars().map(|c| if c.is_ascii_alphanumeric() || c == '.' { c } else { '_' }).collect();
    let dir = backups.join(format!("{stamp}-from-{safe}"));
    std::fs::create_dir_all(&dir)?;
    for name in DATA_FILES {
        let from = app_data.join(name);
        if from.is_file() {
            std::fs::copy(&from, dir.join(name))?;
        }
    }
    // Imported themes are the user's own files.
    let themes = app_data.join("themes");
    if themes.is_dir() {
        let to = dir.join("themes");
        std::fs::create_dir_all(&to)?;
        for e in std::fs::read_dir(&themes)?.flatten() {
            if e.path().is_file() {
                std::fs::copy(e.path(), to.join(e.file_name()))?;
            }
        }
    }
    Ok(dir)
}

/// Keep the newest `keep` backups. Names start with a timestamp, so they sort.
fn prune(backups: &Path, keep: usize) {
    let Ok(rd) = std::fs::read_dir(backups) else { return };
    let mut dirs: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    dirs.sort();
    let excess = dirs.len().saturating_sub(keep);
    for d in dirs.into_iter().take(excess) {
        let _ = std::fs::remove_dir_all(d);
    }
}

// ---------------------------------------------------------------------------
// Interface settings
// ---------------------------------------------------------------------------

/// The interface's settings, as the webview last saved them. None before
/// the first save.
pub fn load_settings(app_data: &Path) -> Option<Map<String, Value>> {
    let bytes = std::fs::read(app_data.join(SETTINGS_FILE)).ok()?;
    match serde_json::from_slice::<Value>(&bytes).ok()? {
        Value::Object(m) => Some(m),
        _ => None,
    }
}

/// Replace the saved settings, atomically so a crash never leaves half a file.
pub fn save_settings(app_data: &Path, settings: &Map<String, Value>) -> std::io::Result<()> {
    std::fs::create_dir_all(app_data)?;
    let tmp = app_data.join(format!("{SETTINGS_FILE}.tmp"));
    std::fs::write(&tmp, serde_json::to_vec_pretty(settings).expect("serializable"))?;
    std::fs::rename(&tmp, app_data.join(SETTINGS_FILE))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp() -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "fm-upgrade-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn first_install_makes_no_backup() {
        let d = temp();
        let info = prepare(&d, "0.3.0");
        assert!(info.previous.is_none() && info.backup.is_none());
        record(&d, "0.3.0");
        assert!(prepare(&d, "0.3.0").previous.is_none(), "same version again: nothing to do");
    }

    #[test]
    fn a_new_version_backs_up_data_but_not_caches() {
        let d = temp();
        std::fs::write(d.join("index.sqlite"), b"db").unwrap();
        std::fs::write(d.join("ui-settings.json"), b"{}").unwrap();
        std::fs::write(d.join("google-catalog.json"), b"big cache").unwrap();
        std::fs::create_dir_all(d.join("themes")).unwrap();
        std::fs::write(d.join("themes/mine.json"), b"theme").unwrap();

        // A build from before version.txt existed.
        let info = prepare(&d, "0.3.0");
        assert_eq!(info.previous.as_deref(), Some("earlier"));
        let b = info.backup.unwrap();
        assert_eq!(std::fs::read(b.join("index.sqlite")).unwrap(), b"db");
        assert!(b.join("ui-settings.json").exists());
        assert!(b.join("themes/mine.json").exists());
        assert!(!b.join("google-catalog.json").exists());

        record(&d, "0.3.0");
        let info = prepare(&d, "0.4.0");
        assert_eq!(info.previous.as_deref(), Some("0.3.0"));
        let name = info.backup.unwrap().file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.ends_with("-from-0.3.0"), "{name}");
    }

    #[test]
    fn only_the_newest_backups_are_kept() {
        let d = temp();
        for i in 0..8 {
            std::fs::create_dir_all(d.join(format!("{}-from-x", 1000 + i))).unwrap();
        }
        prune(&d, 5);
        let mut left: Vec<String> =
            std::fs::read_dir(&d).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        left.sort();
        assert_eq!(left, ["1003-from-x", "1004-from-x", "1005-from-x", "1006-from-x", "1007-from-x"]);
    }

    #[test]
    fn settings_round_trip_and_ignore_garbage() {
        let d = temp();
        assert!(load_settings(&d).is_none());
        let mut m = Map::new();
        m.insert("theme".into(), Value::String("one-dark".into()));
        save_settings(&d, &m).unwrap();
        assert_eq!(load_settings(&d).unwrap(), m);
        std::fs::write(d.join(SETTINGS_FILE), b"not json").unwrap();
        assert!(load_settings(&d).is_none());
    }
}
