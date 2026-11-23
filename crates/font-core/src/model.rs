//! Data model.
//!
//! The unit of everything in this app is a **face**, not a file. One file can
//! contain many faces (`.ttc`/`.otc`), and one face can span an entire design
//! space (variable fonts). Families are *derived* from faces, never stored as
//! the source of truth — regrouping is a pure function of face metadata, so a
//! parser fix reshapes the library without a re-scan.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontFormat {
    /// TrueType outlines (`glyf`), standalone file.
    TrueType,
    /// CFF/CFF2 outlines, standalone file.
    OpenType,
    /// A face inside a TrueType/OpenType Collection.
    Collection,
    /// Recognised but not previewable in a webview and not activatable by us.
    Unsupported,
}

impl FontFormat {
    /// Can a browser `@font-face` load this face's *original* file directly?
    ///
    /// Collections cannot — the webview has no way to address face N. Those
    /// have to be materialised through [`crate::sfnt::extract_face`] first.
    pub fn webview_loadable(self) -> bool {
        matches!(self, FontFormat::TrueType | FontFormat::OpenType)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariationAxis {
    /// Four-byte axis tag, e.g. `wght`, `wdth`, `opsz`.
    pub tag: String,
    pub min: f32,
    pub default: f32,
    pub max: f32,
    /// Axis name from the `name` table, if the font supplies one.
    pub name: Option<String>,
    /// Axes flagged hidden in `fvar` should not get a slider in the UI.
    pub hidden: bool,
}

/// One face, as parsed from disk. This is what the index stores.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FaceRecord {
    /// Stable identity: blake3(content_hash || face_index). Survives the file
    /// being moved or renamed, which is the whole point — activation state and
    /// user tags key off this, not off a path.
    pub face_id: String,
    pub content_hash: String,

    pub file_path: PathBuf,
    pub face_index: u32,
    pub file_size: u64,
    /// Unix seconds. Used to skip re-hashing unchanged files on re-scan.
    pub modified: i64,

    /// name ID 6. The identity a consuming application actually resolves, and
    /// therefore the key for conflict detection between two active faces.
    pub postscript_name: Option<String>,

    /// Typographic family (name ID 16), falling back to ID 1.
    pub family: String,
    /// Typographic subfamily (name ID 17), falling back to ID 2.
    pub subfamily: String,
    /// name ID 1 verbatim. Kept because this is what old apps group by, and
    /// showing it explains "why is Bold Condensed its own family over there".
    pub legacy_family: String,
    /// name ID 2 verbatim: one of Regular / Bold / Italic / Bold Italic.
    pub legacy_subfamily: String,
    pub full_name: Option<String>,
    pub version: Option<String>,

    /// `OS/2.usWeightClass`, 1..=1000. 400 = Regular, 700 = Bold.
    pub weight: u16,
    /// `OS/2.usWidthClass`, 1..=9. 5 = Normal.
    pub width: u16,
    pub italic: bool,
    pub oblique: bool,
    pub monospaced: bool,

    pub is_variable: bool,
    pub axes: Vec<VariationAxis>,

    pub format: FontFormat,
    pub units_per_em: u16,
    pub glyph_count: u16,
    /// Number of distinct codepoints in `cmap`. Cheap proxy for language
    /// coverage, and enough to drive a "has CJK / has Cyrillic" filter later.
    pub codepoint_count: u32,
}

impl FaceRecord {
    /// Sort key for styles within a family.
    ///
    /// Alphabetical ordering puts Black above Regular and Thin below Semibold,
    /// which reads as broken. Order by the axes a designer thinks in:
    /// width, then weight, then upright before slanted.
    pub fn style_order(&self) -> (u16, u16, u8) {
        (self.width, self.weight, u8::from(self.italic || self.oblique))
    }

    /// True when this face needs materialising before it can be previewed or
    /// activated on its own.
    pub fn needs_extraction(&self) -> bool {
        self.format == FontFormat::Collection
    }

    /// CSS `font-family` name for preview. Deliberately synthetic.
    ///
    /// Never expose the real family name to the webview: an uninstalled
    /// "Helvetica Neue" would collide with the system copy and you would be
    /// looking at the wrong font with no indication anything was wrong.
    pub fn css_family(&self) -> String {
        format!("pf-{}", &self.face_id[..16])
    }
}

/// A family, derived from a set of faces at query time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Family {
    pub name: String,
    pub faces: Vec<FaceRecord>,
    /// Whether every / some / no face in the family is currently active.
    pub activation: FamilyActivation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FamilyActivation {
    None,
    /// Drives `indeterminate` on the family checkbox.
    Partial,
    All,
}

impl Family {
    /// The face to render the family's specimen with: the one closest to
    /// upright regular. A family card showing Black Italic misrepresents it.
    pub fn representative(&self) -> Option<&FaceRecord> {
        self.faces.iter().min_by_key(|f| {
            let weight_dist = (f.weight as i32 - 400).unsigned_abs();
            let width_dist = (f.width as i32 - 5).unsigned_abs() * 100;
            let slant = u32::from(f.italic || f.oblique) * 1000;
            weight_dist + width_dist + slant
        })
    }
}

/// Two active faces sharing a PostScript name is undefined behaviour in every
/// consuming application — whichever the OS resolves first wins, silently.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conflict {
    pub postscript_name: String,
    pub face_ids: Vec<String>,
}
