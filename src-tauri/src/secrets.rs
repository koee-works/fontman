//! Storage for the user's Google Fonts API key.
//!
//! The OS keychain is preferred: Keychain on macOS, Credential Manager on
//! Windows, Secret Service (GNOME Keyring, KWallet) on Linux. When none is
//! available — a Linux session without a keyring daemon, say — the key goes in
//! a file in the app's data folder readable only by this user. Settings says
//! which was used, so the fallback is never silent.

use serde::Serialize;
use std::path::PathBuf;

const SERVICE: &str = "dev.fontmanager.app";
const ACCOUNT: &str = "google-fonts-api-key";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Storage {
    Keychain,
    File,
}

pub struct Secrets {
    fallback: PathBuf,
}

impl Secrets {
    pub fn new(app_data: &std::path::Path) -> Self {
        Self { fallback: app_data.join("google-fonts-api-key") }
    }

    fn entry() -> Option<keyring::Entry> {
        keyring::Entry::new(SERVICE, ACCOUNT).ok()
    }

    pub fn get(&self) -> Option<(String, Storage)> {
        if let Some(Ok(key)) = Self::entry().map(|e| e.get_password()) {
            if !key.is_empty() {
                return Some((key, Storage::Keychain));
            }
        }
        std::fs::read_to_string(&self.fallback)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .map(|k| (k, Storage::File))
    }

    pub fn set(&self, key: &str) -> std::io::Result<Storage> {
        // Read back: some keyrings accept a write and lose it at logout.
        if let Some(entry) = Self::entry() {
            if entry.set_password(key).is_ok() && entry.get_password().ok().as_deref() == Some(key) {
                let _ = std::fs::remove_file(&self.fallback);
                return Ok(Storage::Keychain);
            }
        }
        write_private(&self.fallback, key)?;
        Ok(Storage::File)
    }

    pub fn delete(&self) {
        if let Some(entry) = Self::entry() {
            let _ = entry.delete_credential();
        }
        let _ = std::fs::remove_file(&self.fallback);
    }
}

/// Write a file only the current user can read. On Windows the app data folder
/// is already per-user.
fn write_private(path: &std::path::Path, contents: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&tmp)?;
        f.write_all(contents.as_bytes())?;
    }
    #[cfg(not(unix))]
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)
}

/// The last four characters, for "Key ending in …a1B2".
pub fn ending(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    chars[chars.len().saturating_sub(4)..].iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_file_is_private_and_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("k");
        write_private(&path, "AIza-secret").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "AIza-secret");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
    }

    #[test]
    fn only_the_ending_is_shown() {
        assert_eq!(ending("AIzaSyD-abcdEFGH"), "EFGH");
        assert_eq!(ending("ab"), "ab");
    }
}
