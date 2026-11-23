//! Parsing a font file into [`FaceRecord`]s.
//!
//! The single most important thing here is name resolution. Grouping by
//! filename or by name ID 1 is what makes other font managers scatter
//! "Helvetica Neue Condensed Bold" into its own family. The typographic names
//! (IDs 16/17) exist precisely to express families with more than the four
//! RIBBI styles, and they are what we group on when present.

use crate::model::{FaceRecord, FontFormat, VariationAxis};
use crate::sfnt;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("not a font file")]
    NotAFont,
    #[error("malformed font: {0}")]
    Malformed(String),
    #[error("sfnt: {0}")]
    Sfnt(#[from] sfnt::SfntError),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// Extensions we attempt to parse at all.
pub const FONT_EXTENSIONS: &[&str] = &["ttf", "otf", "ttc", "otc"];

/// Recognised but never previewable in a webview and not worth activating.
/// Adobe ended Type 1 support in 2023; surface these as unsupported rather
/// than building a second rendering path for them.
pub const LEGACY_EXTENSIONS: &[&str] = &["pfb", "pfm", "afm", "suit", "dfont", "fon", "bdf", "pcf"];

pub fn has_font_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let e = e.to_ascii_lowercase();
            FONT_EXTENSIONS.contains(&e.as_str())
        })
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Name table
// ---------------------------------------------------------------------------

mod name_id {
    pub const FAMILY: u16 = 1;
    pub const SUBFAMILY: u16 = 2;
    pub const FULL_NAME: u16 = 4;
    pub const VERSION: u16 = 5;
    pub const POST_SCRIPT_NAME: u16 = 6;
    pub const TYPOGRAPHIC_FAMILY: u16 = 16;
    pub const TYPOGRAPHIC_SUBFAMILY: u16 = 17;
}

/// Rank a name record so we can pick the best of several encodings.
///
/// Fonts routinely carry the same name under Mac Roman, Windows UCS-2, and
/// sometimes a localised language. We want English, and we prefer the Windows
/// platform because its UCS-2 records decode unambiguously — Mac Roman records
/// mis-decode on anything outside ASCII.
fn name_rank(platform_id: ttf_parser::PlatformId, language_id: u16) -> Option<u8> {
    let platform_id = match platform_id {
        ttf_parser::PlatformId::Unicode => 0u16,
        ttf_parser::PlatformId::Macintosh => 1,
        ttf_parser::PlatformId::Windows => 3,
        _ => return None,
    };
    match platform_id {
        3 if language_id == 0x0409 => Some(0), // Windows, en-US
        3 if language_id & 0x00FF == 0x09 => Some(1), // Windows, any English
        0 => Some(2),                          // Unicode platform
        1 if language_id == 0 => Some(3),      // Macintosh, English
        3 => Some(4),                          // Windows, other language
        _ => None,
    }
}

pub(crate) fn best_name(face: &ttf_parser::Face, id: u16) -> Option<String> {
    let mut best: Option<(u8, String)> = None;
    for name in face.names() {
        if name.name_id != id {
            continue;
        }
        let Some(rank) = name_rank(name.platform_id, name.language_id) else {
            continue;
        };
        // `to_string` handles the UTF-16BE decode for Unicode-encoded records
        // and returns None for encodings ttf-parser will not guess at.
        let Some(value) = name.to_string() else { continue };
        let value = normalize_name(&value);
        if value.is_empty() {
            continue;
        }
        if best.as_ref().is_none_or(|(r, _)| rank < *r) {
            best = Some((rank, value));
        }
    }
    best.map(|(_, v)| v)
}

/// Trim and collapse internal whitespace. Real fonts ship names with trailing
/// spaces, NBSPs and doubled spaces, and those split families that should merge.
fn normalize_name(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ---------------------------------------------------------------------------
// Face parsing
// ---------------------------------------------------------------------------

/// Parse every face in `data`.
///
/// `content_hash` is the caller's blake3 of the whole file — passed in rather
/// than recomputed because the scanner already has it and hashing dominates.
pub fn parse_file(
    path: &Path,
    data: &[u8],
    content_hash: &str,
    file_size: u64,
    modified: i64,
) -> Result<Vec<FaceRecord>, ParseError> {
    let count = sfnt::face_count(data).map_err(|_| ParseError::NotAFont)?;
    let collection = sfnt::is_collection(data);

    let mut out = Vec::with_capacity(count as usize);
    for index in 0..count {
        // One broken face in a collection should not discard the others.
        match parse_face(path, data, index, collection, content_hash, file_size, modified) {
            Ok(face) => out.push(face),
            Err(e) => tracing::warn!(?path, index, error = %e, "skipping face"),
        }
    }
    if out.is_empty() {
        return Err(ParseError::NotAFont);
    }
    Ok(out)
}

fn parse_face(
    path: &Path,
    data: &[u8],
    index: u32,
    collection: bool,
    content_hash: &str,
    file_size: u64,
    modified: i64,
) -> Result<FaceRecord, ParseError> {
    let face = ttf_parser::Face::parse(data, index)
        .map_err(|e| ParseError::Malformed(format!("{e:?}")))?;

    let legacy_family = best_name(&face, name_id::FAMILY).unwrap_or_else(|| {
        path.file_stem().and_then(|s| s.to_str()).unwrap_or("Unknown").to_string()
    });
    let legacy_subfamily =
        best_name(&face, name_id::SUBFAMILY).unwrap_or_else(|| "Regular".to_string());

    // The fallback chain that makes grouping correct.
    let family = best_name(&face, name_id::TYPOGRAPHIC_FAMILY).unwrap_or_else(|| legacy_family.clone());
    let subfamily =
        best_name(&face, name_id::TYPOGRAPHIC_SUBFAMILY).unwrap_or_else(|| legacy_subfamily.clone());

    let axes: Vec<VariationAxis> = face
        .variation_axes()
        .into_iter()
        .map(|a| VariationAxis {
            tag: a.tag.to_string(),
            min: a.min_value,
            default: a.def_value,
            max: a.max_value,
            name: best_name(&face, a.name_id),
            hidden: a.hidden,
        })
        .collect();

    let format = if collection {
        FontFormat::Collection
    } else if face.tables().cff.is_some() || face.tables().cff2.is_some() {
        FontFormat::OpenType
    } else {
        FontFormat::TrueType
    };

    let codepoint_count = unicode_codepoints(&face).len() as u32;

    let face_id = face_id(content_hash, index);

    Ok(FaceRecord {
        face_id,
        content_hash: content_hash.to_string(),
        file_path: path.to_path_buf(),
        face_index: index,
        file_size,
        modified,
        postscript_name: best_name(&face, name_id::POST_SCRIPT_NAME),
        family,
        subfamily,
        legacy_family,
        legacy_subfamily,
        full_name: best_name(&face, name_id::FULL_NAME),
        version: best_name(&face, name_id::VERSION),
        weight: face.weight().to_number(),
        width: face.width().to_number(),
        italic: face.is_italic(),
        oblique: face.is_oblique(),
        monospaced: face.is_monospaced(),
        is_variable: !axes.is_empty(),
        axes,
        format,
        units_per_em: face.units_per_em(),
        glyph_count: face.number_of_glyphs(),
        codepoint_count,
    })
}

/// Every renderable Unicode codepoint the face maps, sorted and unique.
///
/// Must union *all* Unicode subtables. Fonts commonly carry a format 4
/// subtable for the BMP and a format 12 for everything — taking only the
/// first one silently drops the supplementary planes (rare CJK, emoji, math),
/// which undercounts exactly the fonts where coverage matters most.
///
/// Shared by the stored count and the glyph map so the two can never disagree.
pub fn unicode_codepoints(face: &ttf_parser::Face) -> Vec<u32> {
    let mut cps: Vec<u32> = Vec::new();
    if let Some(cmap) = face.tables().cmap {
        for sub in cmap.subtables {
            if sub.is_unicode() {
                sub.codepoints(|cp| cps.push(cp));
            }
        }
    }
    cps.sort_unstable();
    cps.dedup();
    // Control characters have entries in some cmaps but render as nothing.
    cps.retain(|&cp| cp >= 0x20 && !(0x7F..0xA0).contains(&cp));
    cps
}

/// Identity that survives moves and renames.
pub fn face_id(content_hash: &str, index: u32) -> String {
    let mut h = blake3::Hasher::new();
    h.update(content_hash.as_bytes());
    h.update(&index.to_le_bytes());
    h.finalize().to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    use ttf_parser::PlatformId;

    #[test]
    fn windows_english_wins_over_mac() {
        assert!(
            name_rank(PlatformId::Windows, 0x0409).unwrap()
                < name_rank(PlatformId::Macintosh, 0).unwrap()
        );
    }

    #[test]
    fn localized_windows_ranks_below_unicode() {
        assert!(
            name_rank(PlatformId::Unicode, 0).unwrap()
                < name_rank(PlatformId::Windows, 0x0407).unwrap()
        );
    }

    #[test]
    fn custom_platform_is_rejected() {
        assert!(name_rank(PlatformId::Custom, 0).is_none());
    }

    #[test]
    fn names_are_collapsed() {
        assert_eq!(normalize_name("  Helvetica   Neue \n"), "Helvetica Neue");
    }

    #[test]
    fn face_ids_differ_per_index() {
        assert_ne!(face_id("abc", 0), face_id("abc", 1));
        assert_eq!(face_id("abc", 0), face_id("abc", 0));
    }
}
