//! Google Fonts commands: the API key, the catalog, previews and downloads.
//!
//! The key is read from secure storage when a request needs it and is never
//! sent to the webview after it's saved; the UI only ever sees its last four
//! characters.

use crate::secrets::{ending, Secrets, Storage};
use crate::{blocking, AppState, CmdResult};
use font_core::google::{self, Endpoint, GoogleError, GoogleFamily};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::http::{header, Response, StatusCode};
use tauri::{AppHandle, Emitter};

/// The catalog changes a few times a week; refetching it daily is plenty.
const CATALOG_MAX_AGE_SECS: i64 = 24 * 60 * 60;

#[derive(Serialize, Deserialize)]
pub struct CachedCatalog {
    fetched_at: i64,
    families: Vec<GoogleFamily>,
}

pub struct GoogleState {
    secrets: Secrets,
    endpoint: Endpoint,
    /// Managed library folder that downloaded families go into.
    pub root: PathBuf,
    previews: PathBuf,
    catalog_file: PathBuf,
    catalog: Mutex<Option<Arc<CachedCatalog>>>,
    tag_settings_file: PathBuf,
}

/// Whether Google's tags are imported, and how strong a tag must be.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TagSettings {
    enabled: bool,
    /// 0 to 100; Google's tag weights use the same scale.
    min_weight: f32,
    imported_at: Option<i64>,
}

impl Default for TagSettings {
    fn default() -> Self {
        Self { enabled: false, min_weight: 50.0, imported_at: None }
    }
}

impl GoogleState {
    pub fn new(app_data: &Path) -> Self {
        Self {
            secrets: Secrets::new(app_data),
            endpoint: endpoint(),
            root: app_data.join("google-fonts"),
            previews: app_data.join("google-previews"),
            catalog_file: app_data.join("google-catalog.json"),
            catalog: Mutex::new(None),
            tag_settings_file: app_data.join("google-tags.json"),
        }
    }

    fn tag_settings(&self) -> TagSettings {
        std::fs::read(&self.tag_settings_file)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    fn save_tag_settings(&self, t: &TagSettings) -> std::io::Result<()> {
        let tmp = self.tag_settings_file.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec(t).expect("serializable"))?;
        std::fs::rename(&tmp, &self.tag_settings_file)
    }

    fn cached(&self) -> Option<Arc<CachedCatalog>> {
        let mut slot = self.catalog.lock().unwrap_or_else(|e| e.into_inner());
        if slot.is_none() {
            *slot = std::fs::read(&self.catalog_file)
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok())
                .map(Arc::new);
        }
        slot.clone()
    }

    fn store(&self, cat: CachedCatalog) -> Arc<CachedCatalog> {
        if let Ok(bytes) = serde_json::to_vec(&cat) {
            let tmp = self.catalog_file.with_extension("tmp");
            if std::fs::write(&tmp, bytes).is_ok() {
                let _ = std::fs::rename(&tmp, &self.catalog_file);
            }
        }
        let cat = Arc::new(cat);
        *self.catalog.lock().unwrap_or_else(|e| e.into_inner()) = Some(cat.clone());
        cat
    }
}

/// Production always talks to Google. Debug builds can be pointed at a mock
/// server for testing; release builds ignore these variables entirely.
fn endpoint() -> Endpoint {
    #[cfg(debug_assertions)]
    if let Ok(api_url) = std::env::var("FONT_MANAGER_GOOGLE_API") {
        let font_hosts = std::env::var("FONT_MANAGER_GOOGLE_HOSTS")
            .unwrap_or_default()
            .split(',')
            .filter(|h| !h.is_empty())
            .map(str::to_string)
            .collect();
        return Endpoint { api_url, font_hosts };
    }
    Endpoint::default()
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Settings and the key
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleSettings {
    key_saved: bool,
    /// Last four characters only. The key itself never goes to the webview.
    key_ending: Option<String>,
    storage: Option<Storage>,
    catalog_families: Option<usize>,
    catalog_fetched_at: Option<i64>,
    download_folder: String,
}

fn settings(st: &AppState) -> GoogleSettings {
    let g = &st.google;
    let saved = g.secrets.get();
    let cat = g.cached();
    GoogleSettings {
        key_saved: saved.is_some(),
        key_ending: saved.as_ref().map(|(k, _)| ending(k)),
        storage: saved.map(|(_, s)| s),
        catalog_families: cat.as_ref().map(|c| c.families.len()),
        catalog_fetched_at: cat.as_ref().map(|c| c.fetched_at),
        download_folder: g.root.to_string_lossy().into_owned(),
    }
}

#[tauri::command]
pub async fn google_settings(app: AppHandle) -> CmdResult<GoogleSettings> {
    blocking(app, |st| Ok(settings(st))).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveKeyResult {
    settings: GoogleSettings,
    /// False when Google couldn't be reached, so the key was saved unchecked.
    verified: bool,
    warning: Option<String>,
}

/// Check a key with Google, then store it. A key Google rejects is not saved;
/// one that can't be checked because Google is unreachable is saved, with a
/// warning, so someone offline isn't stuck.
#[tauri::command]
pub async fn save_google_key(app: AppHandle, key: String) -> CmdResult<SaveKeyResult> {
    blocking(app, move |st| {
        let key = key.trim().to_string();
        if key.is_empty() {
            return Err("Paste a key first.".to_string().into());
        }
        if key.chars().any(char::is_whitespace) {
            return Err("That doesn't look like a single key; it contains spaces.".to_string().into());
        }
        let (verified, warning) = match google::check_key(&st.google.endpoint, &key) {
            Ok(()) => (true, None),
            Err(GoogleError::Network(e)) => (
                false,
                Some(format!("Saved, but Google couldn't be reached to check the key ({e}).")),
            ),
            Err(e) => return Err(e.to_string().into()),
        };
        st.google
            .secrets
            .set(&key)
            .map_err(|e| format!("Couldn't save the key: {e}"))?;
        Ok(SaveKeyResult { settings: settings(st), verified, warning })
    })
    .await
}

#[tauri::command]
pub async fn remove_google_key(app: AppHandle) -> CmdResult<GoogleSettings> {
    blocking(app, |st| {
        st.google.secrets.delete();
        Ok(settings(st))
    })
    .await
}

// ---------------------------------------------------------------------------
// Catalog
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleFamilySummary {
    family: String,
    category: String,
    styles: usize,
    variable: bool,
    axes: Vec<String>,
    scripts: usize,
    version: String,
    last_modified: String,
    /// "available", "downloaded", or "update" (a newer version exists).
    status: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogView {
    families: Vec<GoogleFamilySummary>,
    fetched_at: i64,
    /// Set when a refresh failed and an older copy is shown instead.
    stale: Option<String>,
}

/// Versions on disk, keyed by folder name. One directory listing instead of
/// a file read per catalog family.
fn downloaded(root: &Path) -> HashMap<String, String> {
    std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let v = std::fs::read_to_string(e.path().join(".google-fonts-version")).ok()?;
            Some((name, v.trim().to_string()))
        })
        .collect()
}

fn summarize(st: &AppState, cat: &CachedCatalog, stale: Option<String>) -> CatalogView {
    let on_disk = downloaded(&st.google.root);
    let families = cat
        .families
        .iter()
        .map(|f| {
            let dir = google::family_dir(&st.google.root, &f.family);
            let key = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let status = match on_disk.get(&key) {
                None => "available",
                Some(v) if *v == f.version => "downloaded",
                Some(_) => "update",
            };
            GoogleFamilySummary {
                family: f.family.clone(),
                category: f.category.clone(),
                styles: f.variants.len(),
                variable: !f.axes.is_empty(),
                axes: f.axes.iter().map(|a| a.tag.clone()).collect(),
                scripts: f.subsets.len(),
                version: f.version.clone(),
                last_modified: f.last_modified.clone(),
                status,
            }
        })
        .collect();
    CatalogView { families, fetched_at: cat.fetched_at, stale }
}

#[tauri::command]
pub async fn google_catalog(app: AppHandle, refresh: bool) -> CmdResult<CatalogView> {
    blocking(app, move |st| {
        let g = &st.google;
        let cached = g.cached();
        let fresh = cached.as_ref().is_some_and(|c| now() - c.fetched_at < CATALOG_MAX_AGE_SECS);
        if fresh && !refresh {
            return Ok(summarize(st, cached.as_ref().unwrap(), None));
        }

        let Some((key, _)) = g.secrets.get() else {
            return Err("Add a Google Fonts API key in Settings first.".to_string().into());
        };
        match google::fetch_catalog(&g.endpoint, &key) {
            Ok(families) => {
                let cat = g.store(CachedCatalog { fetched_at: now(), families });
                // Google's tags change with the catalog.
                reapply_tags(st);
                Ok(summarize(st, &cat, None))
            }
            // Offline with a cached copy: show it, and say it may be out of date.
            Err(GoogleError::Network(e)) if cached.is_some() => {
                Ok(summarize(st, cached.as_ref().unwrap(), Some(format!("Couldn't reach Google ({e}). Showing the list from the last update."))))
            }
            Err(e) => Err(e.to_string().into()),
        }
    })
    .await
}

// ---------------------------------------------------------------------------
// Google's tags
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleTagStatus {
    #[serde(flatten)]
    settings: TagSettings,
    #[serde(flatten)]
    counts: font_core::db::GoogleTagCounts,
    key_saved: bool,
}

fn tag_status(st: &AppState) -> CmdResult<GoogleTagStatus> {
    Ok(GoogleTagStatus {
        settings: st.google.tag_settings(),
        counts: st.index().google_tag_counts()?,
        key_saved: st.google.secrets.get().is_some(),
    })
}

/// Put Google's tags on library families from a catalog.
fn apply_tags(st: &AppState, cat: &CachedCatalog, min_weight: f32) -> CmdResult<()> {
    let library = st.index().family_keys()?;
    let taggings = google::taggings(&cat.families, &library, min_weight);
    st.index().import_google_tags(&taggings)?;
    Ok(())
}

/// After the library or the catalog changes: re-apply Google's tags if
/// they're switched on, from the cached catalog (no network).
pub(crate) fn reapply_tags(st: &AppState) {
    let t = st.google.tag_settings();
    if !t.enabled {
        return;
    }
    if let Some(cat) = st.google.cached() {
        if let Err(e) = apply_tags(st, &cat, t.min_weight) {
            tracing::warn!(error = %e, "re-applying Google tags failed");
        }
    }
}

#[tauri::command]
pub async fn google_tags_status(app: AppHandle) -> CmdResult<GoogleTagStatus> {
    blocking(app, tag_status).await
}

/// Import Google's tags onto matching library families, and keep them up to
/// date from then on. Fetches the catalog if the cached one has no tags.
#[tauri::command]
pub async fn import_google_tags(app: AppHandle, min_weight: f32) -> CmdResult<GoogleTagStatus> {
    blocking(app, move |st| {
        let g = &st.google;
        let min_weight = min_weight.clamp(1.0, 100.0);
        let has_tags = |c: &CachedCatalog| c.families.iter().any(|f| !f.tags.is_empty());
        let cat = match g.cached() {
            Some(c) if has_tags(&c) && now() - c.fetched_at < CATALOG_MAX_AGE_SECS => c,
            _ => {
                let Some((key, _)) = g.secrets.get() else {
                    return Err("Add a Google Fonts API key above first: tags come with Google's font list.".to_string().into());
                };
                let families = google::fetch_catalog(&g.endpoint, &key).map_err(|e| e.to_string())?;
                g.store(CachedCatalog { fetched_at: now(), families })
            }
        };
        if !has_tags(&cat) {
            return Err("Google's font list came back without tags, so there's nothing to import. Try again later.".to_string().into());
        }
        apply_tags(st, &cat, min_weight)?;
        g.save_tag_settings(&TagSettings { enabled: true, min_weight, imported_at: Some(now()) })
            .map_err(|e| format!("Couldn't save the setting: {e}"))?;
        tag_status(st)
    })
    .await
}

/// Remove every imported tag and stop importing. Your own tags stay.
#[tauri::command]
pub async fn remove_google_tags(app: AppHandle) -> CmdResult<GoogleTagStatus> {
    blocking(app, |st| {
        st.index().remove_google_tags()?;
        let mut t = st.google.tag_settings();
        t.enabled = false;
        t.imported_at = None;
        st.google.save_tag_settings(&t).map_err(|e| format!("Couldn't save the setting: {e}"))?;
        tag_status(st)
    })
    .await
}

// ---------------------------------------------------------------------------
// Downloads
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadResult {
    family: String,
    ok: bool,
    error: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DownloadProgress {
    family: String,
    done: usize,
    total: usize,
}

/// Download (or update) families into the managed folder and index them.
/// The folder is an ordinary library folder, so downloaded fonts are toggled,
/// grouped and rescanned like any others.
#[tauri::command]
pub async fn google_download(app: AppHandle, families: Vec<String>) -> CmdResult<Vec<DownloadResult>> {
    let emitter = app.clone();
    blocking(app, move |st| {
        let g = &st.google;
        let Some(cat) = g.cached() else {
            return Err("The Google Fonts list hasn't been loaded yet.".to_string().into());
        };
        std::fs::create_dir_all(&g.root).map_err(|e| e.to_string())?;
        st.index().add_root(&g.root)?;

        let total = families.len();
        let mut results = Vec::with_capacity(total);
        for (i, name) in families.into_iter().enumerate() {
            let _ = emitter.emit("google-download", DownloadProgress { family: name.clone(), done: i, total });
            let outcome = cat
                .families
                .iter()
                .find(|f| f.family == name)
                .ok_or_else(|| format!("{name} isn't in the Google Fonts list"))
                .and_then(|fam| google::install_family(&g.endpoint, &g.root, fam).map_err(|e| e.to_string()))
                .and_then(|dir| {
                    // Index just this family's folder. remove_missing cleans up
                    // files an update renamed or dropped.
                    let report = font_core::scan::scan(std::slice::from_ref(&dir), &Default::default());
                    let mut idx = st.index();
                    idx.upsert_faces(&report.faces).map_err(|e| e.to_string())?;
                    idx.remove_missing(Some(&dir)).map_err(|e| e.to_string())?;
                    Ok(())
                });
            results.push(match outcome {
                Ok(()) => DownloadResult { family: name, ok: true, error: None },
                Err(e) => DownloadResult { family: name, ok: false, error: Some(e) },
            });
        }
        let _ = emitter.emit("google-download", DownloadProgress { family: String::new(), done: total, total });
        if results.iter().any(|r| r.ok) {
            reapply_tags(st);
        }
        Ok(results)
    })
    .await
}

// ---------------------------------------------------------------------------
// Previews: font://localhost/gf/<family>
// ---------------------------------------------------------------------------

/// Serve a family's name-only preview file, downloading it once and caching it.
/// Fetched by the Rust side, so the webview never contacts Google.
pub fn serve_preview(st: &AppState, family: &str) -> Response<Vec<u8>> {
    let respond = |status: StatusCode, msg: &str| {
        Response::builder()
            .status(status)
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
            .header(header::CONTENT_TYPE, "text/plain")
            .body(msg.as_bytes().to_vec())
            .expect("static response")
    };
    let g = &st.google;
    let Some(cat) = g.cached() else {
        return respond(StatusCode::NOT_FOUND, "no catalog");
    };
    let Some(fam) = cat.families.iter().find(|f| f.family == family) else {
        return respond(StatusCode::NOT_FOUND, "unknown family");
    };

    // Versioned file name: an updated family gets a fresh preview.
    let dir_name = google::family_dir(&g.previews, &fam.family);
    let path = dir_name.with_file_name(format!(
        "{}-{}.ttf",
        dir_name.file_name().and_then(|n| n.to_str()).unwrap_or("_"),
        fam.version
    ));

    let bytes = match std::fs::read(&path) {
        Ok(b) => Ok(b),
        Err(_) => {
            let url = fam.menu.as_deref().or_else(|| fam.files.values().next().map(String::as_str));
            match url {
                None => Err("no preview file".to_string()),
                Some(url) => google::download_font(&g.endpoint, url)
                    .map_err(|e| e.to_string())
                    .inspect(|b| {
                        let _ = std::fs::create_dir_all(&g.previews);
                        let _ = std::fs::write(&path, b);
                    }),
            }
        }
    };

    match bytes {
        Ok(b) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "font/ttf")
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
            .header(header::CACHE_CONTROL, "public, max-age=86400")
            .body(b)
            .expect("valid response"),
        Err(e) => respond(StatusCode::BAD_GATEWAY, &e),
    }
}
