//! Face-level activation.
//!
//! Every OS mechanism available to us registers a **file**. Face-level intent
//! therefore needs a resolution step: single-face files activate in place,
//! faces inside a collection get materialised into a cache directory first.
//! That cache is the reason [`crate::sfnt`] exists.
//!
//! Two invariants hold across all platforms:
//!
//! 1. The ledger in SQLite records *intent* at face level; the OS records
//!    *mechanism* at file level. They drift — the user can install fonts
//!    behind our back or we can crash mid-activation — so
//!    [`FontActivator::list_active`] exists and startup reconciles against it.
//! 2. Already-running applications will mostly not notice changes. Newly
//!    launched ones will. We broadcast what each platform offers and do not
//!    pretend to more.

use crate::model::FaceRecord;
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

#[derive(Debug, thiserror::Error)]
pub enum ActivationError {
    #[error("platform refused to register {path}: {reason}")]
    Refused { path: PathBuf, reason: String },
    #[error("face {0} is in an unsupported format")]
    Unsupported(String),
    #[error("sfnt: {0}")]
    Sfnt(#[from] crate::sfnt::SfntError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ActivationError>;

/// What we handed the OS, and what we need to give back to undo it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivationHandle {
    pub face_id: String,
    /// The file actually registered — the original, or a materialised copy.
    pub registered_path: PathBuf,
    /// True when `registered_path` is ours to delete on deactivation.
    pub materialized: bool,
}

pub trait FontActivator: Send + Sync {
    /// Register `path` with the OS. Must be idempotent: re-registering an
    /// already-active file is a no-op, not an error, because reconciliation
    /// will occasionally ask for both.
    fn register(&self, path: &Path) -> Result<()>;

    fn unregister(&self, path: &Path) -> Result<()>;

    /// Everything currently registered for this user, from the OS's own view.
    /// Used at startup to rebuild truth after a crash or an external change.
    fn list_active(&self) -> Result<Vec<PathBuf>>;

    /// Tell running applications something changed. Best-effort by design;
    /// most will ignore it.
    fn notify_change(&self) -> Result<()> {
        Ok(())
    }
}

impl<T: FontActivator + ?Sized> FontActivator for Box<T> {
    fn register(&self, path: &Path) -> Result<()> {
        (**self).register(path)
    }
    fn unregister(&self, path: &Path) -> Result<()> {
        (**self).unregister(path)
    }
    fn list_active(&self) -> Result<Vec<PathBuf>> {
        (**self).list_active()
    }
    fn notify_change(&self) -> Result<()> {
        (**self).notify_change()
    }
}

/// Owns the materialisation cache and drives a platform activator.
pub struct ActivationService<A: FontActivator> {
    activator: A,
    cache_dir: PathBuf,
}

impl<A: FontActivator> ActivationService<A> {
    pub fn new(activator: A, cache_dir: PathBuf) -> std::io::Result<Self> {
        std::fs::create_dir_all(&cache_dir)?;
        Ok(Self { activator, cache_dir })
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Resolve a face to a file the OS can register, materialising if needed.
    ///
    /// Materialised files are named by `face_id`, so the operation is
    /// idempotent and a crash leaves at worst a stale file we can identify.
    pub fn resolve(&self, face: &FaceRecord) -> Result<(PathBuf, bool)> {
        if !face.needs_extraction() {
            if !face.format.webview_loadable() {
                return Err(ActivationError::Unsupported(face.face_id.clone()));
            }
            return Ok((face.file_path.clone(), false));
        }

        let target = self.cache_dir.join(format!("{}.otf", face.face_id));
        if target.exists() {
            return Ok((target, true));
        }

        let data = std::fs::read(&face.file_path)?;
        let extracted = crate::sfnt::extract_face(&data, face.face_index)?;

        // Write to a temp path then rename, so a crash mid-write cannot leave
        // a truncated font that the OS would happily try to register.
        let tmp = target.with_extension("otf.partial");
        std::fs::write(&tmp, &extracted)?;
        std::fs::rename(&tmp, &target)?;
        Ok((target, true))
    }

    pub fn activate(&self, face: &FaceRecord) -> Result<ActivationHandle> {
        let (path, materialized) = self.resolve(face)?;
        self.activator.register(&path)?;
        self.activator.notify_change()?;
        Ok(ActivationHandle {
            face_id: face.face_id.clone(),
            registered_path: path,
            materialized,
        })
    }

    pub fn deactivate(&self, handle: &ActivationHandle) -> Result<()> {
        self.activator.unregister(&handle.registered_path)?;
        if handle.materialized {
            // A failed delete is not a failed deactivation — the font is
            // already unregistered. Leave the file and let cleanup collect it.
            if let Err(e) = std::fs::remove_file(&handle.registered_path) {
                tracing::warn!(path = ?handle.registered_path, error = %e, "cache cleanup failed");
            }
        }
        self.activator.notify_change()?;
        Ok(())
    }

    /// Batch activation amortises the change broadcast, which is the slow part
    /// on Windows and Linux. Activating 200 faces one at a time runs `fc-cache`
    /// 200 times.
    pub fn activate_many(&self, faces: &[FaceRecord]) -> Vec<(String, Result<ActivationHandle>)> {
        let mut out = Vec::with_capacity(faces.len());
        for face in faces {
            let result = (|| {
                let (path, materialized) = self.resolve(face)?;
                self.activator.register(&path)?;
                Ok(ActivationHandle {
                    face_id: face.face_id.clone(),
                    registered_path: path,
                    materialized,
                })
            })();
            out.push((face.face_id.clone(), result));
        }
        let _ = self.activator.notify_change();
        out
    }

    /// Batch counterpart to [`Self::deactivate`]; broadcasts the change once.
    pub fn deactivate_many(&self, handles: &[ActivationHandle]) -> Vec<(String, Result<()>)> {
        let mut out = Vec::with_capacity(handles.len());
        for handle in handles {
            let result = self.activator.unregister(&handle.registered_path).map(|()| {
                if handle.materialized {
                    if let Err(e) = std::fs::remove_file(&handle.registered_path) {
                        tracing::warn!(path = ?handle.registered_path, error = %e, "cache cleanup failed");
                    }
                }
            });
            out.push((handle.face_id.clone(), result));
        }
        let _ = self.activator.notify_change();
        out
    }

    /// Delete materialised faces nothing needs. Only activation writes to
    /// the cache now (previews extract in memory), so anything not in `keep`
    /// is left over from a deactivation that failed to clean up, or a crash.
    /// Returns bytes freed.
    pub fn prune_cache(&self, keep: &std::collections::HashSet<PathBuf>) -> u64 {
        let Ok(entries) = std::fs::read_dir(&self.cache_dir) else { return 0 };
        let mut freed = 0;
        for entry in entries.flatten() {
            let path = entry.path();
            if keep.contains(&path) {
                continue;
            }
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            if std::fs::remove_file(&path).is_ok() {
                freed += size;
            }
        }
        freed
    }

    pub fn activator(&self) -> &A {
        &self.activator
    }

    /// The OS's view of what is active, filtered to files we could have made.
    /// Anything under our cache dir that no ledger row claims is crash debris.
    pub fn reconcile(&self, ledger: &[ActivationHandle]) -> Result<Reconciliation> {
        let on_system = self.activator.list_active()?;
        let claimed: std::collections::HashSet<&Path> =
            ledger.iter().map(|h| h.registered_path.as_path()).collect();

        let mut orphaned = Vec::new();
        for path in &on_system {
            if path.starts_with(&self.cache_dir) && !claimed.contains(path.as_path()) {
                orphaned.push(path.clone());
            }
        }

        let missing = ledger
            .iter()
            .filter(|h| !on_system.contains(&h.registered_path))
            .cloned()
            .collect();

        Ok(Reconciliation { orphaned, missing })
    }
}

#[derive(Debug, Default)]
pub struct Reconciliation {
    /// Registered by us, not in the ledger — unregister and delete.
    pub orphaned: Vec<PathBuf>,
    /// In the ledger, not registered — re-register or clear the row.
    pub missing: Vec<ActivationHandle>,
}

/// The activator for the current platform.
pub fn platform_activator() -> Box<dyn FontActivator> {
    #[cfg(target_os = "macos")]
    return Box::new(macos::MacActivator::new());
    #[cfg(target_os = "windows")]
    return Box::new(windows::WindowsActivator::new());
    #[cfg(target_os = "linux")]
    return Box::new(linux::LinuxActivator::new());
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    return Box::new(Unsupported);
}

/// Preview still works on platforms we cannot activate on; activation fails
/// with a clear reason instead of the app failing to build.
#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
struct Unsupported;

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
impl FontActivator for Unsupported {
    fn register(&self, path: &Path) -> Result<()> {
        Err(ActivationError::Refused {
            path: path.to_path_buf(),
            reason: "font activation is not supported on this platform".into(),
        })
    }
    fn unregister(&self, _: &Path) -> Result<()> {
        Ok(())
    }
    fn list_active(&self) -> Result<Vec<PathBuf>> {
        Ok(Vec::new())
    }
}
