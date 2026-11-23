//! Scanning font roots.
//!
//! Hashing dominates the cost of a scan, so the two optimisations that matter
//! are: skip files whose size and mtime are unchanged, and hash the rest in
//! parallel. A 20k-font library goes from minutes to a few seconds.

use crate::model::FaceRecord;
use std::collections::HashMap;
use crate::parse;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Default)]
pub struct ScanReport {
    pub files_seen: usize,
    pub files_parsed: usize,
    pub files_skipped: usize,
    pub faces: Vec<FaceRecord>,
    /// Files we recognised as fonts but could not parse. Surface these — a
    /// silently missing font is the most annoying possible failure here.
    pub failures: Vec<(PathBuf, String)>,
}

/// Size and mtime of files already indexed, keyed by path.
pub type KnownStats = HashMap<PathBuf, (u64, i64)>;

/// Walk `roots` and parse every font found.
///
/// Files in `known` with an unchanged size and mtime are skipped. Pass an
/// empty map for a full re-scan.
///
/// Takes a snapshot rather than the index itself, so callers can release the
/// database before the slow part. A scan of a large library takes seconds;
/// holding the index for that long would freeze every preview request.
pub fn scan(roots: &[PathBuf], known: &KnownStats) -> ScanReport {
    scan_with_progress(roots, known, &|_| {})
}

/// Progress through the slow part of a scan: reading files that changed.
#[derive(Debug, Clone, Copy, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    /// Font files found under the roots.
    pub seen: usize,
    /// Of those, the ones that need reading (new or changed).
    pub to_read: usize,
    pub read: usize,
}

/// [`scan`], calling `on_progress` after each file is read. Called from
/// worker threads; throttle in the callback if it does anything expensive.
pub fn scan_with_progress(
    roots: &[PathBuf],
    known: &KnownStats,
    on_progress: &(dyn Fn(Progress) + Sync),
) -> ScanReport {
    let candidates: Vec<PathBuf> = roots
        .iter()
        .flat_map(|root| {
            WalkDir::new(root)
                .follow_links(false) // a symlink loop in a font folder should not hang the app
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
                .map(|e| e.into_path())
                .filter(|p| parse::has_font_extension(p))
        })
        .collect();

    let files_seen = candidates.len();

    let mut to_parse = Vec::with_capacity(candidates.len());
    let mut skipped = 0usize;
    for path in candidates {
        if let (Some(&(size, mtime)), Ok(meta)) = (known.get(&path), std::fs::metadata(&path)) {
            if meta.len() == size && mtime_of(&meta) == mtime {
                skipped += 1;
                continue;
            }
        }
        to_parse.push(path);
    }

    let read = std::sync::atomic::AtomicUsize::new(0);
    let to_read = to_parse.len();
    on_progress(Progress { seen: files_seen, to_read, read: 0 });

    let results: Vec<std::result::Result<Vec<FaceRecord>, (PathBuf, String)>> = to_parse
        .par_iter()
        .map(|path| {
            let r = parse_one(path).map_err(|e| (path.clone(), e));
            let n = read.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
            on_progress(Progress { seen: files_seen, to_read, read: n });
            r
        })
        .collect();

    let mut report = ScanReport {
        files_seen,
        files_skipped: skipped,
        ..Default::default()
    };
    for result in results {
        match result {
            Ok(faces) => {
                report.files_parsed += 1;
                report.faces.extend(faces);
            }
            Err(f) => report.failures.push(f),
        }
    }
    report
}

fn parse_one(path: &Path) -> std::result::Result<Vec<FaceRecord>, String> {
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;

    // Guard against a "font" that is actually a 2GB file someone renamed.
    // Real fonts top out well under this; CJK collections reach ~50MB.
    const MAX_FONT_BYTES: u64 = 512 * 1024 * 1024;
    if meta.len() > MAX_FONT_BYTES {
        return Err(format!("implausibly large for a font ({} bytes)", meta.len()));
    }

    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    let hash = blake3::hash(&data).to_hex().to_string();

    parse::parse_file(path, &data, &hash, meta.len(), mtime_of(&meta)).map_err(|e| e.to_string())
}

fn mtime_of(meta: &std::fs::Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Default font locations, offered as roots on first run.
pub fn default_roots() -> Vec<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from);

    #[cfg(target_os = "macos")]
    {
        let mut v = vec![PathBuf::from("/System/Library/Fonts"), PathBuf::from("/Library/Fonts")];
        if let Ok(h) = home {
            v.push(h.join("Library/Fonts"));
        }
        v
    }
    #[cfg(target_os = "windows")]
    {
        let mut v = vec![PathBuf::from(r"C:\Windows\Fonts")];
        if let Ok(h) = home {
            v.push(h.join(r"AppData\Local\Microsoft\Windows\Fonts"));
        }
        v
    }
    #[cfg(target_os = "linux")]
    {
        let mut v = vec![
            PathBuf::from("/usr/share/fonts"),
            PathBuf::from("/usr/local/share/fonts"),
        ];
        if let Ok(h) = home {
            v.push(h.join(".local/share/fonts"));
        }
        v
    }
}
