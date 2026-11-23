//! The OpenType features a face offers: ligatures, figure styles, small
//! capitals, stylistic sets and so on.
//!
//! ttf-parser lists feature tags, but not the names a designer gives
//! stylistic sets ("ss01: Open digits") and character variants ("cv05:
//! Lowercase l with tail"), nor the characters a variant affects. Those live
//! in each feature's FeatureParams. So this reads the feature lists of `GSUB`
//! and `GPOS` directly. They are small, simple structures, and every read
//! is bounds-checked: a malformed table gives fewer features, never a panic.
//!
//! Spec: https://learn.microsoft.com/typography/opentype/spec/chapter2
//! (FeatureList) and .../features_pt (ssXX and cvXX params).

use serde::Serialize;

use crate::model::FaceRecord;
use crate::PreviewError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FontFeature {
    /// Four-letter tag: "liga", "ss01".
    pub tag: String,
    /// The designer's name for a stylistic set or character variant.
    pub name: Option<String>,
    /// Characters a character variant changes, when the font says.
    pub chars: Vec<char>,
}

/// The features of a face, read from its file.
pub fn features(face: &FaceRecord) -> Result<Vec<FontFeature>, PreviewError> {
    let data = std::fs::read(&face.file_path)?;
    let parsed =
        ttf_parser::Face::parse(&data, face.face_index).map_err(|_| PreviewError::Unsupported)?;
    Ok(feature_list(&parsed))
}

/// Every feature in `GSUB` and `GPOS`, once each, in tag order. Features
/// with no lookups do nothing, so they are left out.
pub fn feature_list(face: &ttf_parser::Face) -> Vec<FontFeature> {
    let mut out: Vec<FontFeature> = Vec::new();
    for table in [b"GSUB", b"GPOS"] {
        let Some(data) = face.raw_face().table(ttf_parser::Tag::from_bytes(table)) else {
            continue;
        };
        for raw in read_feature_list(data) {
            if let Some(existing) = out.iter_mut().find(|f| f.tag == raw.tag) {
                // The same tag recurs per script and language; keep any name.
                if existing.name.is_none() {
                    existing.name = raw.name_id.and_then(|id| crate::parse::best_name(face, id));
                }
                if existing.chars.is_empty() {
                    existing.chars = raw.chars;
                }
                continue;
            }
            out.push(FontFeature {
                name: raw.name_id.and_then(|id| crate::parse::best_name(face, id)),
                tag: raw.tag,
                chars: raw.chars,
            });
        }
    }
    out.sort_by(|a, b| a.tag.cmp(&b.tag));
    out
}

#[derive(Debug, PartialEq)]
struct RawFeature {
    tag: String,
    name_id: Option<u16>,
    chars: Vec<char>,
}

fn u16_at(d: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*d.get(at)?, *d.get(at + 1)?]))
}

fn u24_at(d: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes([0, *d.get(at)?, *d.get(at + 1)?, *d.get(at + 2)?]))
}

/// The features of one GSUB or GPOS table.
fn read_feature_list(d: &[u8]) -> Vec<RawFeature> {
    let mut out = Vec::new();
    // Header: major, minor, scriptList, featureList, lookupList (all u16 here;
    // version 1.1 only adds a field after these).
    let Some(list) = u16_at(d, 6).map(usize::from) else { return out };
    if list == 0 {
        return out;
    }
    let Some(count) = u16_at(d, list) else { return out };
    for i in 0..usize::from(count) {
        let rec = list + 2 + i * 6;
        let Some(tag_bytes) = d.get(rec..rec + 4) else { break };
        let Some(off) = u16_at(d, rec + 4) else { break };
        let feat = list + usize::from(off);
        let Some(params) = u16_at(d, feat) else { continue };
        let Some(lookups) = u16_at(d, feat + 2) else { continue };
        if lookups == 0 {
            continue;
        }
        // Tags are printable ASCII; anything else is a broken table.
        if !tag_bytes.iter().all(|b| (0x20..0x7F).contains(b)) {
            continue;
        }
        let tag = String::from_utf8_lossy(tag_bytes).into_owned();
        let (name_id, chars) = if params == 0 {
            (None, Vec::new())
        } else {
            read_params(d, feat + usize::from(params), &tag)
        };
        out.push(RawFeature { tag, name_id, chars });
    }
    out
}

/// A feature's name, and the characters it affects, from its params.
fn read_params(d: &[u8], at: usize, tag: &str) -> (Option<u16>, Vec<char>) {
    let is = |p: &str| tag.len() == 4 && tag.starts_with(p) && tag[2..].bytes().all(|b| b.is_ascii_digit());
    if is("ss") {
        // version, uiNameID
        let id = u16_at(d, at + 2).filter(|&id| id >= 256);
        return (id, Vec::new());
    }
    if is("cv") {
        // format, featUiLabelNameId, tooltip, sampleText, numNamedParameters,
        // firstParamUiLabelNameId, charCount, then uint24 characters.
        let id = u16_at(d, at + 2).filter(|&id| id >= 256);
        let mut chars = Vec::new();
        if let Some(n) = u16_at(d, at + 12) {
            for k in 0..usize::from(n).min(64) {
                match u24_at(d, at + 14 + k * 3).and_then(char::from_u32) {
                    Some(c) => chars.push(c),
                    None => break,
                }
            }
        }
        return (id, chars);
    }
    (None, Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A GSUB with three features: liga (no params), ss01 named by id 256,
    /// and cv01 named by id 257 affecting "a" and "g". Plus an empty feature,
    /// which must be skipped.
    fn gsub() -> Vec<u8> {
        let mut d = vec![0, 1, 0, 0, 0, 0, 0, 10, 0, 0]; // header, featureList at 10
        let list = d.len();
        let n = 4u16;
        d.extend(n.to_be_bytes());
        let first = 2 + usize::from(n) * 6;
        // Feature tables, laid out after the records.
        let liga = first; // params 0, 1 lookup, index 0 = 6 bytes
        let ss01 = liga + 6; // params at +6, 1 lookup; then params (4 bytes)
        let cv01 = ss01 + 6 + 4; // params at +6, then 14 + 2*3 bytes
        let empty = cv01 + 6 + 20;
        for (tag, off) in [(b"liga", liga), (b"ss01", ss01), (b"cv01", cv01), (b"dlig", empty)] {
            d.extend(tag);
            d.extend((off as u16).to_be_bytes());
        }
        assert_eq!(d.len(), list + first);
        d.extend([0, 0, 0, 1, 0, 0]); // liga
        d.extend([0, 6, 0, 1, 0, 0, 0, 0, 1, 0]); // ss01 + params(version 0, name 256)
        d.extend([0, 6, 0, 1, 0, 0]); // cv01
        d.extend([0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2]); // params, 2 chars
        d.extend([0, 0, 0x61, 0, 0, 0x67]);
        d.extend([0, 0, 0, 0]); // dlig: no lookups
        d
    }

    #[test]
    fn reads_tags_names_and_characters() {
        let f = read_feature_list(&gsub());
        assert_eq!(
            f,
            vec![
                RawFeature { tag: "liga".into(), name_id: None, chars: vec![] },
                RawFeature { tag: "ss01".into(), name_id: Some(256), chars: vec![] },
                RawFeature { tag: "cv01".into(), name_id: Some(257), chars: vec!['a', 'g'] },
            ]
        );
    }

    #[test]
    fn truncated_tables_give_fewer_features_not_panics() {
        let full = gsub();
        for len in 0..full.len() {
            let _ = read_feature_list(&full[..len]);
        }
        assert!(read_feature_list(&[]).is_empty());
    }
}
