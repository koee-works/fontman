//! Collections, style viewer pairings, and saving exported text.

use crate::{blocking, CmdResult};
use font_core::db::{Collection, DuplicateGroup, NameError, Pairing, Tag};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

fn named<T>(r: Result<T, NameError>) -> CmdResult<T> {
    r.map_err(|e| e.to_string().into())
}

// -- collections ---------------------------------------------------------------

#[tauri::command]
pub async fn list_collections(app: AppHandle) -> CmdResult<Vec<Collection>> {
    blocking(app, |st| Ok(st.index().collections()?)).await
}

/// Create a collection, optionally starting with some faces.
#[tauri::command]
pub async fn create_collection(
    app: AppHandle,
    name: String,
    face_ids: Vec<String>,
) -> CmdResult<Collection> {
    blocking(app, move |st| named(st.index().create_collection(&name, &face_ids)?)).await
}

#[tauri::command]
pub async fn rename_collection(app: AppHandle, id: i64, name: String) -> CmdResult<()> {
    blocking(app, move |st| named(st.index().rename_collection(id, &name)?)).await
}

/// Deletes the collection only; its fonts stay in the library, and stay
/// active if they were.
#[tauri::command]
pub async fn delete_collection(app: AppHandle, id: i64) -> CmdResult<()> {
    blocking(app, move |st| {
        st.index().delete_collection(id)?;
        Ok(())
    })
    .await
}

/// Add faces to a collection (`member` true) or take them out.
#[tauri::command]
pub async fn set_collection_members(
    app: AppHandle,
    id: i64,
    face_ids: Vec<String>,
    member: bool,
) -> CmdResult<Collection> {
    blocking(app, move |st| {
        let mut idx = st.index();
        if !idx.set_collection_members(id, &face_ids, member)? {
            return Err("That collection no longer exists.".to_string().into());
        }
        idx.collections()?
            .into_iter()
            .find(|c| c.id == id)
            .ok_or_else(|| "That collection no longer exists.".to_string().into())
    })
    .await
}

// -- tags ----------------------------------------------------------------------

#[tauri::command]
pub async fn list_tags(app: AppHandle) -> CmdResult<Vec<Tag>> {
    blocking(app, |st| Ok(st.index().tags()?)).await
}

/// Create a tag, optionally on some families from the start.
#[tauri::command]
pub async fn create_tag(app: AppHandle, name: String, families: Vec<String>) -> CmdResult<Tag> {
    blocking(app, move |st| named(st.index().create_tag(&name, &families)?)).await
}

#[tauri::command]
pub async fn rename_tag(app: AppHandle, id: i64, name: String) -> CmdResult<()> {
    blocking(app, move |st| named(st.index().rename_tag(id, &name)?)).await
}

/// Removes the tag from every family; the fonts are untouched.
#[tauri::command]
pub async fn delete_tag(app: AppHandle, id: i64) -> CmdResult<()> {
    blocking(app, move |st| {
        st.index().delete_tag(id)?;
        Ok(())
    })
    .await
}

/// Put a tag on families (`tagged` true) or take it off.
#[tauri::command]
pub async fn set_tag_families(
    app: AppHandle,
    id: i64,
    families: Vec<String>,
    tagged: bool,
) -> CmdResult<Tag> {
    blocking(app, move |st| {
        let mut idx = st.index();
        if !idx.set_tag_families(id, &families, tagged)? {
            return Err("That tag no longer exists.".to_string().into());
        }
        idx.tags()?
            .into_iter()
            .find(|t| t.id == id)
            .ok_or_else(|| "That tag no longer exists.".to_string().into())
    })
    .await
}

// -- duplicates ------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateCopyView {
    path: PathBuf,
    modified: i64,
    in_use: bool,
    /// In a system font folder: the OS owns it, so it can't be trashed here.
    installed: bool,
    /// An active style is registered from this exact file.
    active: bool,
    exists: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroupView {
    content_hash: String,
    file_size: u64,
    faces: Vec<String>,
    copies: Vec<DuplicateCopyView>,
}

fn duplicate_views(st: &crate::AppState) -> CmdResult<Vec<DuplicateGroupView>> {
    let (groups, handles) = {
        let idx = st.index();
        (idx.duplicate_groups()?, idx.active_handles()?)
    };
    let registered: HashSet<PathBuf> =
        handles.into_iter().filter(|h| !h.materialized).map(|h| h.registered_path).collect();
    Ok(groups
        .into_iter()
        .map(|g: DuplicateGroup| DuplicateGroupView {
            content_hash: g.content_hash,
            file_size: g.file_size,
            faces: g.faces,
            copies: g
                .copies
                .into_iter()
                .map(|c| DuplicateCopyView {
                    installed: st.is_installed_path(&c.path),
                    active: registered.contains(&c.path),
                    exists: c.path.exists(),
                    path: c.path,
                    modified: c.modified,
                    in_use: c.in_use,
                })
                .collect(),
        })
        .collect())
}

#[tauri::command]
pub async fn duplicate_groups(app: AppHandle) -> CmdResult<Vec<DuplicateGroupView>> {
    blocking(app, duplicate_views).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashOutcome {
    path: PathBuf,
    ok: bool,
    error: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashReport {
    results: Vec<TrashOutcome>,
    groups: Vec<DuplicateGroupView>,
}

/// Why a copy may not be trashed, if it may not. `remaining` counts the
/// group's copies still on disk, including this one.
fn refuse(installed: bool, active: bool, remaining: usize) -> Option<&'static str> {
    if installed {
        Some("It's in a system font folder, which the OS manages.")
    } else if active {
        Some("An active style is registered from this copy. Deactivate it first, or keep this copy.")
    } else if remaining <= 1 {
        Some("It's the last copy.")
    } else {
        None
    }
}

/// Move duplicate copies to the Trash (Recycle Bin), never deleting outright.
/// Each copy is checked first: system fonts, copies an active style is
/// registered from, and the last remaining copy are refused.
#[tauri::command]
pub async fn trash_duplicates(app: AppHandle, paths: Vec<PathBuf>) -> CmdResult<TrashReport> {
    blocking(app, move |st| {
        let views = duplicate_views(st)?;
        let mut group_of: HashMap<&PathBuf, usize> = HashMap::new();
        let mut remaining: Vec<usize> = Vec::new();
        for (i, g) in views.iter().enumerate() {
            remaining.push(g.copies.iter().filter(|c| c.exists).count());
            for c in &g.copies {
                group_of.insert(&c.path, i);
            }
        }
        let copy = |p: &PathBuf| {
            group_of.get(p).and_then(|&i| views[i].copies.iter().find(|c| &c.path == p))
        };

        let mut results = Vec::new();
        let mut trashed = Vec::new();
        for path in &paths {
            let (Some(&gi), Some(c)) = (group_of.get(path), copy(path)) else {
                results.push(TrashOutcome {
                    path: path.clone(),
                    ok: false,
                    error: Some("It isn't a known duplicate.".into()),
                });
                continue;
            };
            if let Some(why) = refuse(c.installed, c.active, remaining[gi]) {
                results.push(TrashOutcome { path: path.clone(), ok: false, error: Some(why.into()) });
                continue;
            }
            match trash::delete(path) {
                Ok(()) => {
                    remaining[gi] -= 1;
                    trashed.push(path.clone());
                    results.push(TrashOutcome { path: path.clone(), ok: true, error: None });
                }
                Err(e) => results.push(TrashOutcome {
                    path: path.clone(),
                    ok: false,
                    error: Some(format!("Couldn't move it to the Trash: {e}")),
                }),
            }
        }
        drop(views);
        st.index().forget_files(&trashed)?;
        Ok(TrashReport { results, groups: duplicate_views(st)? })
    })
    .await
}

// -- pairings ------------------------------------------------------------------

/// Pairings are small; anything bigger than this is not one.
const MAX_PAIRING_BYTES: usize = 64 * 1024;

#[tauri::command]
pub async fn list_pairings(app: AppHandle) -> CmdResult<Vec<Pairing>> {
    blocking(app, |st| Ok(st.index().pairings()?)).await
}

/// Save a new pairing (`id` null) or overwrite an existing one.
#[tauri::command]
pub async fn save_pairing(
    app: AppHandle,
    id: Option<i64>,
    name: String,
    settings: serde_json::Value,
) -> CmdResult<Pairing> {
    blocking(app, move |st| {
        if !settings.is_object() {
            return Err("A pairing's settings must be an object.".to_string().into());
        }
        let json = settings.to_string();
        if json.len() > MAX_PAIRING_BYTES {
            return Err("That pairing is too large to save.".to_string().into());
        }
        named(st.index().save_pairing(id, &name, &json)?)
    })
    .await
}

#[tauri::command]
pub async fn delete_pairing(app: AppHandle, id: i64) -> CmdResult<()> {
    blocking(app, move |st| {
        st.index().delete_pairing(id)?;
        Ok(())
    })
    .await
}

// -- export --------------------------------------------------------------------

/// Kinds of file the frontend may save. The user picks the path in a native
/// dialog opened here, so the webview never names a path to write.
fn export_kind(kind: &str) -> Option<(&'static str, &'static str)> {
    match kind {
        "css" => Some(("CSS", "css")),
        "json" => Some(("JSON", "json")),
        _ => None,
    }
}

/// Ask where to save, then write `contents` there. Returns the path, or None
/// if the user cancelled.
#[tauri::command]
pub async fn save_text_file(
    app: AppHandle,
    kind: String,
    file_name: String,
    contents: String,
) -> CmdResult<Option<String>> {
    let (filter, ext) =
        export_kind(&kind).ok_or_else(|| format!("can't save files of type {kind}"))?;
    let dialog = app.clone();
    blocking(app, move |_| {
        let picked = dialog
            .dialog()
            .file()
            .set_title("Export pairing")
            .set_file_name(&file_name)
            .add_filter(filter, &[ext])
            .blocking_save_file();
        let Some(picked) = picked else { return Ok(None) };
        let mut path = picked.into_path().map_err(|e| e.to_string())?;
        if path.extension().is_none() {
            path.set_extension(ext);
        }
        std::fs::write(&path, contents)
            .map_err(|e| format!("Couldn't save {}: {e}", path.display()))?;
        Ok(Some(path.to_string_lossy().into_owned()))
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trashing_is_refused_for_system_active_and_last_copies() {
        assert!(refuse(true, false, 3).is_some());
        assert!(refuse(false, true, 3).is_some());
        assert!(refuse(false, false, 1).is_some());
        assert_eq!(refuse(false, false, 2), None);
    }

    #[test]
    fn only_known_export_kinds_are_accepted() {
        assert_eq!(export_kind("css"), Some(("CSS", "css")));
        assert_eq!(export_kind("json"), Some(("JSON", "json")));
        assert_eq!(export_kind("sh"), None);
        assert_eq!(export_kind("../css"), None);
    }
}
