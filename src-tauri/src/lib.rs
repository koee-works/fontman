//! Tauri shell: application state, the `font://` preview protocol, and IPC
//! commands. Everything substantive lives in `font-core`; this file is wiring.
//!
//! Threading rule: no command does I/O on the main thread or while holding
//! the index lock longer than a query. Every command that touches disk runs
//! through [`blocking`], and the preview protocol does the same, so a 20 MB
//! CJK extraction never stalls the UI or another request.

mod google_cmds;
mod library_cmds;
mod secrets;
mod theme_cmds;
mod watch;

use font_core::activate::{platform_activator, ActivationHandle, ActivationService, FontActivator};
use font_core::db::Index;
use font_core::{family, scan, FaceRecord, FamilyActivation, Paths};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::http::{header, Response, StatusCode};
use tauri::{AppHandle, Emitter, Manager};

pub struct AppState {
    pub(crate) google: google_cmds::GoogleState,
    index: Mutex<Index>,
    activation: ActivationService<Box<dyn FontActivator>>,
    /// Folders the OS manages. Faces under these are previewable but never
    /// toggled — there is no API to "deactivate" a system font.
    installed_roots: Vec<PathBuf>,
    scanning: AtomicBool,
    /// Held for the whole of a scan, so scans queue rather than collide: a
    /// rescan asked for while the folder watcher is scanning waits its turn.
    scan_lock: Mutex<()>,
    pub(crate) watch: watch::Watch,
}

impl AppState {
    pub(crate) fn is_installed_path(&self, path: &Path) -> bool {
        self.installed_roots.iter().any(|r| path.starts_with(r))
    }

    pub(crate) fn index(&self) -> std::sync::MutexGuard<'_, Index> {
        // A panic while holding the lock leaves the connection usable;
        // SQLite's own transactions keep the data consistent.
        self.index.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn is_installed(&self, face: &FaceRecord) -> bool {
        self.installed_roots
            .iter()
            .any(|r| face.file_path.starts_with(r))
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub(crate) enum CmdError {
    #[error("{0}")]
    Db(#[from] font_core::db::DbError),
    #[error("{0}")]
    Other(String),
}

impl From<String> for CmdError {
    fn from(s: String) -> Self {
        CmdError::Other(s)
    }
}

impl Serialize for CmdError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub(crate) type CmdResult<T> = Result<T, CmdError>;

/// Run `f` on the blocking pool with access to app state.
pub(crate) async fn blocking<T, F>(app: AppHandle, f: F) -> CmdResult<T>
where
    T: Send + 'static,
    F: FnOnce(&AppState) -> CmdResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || f(&app.state::<AppState>()))
        .await
        .map_err(|e| CmdError::Other(format!("worker failed: {e}")))?
}

// ---------------------------------------------------------------------------
// DTOs. Summaries only — the full record is fetched when a face is opened.
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FaceSummary {
    id: String,
    subfamily: String,
    full_name: Option<String>,
    postscript_name: Option<String>,
    weight: u16,
    width: u16,
    italic: bool,
    variable: bool,
    format: font_core::FontFormat,
    active: bool,
    installed: bool,
    /// Approximate bytes the webview holds once this face is loaded: the file
    /// size (an upper bound for a face extracted from a collection). Lets the
    /// preview loader budget memory, not just count faces.
    bytes: u64,
    /// CSS `font-variation-settings` pinning every axis to its default.
    /// Without it, WebKit and Blink map `font-weight: normal` onto `wght`,
    /// so a variable face whose default is Thin previews as Regular — an
    /// instance its own name table does not describe.
    variation: Option<String>,
    /// File modification time (Unix seconds), for "recently added" sorting.
    modified: i64,
    /// Variation axis tags, such as `wght` and `opsz`.
    axes: Vec<String>,
    /// The `wght` axis range of a variable face, for weight filtering.
    weight_range: Option<[f32; 2]>,
}

fn default_variation(face: &FaceRecord) -> Option<String> {
    if face.axes.is_empty() {
        return None;
    }
    Some(
        face.axes
            .iter()
            .map(|a| format!("\"{}\" {}", a.tag, a.default))
            .collect::<Vec<_>>()
            .join(", "),
    )
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FamilySummary {
    name: String,
    activation: FamilyActivation,
    representative: String,
    variable: bool,
    monospaced: bool,
    /// True when every face is OS-managed: the family row gets no checkbox.
    installed: bool,
    faces: Vec<FaceSummary>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LibraryStatus {
    roots: Vec<RootInfo>,
    face_count: usize,
    active_count: usize,
    /// Files whose bytes also exist elsewhere in the library.
    duplicate_files: usize,
    scanning: bool,
    platform: &'static str,
    watch: watch::WatchStatus,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RootInfo {
    path: String,
    installed: bool,
    exists: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScanSummary {
    files_seen: usize,
    files_parsed: usize,
    files_skipped: usize,
    faces_indexed: usize,
    removed: usize,
    /// Files no longer on disk, including duplicate copies whose styles
    /// live on in another copy (so `removed` doesn't count them).
    files_removed: usize,
    failures: Vec<ScanFailure>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ScanFailure {
    path: String,
    error: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ToggleResult {
    id: String,
    ok: bool,
    error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FaceDetail {
    #[serde(flatten)]
    record: FaceRecord,
    active: bool,
    installed: bool,
    css_family: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConflictReport {
    /// Active faces sharing a PostScript name: undefined behaviour in every
    /// application that resolves fonts by name.
    active: Vec<font_core::Conflict>,
    /// The same, across the whole library — would conflict if both activated.
    library: Vec<font_core::Conflict>,
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[tauri::command]
async fn library_status(app: AppHandle) -> CmdResult<LibraryStatus> {
    blocking(app, |st| {
        let idx = st.index();
        let roots = idx
            .roots()?
            .into_iter()
            .map(|p| RootInfo {
                installed: st.installed_roots.contains(&p),
                exists: p.exists(),
                path: p.to_string_lossy().into_owned(),
            })
            .collect();
        Ok(LibraryStatus {
            roots,
            face_count: idx.face_count()?,
            active_count: idx.active_face_ids()?.len(),
            duplicate_files: idx.duplicate_file_count()?,
            scanning: st.scanning.load(Ordering::Relaxed),
            platform: std::env::consts::OS,
            watch: st.watch.status(),
        })
    })
    .await
}

/// Scan the library, or just one folder of it.
///
/// `folder` limits both the walk and the cleanup of missing files to that
/// folder, which must be a library root or inside one. A targeted scan re-reads
/// every file in the folder (the reason to target one is that something there
/// looks wrong); a whole-library scan only reads files whose size or mtime
/// changed unless `full` is set.
#[tauri::command]
async fn scan_library(
    app: AppHandle,
    full: bool,
    folder: Option<String>,
) -> CmdResult<ScanSummary> {
    let emitter = app.clone();
    blocking(app, move |st| {
        let summary = run_scan(st, &emitter, full, folder.map(PathBuf::from))?;
        if summary.files_parsed > 0 || summary.removed > 0 {
            google_cmds::reapply_tags(st);
        }
        Ok(summary)
    })
    .await
}

/// The scan behind [`scan_library`], also run by the folder watcher.
///
/// `scope` limits both the walk and the cleanup of missing files to that
/// folder, which must be a library root or inside one. A targeted scan re-reads
/// every file in the folder (the reason to target one is that something there
/// looks wrong); a whole-library scan only reads files whose size or mtime
/// changed unless `full` is set. Scans run one at a time: a second one waits.
pub(crate) fn run_scan(
    st: &AppState,
    emitter: &AppHandle,
    full: bool,
    scope: Option<PathBuf>,
) -> CmdResult<ScanSummary> {
    if let Some(dir) = &scope {
        let roots = st.index().roots()?;
        if !roots.iter().any(|r| dir.starts_with(r)) {
            return Err(format!(
                "{} is not in your library. Add it as a folder first.",
                dir.display()
            )
            .into());
        }
        if !dir.is_dir() {
            return Err(format!("Can't find the folder {}.", dir.display()).into());
        }
    }

    let _turn = st.scan_lock.lock().unwrap_or_else(|e| e.into_inner());
    st.scanning.store(true, Ordering::Release);
    // Reset the flag however we leave this function.
    struct Done<'a>(&'a AtomicBool);
    impl Drop for Done<'_> {
        fn drop(&mut self) {
            self.0.store(false, Ordering::Release);
        }
    }
    let _done = Done(&st.scanning);

    // Snapshot, release, scan, reacquire. The scan is the slow part and
    // must not hold the index — previews need it meanwhile.
    let (targets, known) = {
        let idx = st.index();
        let reread_everything = full || scope.is_some();
        let known = if reread_everything {
            Default::default()
        } else {
            idx.known_stats()?
        };
        (
            scope.clone().map(|d| vec![d]).unwrap_or(idx.roots()?),
            known,
        )
    };

    // Progress arrives from worker threads per file; forward at most ten
    // times a second so a 50k-file scan doesn't flood the webview.
    let last = Mutex::new(std::time::Instant::now() - std::time::Duration::from_secs(1));
    let report = scan::scan_with_progress(&targets, &known, &|p| {
        let finished = p.read == p.to_read;
        let mut last = last.lock().unwrap_or_else(|e| e.into_inner());
        if finished || last.elapsed() >= std::time::Duration::from_millis(100) {
            *last = std::time::Instant::now();
            let _ = emitter.emit("scan-progress", p);
        }
    });

    let mut idx = st.index();
    idx.upsert_faces(&report.faces)?;
    let files_before = idx.file_count()?;
    let removed = idx.remove_missing(scope.as_deref())?;
    let files_removed = files_before.saturating_sub(idx.file_count()?);

    Ok(ScanSummary {
        files_seen: report.files_seen,
        files_parsed: report.files_parsed,
        files_skipped: report.files_skipped,
        faces_indexed: report.faces.len(),
        removed,
        files_removed,
        failures: report
            .failures
            .into_iter()
            .map(|(p, e)| ScanFailure {
                path: p.to_string_lossy().into_owned(),
                error: e,
            })
            .collect(),
    })
}

#[tauri::command]
async fn add_root(app: AppHandle, path: String) -> CmdResult<()> {
    blocking(app, move |st| {
        let p = PathBuf::from(&path);
        if !p.is_dir() {
            return Err(format!("not a folder: {path}").into());
        }
        st.index().add_root(&p)?;
        st.watch.sync(st);
        Ok(())
    })
    .await
}

/// Stop indexing a library folder. System font folders can't be removed:
/// their fonts are always on, and the app re-adds them on first run anyway.
#[tauri::command]
async fn remove_root(app: AppHandle, path: String) -> CmdResult<usize> {
    blocking(app, move |st| {
        let p = PathBuf::from(&path);
        if st.installed_roots.contains(&p) {
            return Err("System font folders can't be removed from the library.".to_string().into());
        }
        let outcome = st.index().remove_root(&p)?;
        match outcome {
            Ok(removed) => {
                st.watch.sync(st);
                Ok(removed)
            }
            Err(active) => Err(format!(
                "{active} {} from this folder {} active. Deactivate {} first, then remove the folder.",
                if active == 1 { "style" } else { "styles" },
                if active == 1 { "is" } else { "are" },
                if active == 1 { "it" } else { "them" },
            )
            .into()),
        }
    })
    .await
}

#[tauri::command]
async fn list_families(app: AppHandle) -> CmdResult<Vec<FamilySummary>> {
    blocking(app, |st| {
        let (faces, active) = {
            let idx = st.index();
            (idx.all_faces()?, idx.active_face_ids()?)
        };
        let installed: HashSet<String> = faces
            .iter()
            .filter(|f| st.is_installed(f))
            .map(|f| f.face_id.clone())
            .collect();

        Ok(family::group(faces, &active)
            .into_iter()
            .map(|fam| {
                let representative = fam
                    .representative()
                    .map(|f| f.face_id.clone())
                    .unwrap_or_default();
                let all_installed = fam.faces.iter().all(|f| installed.contains(&f.face_id));
                FamilySummary {
                    name: fam.name,
                    // The checkbox governs library faces only; installed faces
                    // are always on and don't count toward "partial".
                    activation: {
                        let lib: Vec<&FaceRecord> = fam
                            .faces
                            .iter()
                            .filter(|f| !installed.contains(&f.face_id))
                            .collect();
                        let on = lib.iter().filter(|f| active.contains(&f.face_id)).count();
                        match (lib.len(), on) {
                            (0, _) => FamilyActivation::All,
                            (_, 0) => FamilyActivation::None,
                            (n, on) if on == n => FamilyActivation::All,
                            _ => FamilyActivation::Partial,
                        }
                    },
                    representative,
                    variable: fam.faces.iter().any(|f| f.is_variable),
                    monospaced: fam.faces.iter().all(|f| f.monospaced),
                    installed: all_installed,
                    faces: fam
                        .faces
                        .into_iter()
                        .map(|f| FaceSummary {
                            variation: default_variation(&f),
                            bytes: f.file_size,
                            modified: f.modified,
                            axes: f.axes.iter().map(|a| a.tag.clone()).collect(),
                            weight_range: f
                                .axes
                                .iter()
                                .find(|a| a.tag == "wght")
                                .map(|a| [a.min, a.max]),
                            active: active.contains(&f.face_id),
                            installed: installed.contains(&f.face_id),
                            id: f.face_id,
                            subfamily: f.subfamily,
                            full_name: f.full_name,
                            postscript_name: f.postscript_name,
                            weight: f.weight,
                            width: f.width,
                            italic: f.italic || f.oblique,
                            variable: f.is_variable,
                            format: f.format,
                        })
                        .collect(),
                }
            })
            .collect())
    })
    .await
}

#[tauri::command]
async fn face_detail(app: AppHandle, id: String) -> CmdResult<FaceDetail> {
    blocking(app, move |st| {
        let idx = st.index();
        let record = idx.face(&id)?.ok_or_else(|| format!("unknown face {id}"))?;
        Ok(FaceDetail {
            active: idx.active_face_ids()?.contains(&id),
            installed: st.is_installed(&record),
            css_family: record.css_family(),
            record,
        })
    })
    .await
}

#[tauri::command]
async fn face_coverage(app: AppHandle, id: String) -> CmdResult<Vec<[u32; 2]>> {
    blocking(app, move |st| {
        let face = st
            .index()
            .face(&id)?
            .ok_or_else(|| format!("unknown face {id}"))?;
        font_core::coverage(&face).map_err(|e| e.to_string().into())
    })
    .await
}

#[tauri::command]
async fn face_features(app: AppHandle, id: String) -> CmdResult<Vec<font_core::FontFeature>> {
    blocking(app, move |st| {
        let face = st
            .index()
            .face(&id)?
            .ok_or_else(|| format!("unknown face {id}"))?;
        font_core::features(&face).map_err(|e| e.to_string().into())
    })
    .await
}

#[tauri::command]
async fn glyph_outlines(
    app: AppHandle,
    id: String,
    text: String,
) -> CmdResult<font_core::Outlines> {
    blocking(app, move |st| {
        let face = st
            .index()
            .face(&id)?
            .ok_or_else(|| format!("unknown face {id}"))?;
        font_core::outlines(&face, &text).map_err(|e| e.to_string().into())
    })
    .await
}

#[tauri::command]
async fn set_faces_active(
    app: AppHandle,
    ids: Vec<String>,
    active: bool,
) -> CmdResult<Vec<ToggleResult>> {
    blocking(app, move |st| set_active(st, &ids, active)).await
}

/// Activate or deactivate library faces, recording each in the ledger.
/// Shared by the toggle command and Google's download-and-activate.
pub(crate) fn set_active(
    st: &AppState,
    ids: &[String],
    active: bool,
) -> CmdResult<Vec<ToggleResult>> {
    {
        let mut results = Vec::with_capacity(ids.len());
        let (faces, ledger) = {
            let idx = st.index();
            let mut faces = Vec::new();
            for id in ids {
                match idx.face(id)? {
                    Some(f) if st.is_installed(&f) => results.push(ToggleResult {
                        id: id.clone(),
                        ok: false,
                        error: Some("installed by the system; it cannot be toggled here".into()),
                    }),
                    Some(f) => faces.push(f),
                    None => results.push(ToggleResult {
                        id: id.clone(),
                        ok: false,
                        error: Some("face is no longer in the library".into()),
                    }),
                }
            }
            (faces, idx.active_handles()?)
        };

        let ledger: HashMap<&str, &ActivationHandle> =
            ledger.iter().map(|h| (h.face_id.as_str(), h)).collect();

        if active {
            // Already-active faces are a no-op, not an error.
            let todo: Vec<FaceRecord> = faces
                .into_iter()
                .filter(|f| !ledger.contains_key(f.face_id.as_str()))
                .collect();
            let outcomes = st.activation.activate_many(&todo);
            let idx = st.index();
            for (id, outcome) in outcomes {
                results.push(match outcome {
                    Ok(h) => match idx.record_activation(&id, &h.registered_path, h.materialized) {
                        Ok(()) => ToggleResult {
                            id,
                            ok: true,
                            error: None,
                        },
                        Err(e) => ToggleResult {
                            id,
                            ok: false,
                            error: Some(e.to_string()),
                        },
                    },
                    Err(e) => ToggleResult {
                        id,
                        ok: false,
                        error: Some(e.to_string()),
                    },
                });
            }
        } else {
            let handles: Vec<ActivationHandle> = faces
                .iter()
                .filter_map(|f| ledger.get(f.face_id.as_str()).map(|h| (*h).clone()))
                .collect();
            let outcomes = st.activation.deactivate_many(&handles);
            let idx = st.index();
            for (id, outcome) in outcomes {
                results.push(match outcome {
                    Ok(()) => match idx.clear_activation(&id) {
                        Ok(()) => ToggleResult {
                            id,
                            ok: true,
                            error: None,
                        },
                        Err(e) => ToggleResult {
                            id,
                            ok: false,
                            error: Some(e.to_string()),
                        },
                    },
                    Err(e) => ToggleResult {
                        id,
                        ok: false,
                        error: Some(e.to_string()),
                    },
                });
            }
        }
        Ok(results)
    }
}

#[tauri::command]
async fn conflicts(app: AppHandle) -> CmdResult<ConflictReport> {
    blocking(app, |st| {
        let idx = st.index();
        // Only faces whose name is shared are loaded, not the whole library.
        let all = idx.conflict_candidates()?;
        let active_ids = idx.active_face_ids()?;
        // Installed faces are always live, so they count as active here: an
        // activated library face can collide with a system one.
        let live: Vec<FaceRecord> = all
            .iter()
            .filter(|f| active_ids.contains(&f.face_id) || st.is_installed(f))
            .cloned()
            .collect();
        Ok(ConflictReport {
            active: family::conflicts(&live),
            library: family::conflicts(&all),
        })
    })
    .await
}

// ---------------------------------------------------------------------------
// font:// — preview bytes by face id
// ---------------------------------------------------------------------------

/// Face ids are blake3 hex. Anything else is rejected before touching the
/// index: the frontend never gets to name a path.
fn valid_face_id(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn serve_face(st: &AppState, id: &str) -> Response<Vec<u8>> {
    let respond = |status: StatusCode, msg: &str| {
        Response::builder()
            .status(status)
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
            .header(header::CONTENT_TYPE, "text/plain")
            .body(msg.as_bytes().to_vec())
            .expect("static response")
    };

    if !valid_face_id(id) {
        return respond(StatusCode::BAD_REQUEST, "bad face id");
    }
    let face = match st.index().face(id) {
        Ok(Some(f)) => f,
        Ok(None) => return respond(StatusCode::NOT_FOUND, "unknown face"),
        Err(e) => return respond(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    };
    // Lock released. Collections are extracted in memory: writing every
    // previewed face to disk would grow the cache by ~16 MB per CJK face
    // browsed. The webview caches the response, so this runs about once per
    // face per session; only activation writes to the disk cache.
    let bytes = font_core::preview_bytes(&face).map_err(|e| e.to_string());

    match bytes {
        Ok(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, font_core::preview_mime(&face))
            // Fonts are fetched cross-origin from the app's origin and the
            // FontFace API enforces CORS on them.
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
            // The id is derived from the content hash, so a given URL can
            // never serve different bytes: cache forever.
            .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
            .body(bytes)
            .expect("valid response"),
        Err(e) => respond(StatusCode::UNPROCESSABLE_ENTITY, &e),
    }
}

// ---------------------------------------------------------------------------
// Startup
// ---------------------------------------------------------------------------

/// Bring the ledger and the OS back into agreement.
///
/// The ledger is intent; the OS is mechanism. After a crash or an external
/// change they disagree, and the ledger wins: missing registrations are
/// re-registered, and anything in our cache the OS has but the ledger does
/// not claim is crash debris and gets removed.
fn reconcile(st: &AppState) {
    let handles = match st.index().active_handles() {
        Ok(h) => h,
        Err(e) => return tracing::error!(error = %e, "cannot read activation ledger"),
    };
    let rec = match st.activation.reconcile(&handles) {
        Ok(r) => r,
        Err(e) => return tracing::error!(error = %e, "cannot list active fonts"),
    };

    for path in &rec.orphaned {
        let _ = st.activation.activator().unregister(path);
        let _ = std::fs::remove_file(path);
    }
    for handle in &rec.missing {
        let face = st.index().face(&handle.face_id).ok().flatten();
        let outcome = face.as_ref().map(|f| st.activation.activate(f));
        match outcome {
            Some(Ok(h)) => {
                let _ =
                    st.index()
                        .record_activation(&h.face_id, &h.registered_path, h.materialized);
            }
            // The font is gone or unusable; keeping the row would lie.
            _ => {
                let _ = st.index().clear_activation(&handle.face_id);
            }
        }
    }
    if !rec.orphaned.is_empty() || !rec.missing.is_empty() {
        tracing::info!(
            orphaned = rec.orphaned.len(),
            missing = rec.missing.len(),
            "reconciled"
        );
    }

    // Everything in the cache that an active face doesn't need can go.
    let keep: HashSet<PathBuf> = st
        .index()
        .active_handles()
        .unwrap_or_default()
        .into_iter()
        .filter(|h| h.materialized)
        .map(|h| h.registered_path)
        .collect();
    let freed = st.activation.prune_cache(&keep);
    if freed > 0 {
        tracing::info!(bytes = freed, "pruned font cache");
    }
}

fn build_state(app_data: &Path) -> Result<AppState, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(app_data)?;
    let paths = Paths::new(app_data.to_path_buf());
    let index = Index::open(&paths.index_db)?;

    let installed_roots: Vec<PathBuf> = scan::default_roots();
    if index.roots()?.is_empty() {
        // First run: index what is already installed so there is something
        // to browse immediately. Library folders are added by the user.
        for root in installed_roots.iter().filter(|r| r.is_dir()) {
            index.add_root(root)?;
        }
    }

    Ok(AppState {
        google: google_cmds::GoogleState::new(app_data),
        index: Mutex::new(index),
        activation: ActivationService::new(platform_activator(), paths.font_cache)?,
        installed_roots,
        scanning: AtomicBool::new(false),
        scan_lock: Mutex::new(()),
        watch: watch::Watch::new(app_data),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .register_asynchronous_uri_scheme_protocol("font", |ctx, request, responder| {
            let app = ctx.app_handle().clone();
            // font://localhost/<id> on macOS/Linux, http://font.localhost/<id> on
            // Windows. Google previews are font://localhost/gf/<family> (name only) or
            // gf-full/<family> (the regular style in full), with the
            // path percent-encoded by the frontend.
            let path = percent_encoding::percent_decode_str(request.uri().path())
                .decode_utf8_lossy()
                .trim_matches('/')
                .to_string();
            tauri::async_runtime::spawn_blocking(move || {
                let st = app.state::<AppState>();
                let response = if let Some(family) = path.strip_prefix("gf-full/") {
                    google_cmds::serve_preview(&st, family, true)
                } else if let Some(family) = path.strip_prefix("gf/") {
                    google_cmds::serve_preview(&st, family, false)
                } else {
                    serve_face(&st, &path)
                };
                responder.respond(response);
            });
        })
        .setup(|app| {
            let state = build_state(&app.path().app_data_dir()?)?;
            reconcile(&state);
            app.manage(state);
            watch::start(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            library_status,
            scan_library,
            add_root,
            remove_root,
            list_families,
            face_detail,
            face_coverage,
            face_features,
            glyph_outlines,
            set_faces_active,
            conflicts,
            google_cmds::google_settings,
            google_cmds::save_google_key,
            google_cmds::remove_google_key,
            google_cmds::google_catalog,
            google_cmds::google_download,
            google_cmds::google_tags_status,
            google_cmds::import_google_tags,
            google_cmds::remove_google_tags,
            library_cmds::list_collections,
            library_cmds::create_collection,
            library_cmds::rename_collection,
            library_cmds::delete_collection,
            library_cmds::set_collection_members,
            library_cmds::list_pairings,
            library_cmds::save_pairing,
            library_cmds::delete_pairing,
            library_cmds::save_text_file,
            library_cmds::list_tags,
            library_cmds::create_tag,
            library_cmds::rename_tag,
            library_cmds::delete_tag,
            library_cmds::set_tag_families,
            library_cmds::duplicate_groups,
            library_cmds::trash_duplicates,
            watch::set_watching,
            theme_cmds::list_user_themes,
            theme_cmds::import_zed_theme,
            theme_cmds::remove_user_theme,
        ])
        .run(tauri::generate_context!())
        .expect("error while running the application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variable_faces_pin_their_default_instance() {
        let mut face: FaceRecord = serde_json::from_value(serde_json::json!({
            "faceId": "a", "contentHash": "a", "filePath": "/x.ttf", "faceIndex": 0,
            "fileSize": 1, "modified": 0, "postscriptName": null, "family": "F",
            "subfamily": "Thin", "legacyFamily": "F", "legacySubfamily": "Regular",
            "fullName": null, "version": null, "weight": 100, "width": 5,
            "italic": false, "oblique": false, "monospaced": false, "isVariable": true,
            "axes": [
                {"tag": "wght", "min": 100.0, "default": 100.0, "max": 900.0, "name": null, "hidden": false},
                {"tag": "opsz", "min": 8.0, "default": 14.0, "max": 144.0, "name": null, "hidden": false}
            ],
            "format": "truetype", "unitsPerEm": 1000, "glyphCount": 1, "codepointCount": 1
        }))
        .unwrap();
        assert_eq!(
            default_variation(&face).as_deref(),
            Some(r#""wght" 100, "opsz" 14"#)
        );
        face.axes.clear();
        assert_eq!(default_variation(&face), None);
    }

    #[test]
    fn face_ids_are_validated_before_lookup() {
        assert!(valid_face_id(&"a".repeat(64)));
        assert!(!valid_face_id("../../etc/passwd"));
        assert!(!valid_face_id(&"g".repeat(64)));
        assert!(!valid_face_id(&"a".repeat(63)));
    }
}
