//! Core of a cross-platform font manager.
//!
//! Deliberately free of Tauri: everything here is testable with `cargo test`,
//! and the Tauri layer is a thin IPC wrapper over it.
//!
//! The design rests on one constraint: **a font must be fully previewable
//! without being activated.** That rules out rendering through the OS font
//! stack and pushes preview into the webview via a custom protocol serving
//! raw bytes — which in turn is why [`sfnt`] exists, since a webview cannot
//! address face N of a collection.

pub mod activate;
pub mod db;
pub mod family;
pub mod features;
pub mod google;
pub mod model;
pub mod outline;
pub mod parse;
pub mod scan;
pub mod sfnt;

pub use features::{features, FontFeature};
pub use outline::{outlines, Outlines};
pub use model::{Conflict, FaceRecord, Family, FamilyActivation, FontFormat, VariationAxis};

use std::path::PathBuf;

/// Bytes to serve for a preview request, ready for `@font-face`.
///
/// Collections are materialised on the fly. Note this is the *same* extraction
/// the activation path uses — preview and face-level activation fail on
/// collections for the same reason and are fixed by the same component.
pub fn preview_bytes(face: &FaceRecord) -> Result<Vec<u8>, PreviewError> {
    if !matches!(
        face.format,
        FontFormat::TrueType | FontFormat::OpenType | FontFormat::Collection
    ) {
        return Err(PreviewError::Unsupported);
    }

    let data = std::fs::read(&face.file_path)?;
    match sfnt::materialize(&data, face.face_index)? {
        Some(extracted) => Ok(extracted),
        None => Ok(data),
    }
}

/// Unicode coverage of a face as inclusive `[start, end]` ranges.
///
/// Ranges, not a flat list: a CJK face covers ~45k codepoints in a few
/// hundred runs, and the glyph map only needs to page through them.
pub fn coverage(face: &FaceRecord) -> Result<Vec<[u32; 2]>, PreviewError> {
    let data = std::fs::read(&face.file_path)?;
    let parsed = ttf_parser::Face::parse(&data, face.face_index)
        .map_err(|_| PreviewError::Unsupported)?;

    let cps = parse::unicode_codepoints(&parsed);
    Ok(to_ranges(&cps))
}

fn to_ranges(sorted: &[u32]) -> Vec<[u32; 2]> {
    let mut out: Vec<[u32; 2]> = Vec::new();
    for &cp in sorted {
        match out.last_mut() {
            Some(r) if r[1] + 1 == cp => r[1] = cp,
            _ => out.push([cp, cp]),
        }
    }
    out
}

#[derive(Debug, thiserror::Error)]
pub enum PreviewError {
    #[error("format cannot be rendered in a webview")]
    Unsupported,
    #[error("sfnt: {0}")]
    Sfnt(#[from] sfnt::SfntError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// MIME type for the preview response. `font/otf` and `font/ttf` are both
/// accepted by every engine; the generic one avoids sniffing surprises.
pub fn preview_mime(face: &FaceRecord) -> &'static str {
    match face.format {
        FontFormat::TrueType => "font/ttf",
        // Extracted collection faces keep their original sfnt version, but
        // `font/otf` is accepted for both TrueType and CFF outlines.
        _ => "font/otf",
    }
}

/// Standard application directories.
pub struct Paths {
    pub index_db: PathBuf,
    /// Where extracted collection faces live. Disposable: deleting it costs
    /// only the CPU to re-extract.
    pub font_cache: PathBuf,
}

impl Paths {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self {
            index_db: app_data_dir.join("index.sqlite"),
            font_cache: app_data_dir.join("materialized"),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn ranges_merge_runs() {
        assert_eq!(super::to_ranges(&[32, 33, 34, 40, 42, 43]), vec![[32, 34], [40, 40], [42, 43]]);
        assert!(super::to_ranges(&[]).is_empty());
    }
}
