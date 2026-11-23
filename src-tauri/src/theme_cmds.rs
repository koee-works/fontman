//! Imported Zed themes: validated, reduced to what the app reads, and kept
//! in the app's data folder. The frontend maps them onto its tokens.

use crate::{blocking, CmdResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

/// Real theme files are tens of kilobytes; this leaves plenty of room.
const MAX_THEME_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ThemeFamily {
    pub name: String,
    #[serde(default)]
    pub author: String,
    pub themes: Vec<Theme>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Theme {
    pub name: String,
    pub appearance: Appearance,
    /// Zed's colour map. Kept whole: unknown keys are harmless.
    pub style: serde_json::Map<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    Light,
    Dark,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserTheme {
    /// File name in the themes folder; the id for removing it.
    file: String,
    family: ThemeFamily,
}

fn themes_dir(app: &AppHandle) -> CmdResult<PathBuf> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("themes");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// Parse a theme file, refusing anything that isn't a usable Zed theme.
pub fn parse_theme(bytes: &[u8]) -> Result<ThemeFamily, String> {
    let fam: ThemeFamily = serde_json::from_slice(bytes).map_err(|e| {
        format!("This isn't a Zed theme file: {e}. Zed themes are JSON with a name and a list of themes.")
    })?;
    if fam.name.trim().is_empty() || fam.themes.is_empty() {
        return Err("This theme file has no themes in it.".into());
    }
    Ok(fam)
}

/// A safe file name from a family name.
fn file_name(name: &str) -> String {
    let stem: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let stem: String = if stem.is_empty() { "theme".into() } else { stem.chars().take(60).collect() };
    format!("{stem}.json")
}

fn read_dir(dir: &Path) -> Vec<UserTheme> {
    let mut out: Vec<UserTheme> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        .filter_map(|e| {
            let bytes = std::fs::read(e.path()).ok()?;
            let family = parse_theme(&bytes).ok()?;
            Some(UserTheme { file: e.file_name().to_string_lossy().into_owned(), family })
        })
        .collect();
    out.sort_by_key(|u| u.family.name.to_lowercase());
    out
}

#[tauri::command]
pub async fn list_user_themes(app: AppHandle) -> CmdResult<Vec<UserTheme>> {
    let dir = themes_dir(&app)?;
    blocking(app, move |_| Ok(read_dir(&dir))).await
}

/// Ask for a Zed theme file and keep a copy. Importing a family again
/// replaces the earlier copy. None if the user cancelled.
#[tauri::command]
pub async fn import_zed_theme(app: AppHandle) -> CmdResult<Option<UserTheme>> {
    let dir = themes_dir(&app)?;
    let dialog = app.clone();
    blocking(app, move |_| {
        let Some(picked) = dialog
            .dialog()
            .file()
            .set_title("Import a Zed theme")
            .add_filter("Zed theme", &["json"])
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        let path = picked.into_path().map_err(|e| e.to_string())?;
        let size = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
        if size > MAX_THEME_BYTES {
            return Err("That file is too large to be a theme.".to_string().into());
        }
        let bytes = std::fs::read(&path).map_err(|e| format!("Couldn't read {}: {e}", path.display()))?;
        let family = parse_theme(&bytes)?;
        let file = file_name(&family.name);
        let json = serde_json::to_vec_pretty(&family).expect("serializable");
        let tmp = dir.join(format!("{file}.tmp"));
        std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, dir.join(&file)).map_err(|e| e.to_string())?;
        Ok(Some(UserTheme { file, family }))
    })
    .await
}

#[tauri::command]
pub async fn remove_user_theme(app: AppHandle, file: String) -> CmdResult<()> {
    let dir = themes_dir(&app)?;
    // Only a plain file name inside the themes folder.
    if file.contains(['/', '\\']) || file.starts_with('.') || !file.ends_with(".json") {
        return Err("Not a theme file.".to_string().into());
    }
    blocking(app, move |_| {
        match std::fs::remove_file(dir.join(&file)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string().into()),
        }
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zed_theme_files_parse_and_others_are_refused() {
        let ok = br##"{"$schema":"https://zed.dev/schema/themes/v0.2.0.json","name":"Mine","author":"Me",
            "themes":[{"name":"Mine Dark","appearance":"dark","style":{"text":"#ffffff","syntax":{}}}]}"##;
        let fam = parse_theme(ok).unwrap();
        assert_eq!(fam.themes[0].appearance, Appearance::Dark);
        assert!(parse_theme(br#"{"name":"x","themes":[]}"#).is_err());
        assert!(parse_theme(br#"{"name":"x","themes":[{"name":"y","appearance":"sepia","style":{}}]}"#).is_err());
        assert!(parse_theme(b"not json").is_err());
    }

    #[test]
    fn file_names_are_safe() {
        assert_eq!(file_name("Catppuccin Mocha"), "catppuccin-mocha.json");
        assert_eq!(file_name("../../etc"), "etc.json");
        assert_eq!(file_name("***"), "theme.json");
    }
}
