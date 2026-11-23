//! Linux activation via fontconfig.
//!
//! Symlink into the user font directory, then rebuild the cache. Symlinks
//! rather than copies so activation is instant regardless of font size and so
//! a stale link is obviously ours.
//!
//! `fc-cache` is the expensive part and it is per-invocation, not per-font —
//! which is why [`super::ActivationService::activate_many`] exists. Activating
//! a 200-face family one at a time would run it 200 times.

use super::{ActivationError, FontActivator, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct LinuxActivator {
    font_dir: PathBuf,
}

impl LinuxActivator {
    pub fn new() -> Self {
        Self { font_dir: default_font_dir() }
    }

    pub fn with_dir(font_dir: PathBuf) -> Self {
        Self { font_dir }
    }

    fn link_for(&self, path: &Path) -> PathBuf {
        // Prefix so reconciliation can tell our links from the user's own
        // manually-installed fonts and never touch theirs.
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        self.font_dir.join(format!("fm-{name}"))
    }
}

fn default_font_dir() -> PathBuf {
    // XDG first; `~/.fonts` is deprecated and unread by modern fontconfig.
    if let Ok(data) = std::env::var("XDG_DATA_HOME") {
        if !data.is_empty() {
            return PathBuf::from(data).join("fonts");
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".local/share/fonts")
}

impl Default for LinuxActivator {
    fn default() -> Self {
        Self::new()
    }
}

impl FontActivator for LinuxActivator {
    fn register(&self, path: &Path) -> Result<()> {
        std::fs::create_dir_all(&self.font_dir)?;
        let link = self.link_for(path);

        match std::fs::symlink_metadata(&link) {
            Ok(_) => {
                // Point an existing link at the current target rather than
                // failing; the cache path may have been rebuilt.
                std::fs::remove_file(&link)?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }

        std::os::unix::fs::symlink(path, &link).map_err(|e| ActivationError::Refused {
            path: path.to_path_buf(),
            reason: format!("symlink failed: {e}"),
        })?;
        Ok(())
    }

    fn unregister(&self, path: &Path) -> Result<()> {
        let link = self.link_for(path);
        match std::fs::remove_file(&link) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    fn list_active(&self) -> Result<Vec<PathBuf>> {
        let mut out = Vec::new();
        let entries = match std::fs::read_dir(&self.font_dir) {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
            Err(e) => return Err(e.into()),
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !is_ours(&path) {
                continue;
            }
            // Report the *target*, since that is what the ledger stores.
            // A dangling link means the cached file went away — report the
            // link itself so reconciliation can clean it up.
            match std::fs::read_link(&path) {
                Ok(target) => out.push(target),
                Err(_) => out.push(path),
            }
        }
        Ok(out)
    }

    fn notify_change(&self) -> Result<()> {
        // Never `fc-cache -f` by default: forcing a rebuild makes fontconfig
        // sleep ~2 s to outlast filesystem timestamp granularity, which made
        // every toggle take two seconds. Without -f it takes ~10 ms and picks
        // up the change, because adding or removing a link updates the folder's
        // mtime. On filesystems with coarse timestamps two quick changes can
        // share one, so verify, and force only when the cache disagrees.
        if !run_fc_cache(&["-q"], &self.font_dir) {
            return Ok(()); // fontconfig not installed: nothing to notify
        }
        if !self.cache_matches_disk() {
            tracing::info!("fontconfig cache stale after quick update; forcing rebuild");
            run_fc_cache(&["-q", "-f"], &self.font_dir);
        }
        Ok(())
    }
}

impl LinuxActivator {
    /// Does fontconfig list exactly the links we have on disk?
    fn cache_matches_disk(&self) -> bool {
        let on_disk: HashSet<PathBuf> = std::fs::read_dir(&self.font_dir)
            .map(|entries| {
                entries
                    .flatten()
                    .map(|e| e.path())
                    .filter(|p| is_ours(p))
                    .collect()
            })
            .unwrap_or_default();

        let Ok(out) = Command::new("fc-list").args([":", "file"]).output() else {
            return true; // cannot check; the quick update is the best we have
        };
        let listed = our_fonts_in_fc_list(&String::from_utf8_lossy(&out.stdout), &self.font_dir);

        // A link fontconfig can't parse never appears; that costs a forced
        // rebuild each time but stays correct.
        listed == on_disk
    }
}

/// Our links in `fc-list : file` output. Lines look like "/dir/font.ttf: ",
/// once per face, so a collection appears several times.
fn our_fonts_in_fc_list(output: &str, dir: &Path) -> HashSet<PathBuf> {
    output
        .lines()
        .map(|l| PathBuf::from(l.trim_end().trim_end_matches(':')))
        .filter(|p| p.parent() == Some(dir) && is_ours(p))
        .collect()
}

fn is_ours(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with("fm-"))
}

/// Returns false when fc-cache isn't available at all.
fn run_fc_cache(flags: &[&str], dir: &Path) -> bool {
    match Command::new("fc-cache").args(flags).arg(dir).status() {
        Ok(s) => {
            if !s.success() {
                tracing::warn!(code = ?s.code(), "fc-cache exited non-zero");
            }
            true
        }
        Err(e) => {
            tracing::warn!(error = %e, "fc-cache not available");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("Inter-Regular.otf");
        std::fs::write(&src, b"not really a font").unwrap();

        let activator = LinuxActivator::with_dir(dir.path().join("fonts"));
        activator.register(&src).unwrap();
        assert_eq!(activator.list_active().unwrap(), vec![src.clone()]);

        activator.unregister(&src).unwrap();
        assert!(activator.list_active().unwrap().is_empty());
    }

    #[test]
    fn register_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("A.otf");
        std::fs::write(&src, b"x").unwrap();
        let activator = LinuxActivator::with_dir(dir.path().join("fonts"));

        activator.register(&src).unwrap();
        activator.register(&src).unwrap();
        assert_eq!(activator.list_active().unwrap().len(), 1);
    }

    #[test]
    fn unregister_of_unknown_font_is_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let activator = LinuxActivator::with_dir(dir.path().join("fonts"));
        assert!(activator.unregister(Path::new("/nowhere/B.otf")).is_ok());
    }

    #[test]
    fn reads_our_links_from_fc_list_output() {
        let dir = Path::new("/home/u/.local/share/fonts");
        let out = "/home/u/.local/share/fonts/fm-A.ttf: \n\
                   /home/u/.local/share/fonts/fm-Coll.ttc: \n\
                   /home/u/.local/share/fonts/fm-Coll.ttc: \n\
                   /home/u/.local/share/fonts/TheirOwn.ttf: \n\
                   /home/u/.local/share/fonts/sub/fm-Nested.ttf: \n\
                   /usr/share/fonts/fm-Elsewhere.ttf: \n";
        let got = our_fonts_in_fc_list(out, dir);
        let want: HashSet<PathBuf> = [dir.join("fm-A.ttf"), dir.join("fm-Coll.ttc")].into();
        assert_eq!(got, want);
    }

    #[test]
    fn ignores_fonts_the_user_installed_themselves() {
        let dir = tempfile::tempdir().unwrap();
        let font_dir = dir.path().join("fonts");
        std::fs::create_dir_all(&font_dir).unwrap();
        std::fs::write(font_dir.join("TheirOwn.otf"), b"x").unwrap();

        let activator = LinuxActivator::with_dir(font_dir);
        assert!(activator.list_active().unwrap().is_empty());
    }
}
