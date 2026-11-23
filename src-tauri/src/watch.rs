//! Folder watching: rescan the library when fonts are added, changed or
//! removed behind the app's back.
//!
//! Events only say *that* something changed. They are debounced (a font
//! install copies dozens of files) and answered with the ordinary incremental
//! scan, which compares sizes and mtimes and re-reads only what changed, so a
//! missed or duplicated event can never leave the index wrong, only late.

use crate::{run_scan, AppState, CmdResult};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

/// Wait this long after the last event before scanning.
const QUIET: Duration = Duration::from_millis(1500);
/// But never longer than this while events keep coming.
const MAX_WAIT: Duration = Duration::from_secs(10);

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WatchStatus {
    pub enabled: bool,
    /// Library folders being watched right now.
    pub folders: usize,
    /// Why some or all folders can't be watched, if so.
    pub error: Option<String>,
}

#[derive(Default)]
struct Inner {
    watcher: Option<RecommendedWatcher>,
    watched: Vec<PathBuf>,
    error: Option<String>,
}

pub struct Watch {
    settings: PathBuf,
    enabled: AtomicBool,
    inner: Mutex<Inner>,
    /// Wakes the scanning thread. Set once it has started.
    wake: Mutex<Option<Sender<()>>>,
}

#[derive(serde::Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    watch_folders: bool,
}

impl Watch {
    pub fn new(app_data: &Path) -> Self {
        let settings = app_data.join("settings.json");
        let enabled = std::fs::read(&settings)
            .ok()
            .and_then(|b| serde_json::from_slice::<Settings>(&b).ok())
            .map(|s| s.watch_folders)
            .unwrap_or(true);
        Self { settings, enabled: AtomicBool::new(enabled), inner: Default::default(), wake: Mutex::new(None) }
    }

    fn inner(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn status(&self) -> WatchStatus {
        let inner = self.inner();
        WatchStatus {
            enabled: self.enabled.load(Ordering::Relaxed),
            folders: inner.watched.len(),
            error: inner.error.clone(),
        }
    }

    fn ping(&self) {
        if let Some(tx) = self.wake.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            let _ = tx.send(());
        }
    }

    /// Watch exactly the library's folders: call after folders are added or
    /// removed, and when watching is switched on or off.
    pub fn sync(&self, st: &AppState) {
        let mut inner = self.inner();
        if !self.enabled.load(Ordering::Relaxed) {
            *inner = Inner::default();
            return;
        }
        let Some(tx) = self.wake.lock().unwrap_or_else(|e| e.into_inner()).clone() else {
            return; // not started yet; start() syncs
        };
        if inner.watcher.is_none() {
            let handler = move |res: notify::Result<notify::Event>| {
                if let Ok(ev) = res {
                    if relevant(&ev) {
                        let _ = tx.send(());
                    }
                }
            };
            match notify::recommended_watcher(handler) {
                Ok(w) => inner.watcher = Some(w),
                Err(e) => {
                    inner.error = Some(format!("Folder watching isn't available on this system: {e}"));
                    return;
                }
            }
        }

        let roots: Vec<PathBuf> = st
            .index()
            .roots()
            .unwrap_or_default()
            .into_iter()
            .filter(|r| r.is_dir())
            .collect();
        let Inner { watcher, watched, error } = &mut *inner;
        let watcher = watcher.as_mut().expect("created above");
        for old in watched.iter().filter(|p| !roots.contains(p)) {
            let _ = watcher.unwatch(old);
        }
        watched.retain(|p| roots.contains(p));
        let mut failed = Vec::new();
        let new: Vec<PathBuf> = roots.iter().filter(|r| !watched.contains(r)).cloned().collect();
        for root in &new {
            match watcher.watch(root, RecursiveMode::Recursive) {
                Ok(()) => watched.push(root.clone()),
                Err(e) => failed.push(format!("{}: {e}", root.display())),
            }
        }
        *error = (!failed.is_empty()).then(|| {
            format!(
                "Some folders can't be watched, so changes there need a manual check. {}",
                failed.join("; ")
            )
        });
    }

    fn set_enabled(&self, on: bool) -> std::io::Result<()> {
        self.enabled.store(on, Ordering::Relaxed);
        let json = serde_json::to_vec(&Settings { watch_folders: on }).expect("serializable");
        let tmp = self.settings.with_extension("json.tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, &self.settings)
    }
}

/// Only changes that could matter to the index: font files, and folders
/// (which may hold fonts) appearing, moving or going. Not the activation
/// links this app writes into the user's font folder, and not reads.
fn relevant(ev: &notify::Event) -> bool {
    if matches!(ev.kind, EventKind::Access(_)) {
        return false;
    }
    ev.paths.iter().any(|p| {
        let ours = p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with("fm-"));
        !ours && (font_core::parse::has_font_extension(p) || p.extension().is_none())
    })
}

/// Start the scanning thread and begin watching, if switched on.
pub fn start(app: AppHandle) {
    let (tx, rx) = channel::<()>();
    {
        let st = app.state::<AppState>();
        *st.watch.wake.lock().unwrap_or_else(|e| e.into_inner()) = Some(tx);
        st.watch.sync(&st);
    }
    std::thread::Builder::new()
        .name("folder-watch".into())
        .spawn(move || {
            while rx.recv().is_ok() {
                // Debounce: wait for a quiet spell, or MAX_WAIT at most.
                let began = Instant::now();
                loop {
                    match rx.recv_timeout(QUIET) {
                        Ok(()) if began.elapsed() < MAX_WAIT => continue,
                        Ok(()) | Err(RecvTimeoutError::Timeout) => break,
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                }
                let st = app.state::<AppState>();
                if !st.watch.enabled.load(Ordering::Relaxed) {
                    continue;
                }
                match run_scan(&st, &app, false, None) {
                    Ok(summary) if summary.files_parsed > 0 || summary.removed > 0 || summary.files_removed > 0 => {
                        tracing::info!(read = summary.files_parsed, removed = summary.removed, "folder change");
                        crate::google_cmds::reapply_tags(&st);
                        let _ = app.emit("library-changed", summary);
                    }
                    Ok(_) => {}
                    Err(e) => tracing::warn!(error = %e, "scan after folder change failed"),
                }
            }
        })
        .expect("spawn watch thread");
}

/// Switch watching on or off. Switching on also checks for anything that
/// changed while it was off.
#[tauri::command]
pub async fn set_watching(app: AppHandle, enabled: bool) -> CmdResult<WatchStatus> {
    crate::blocking(app, move |st| {
        st.watch.set_enabled(enabled).map_err(|e| format!("Couldn't save the setting: {e}"))?;
        st.watch.sync(st);
        if enabled {
            st.watch.ping();
        }
        Ok(st.watch.status())
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{AccessKind, CreateKind, ModifyKind, RemoveKind};

    fn ev(kind: EventKind, path: &str) -> notify::Event {
        notify::Event::new(kind).add_path(PathBuf::from(path))
    }

    #[test]
    fn only_font_and_folder_changes_count() {
        assert!(relevant(&ev(EventKind::Create(CreateKind::File), "/f/New.otf")));
        assert!(relevant(&ev(EventKind::Remove(RemoveKind::Any), "/f/Old.TTF")));
        assert!(relevant(&ev(EventKind::Create(CreateKind::Folder), "/f/Vendor")));
        assert!(relevant(&ev(EventKind::Modify(ModifyKind::Any), "/f/Changed.ttc")));
        assert!(!relevant(&ev(EventKind::Create(CreateKind::File), "/f/readme.txt")));
        assert!(!relevant(&ev(EventKind::Create(CreateKind::File), "/f/.DS_Store.json")));
        assert!(!relevant(&ev(EventKind::Access(AccessKind::Any), "/f/New.otf")));
        assert!(
            !relevant(&ev(EventKind::Create(CreateKind::File), "/home/u/.local/share/fonts/fm-Inter.otf")),
            "the app's own activation links"
        );
    }

    #[test]
    fn the_setting_persists() {
        let dir = tempfile::tempdir().unwrap();
        let w = Watch::new(dir.path());
        assert!(w.status().enabled, "on by default");
        w.set_enabled(false).unwrap();
        assert!(!Watch::new(dir.path()).status().enabled);
        w.set_enabled(true).unwrap();
        assert!(Watch::new(dir.path()).status().enabled);
    }
}
