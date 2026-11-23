//! Integration tests against fonts actually installed on the machine.
//! Pass trivially where none exist, e.g. minimal CI images.

use font_core::{coverage, parse, scan, sfnt};
use std::path::PathBuf;

fn system_faces() -> Vec<font_core::FaceRecord> {
    scan::scan(&scan::default_roots(), &Default::default()).faces
}

/// The stored count and the glyph map must agree, for every face. This
/// caught a real bug: counting one cmap subtable dropped supplementary planes.
#[test]
fn stored_codepoint_count_matches_coverage() {
    for face in system_faces() {
        let ranges = coverage(&face).unwrap();
        let total: u32 = ranges.iter().map(|[a, b]| b - a + 1).sum();
        assert_eq!(total, face.codepoint_count, "{}", face.file_path.display());
    }
}

/// Every face of every collection on the system extracts to a file our parser
/// reads back with the same identity. fontTools validates the same output in
/// the README's verification step.
#[test]
fn every_collection_face_extracts_and_reparses() {
    let mut checked = 0;
    for face in system_faces().into_iter().filter(|f| f.needs_extraction()) {
        let data = std::fs::read(&face.file_path).unwrap();
        let out = sfnt::extract_face(&data, face.face_index).unwrap();
        let again = parse::parse_file(&PathBuf::from("x.otf"), &out, "h", 0, 0).unwrap();
        assert_eq!(again.len(), 1);
        let again = &again[0];
        assert_eq!(again.postscript_name, face.postscript_name);
        assert_eq!(again.family, face.family);
        assert_eq!(again.glyph_count, face.glyph_count);
        assert_eq!(again.codepoint_count, face.codepoint_count);
        checked += 1;
    }
    eprintln!("checked {checked} collection faces");
}

/// Feature lists read from real fonts: never panics, and Inter (when present)
/// names its stylistic sets and lists the characters its variants change.
#[test]
fn features_read_from_installed_fonts() {
    let mut named = 0;
    for face in system_faces() {
        let feats = font_core::features(&face).unwrap_or_default();
        if face.family == "Inter" && face.subfamily == "Regular" && face.format == font_core::FontFormat::OpenType {
            // As fontTools lists them for Inter 4 (features with lookups).
            let tags: Vec<&str> = feats.iter().map(|f| f.tag.as_str()).collect();
            for t in ["calt", "case", "cv05", "dlig", "frac", "kern", "smcp", "ss01", "tnum", "zero"] {
                assert!(tags.contains(&t), "{t} missing from {tags:?}");
            }
            let cv05 = feats.iter().find(|f| f.tag == "cv05").unwrap();
            assert_eq!(cv05.name.as_deref(), Some("Lower-case L with tail"));
            named += 1;
        }
    }
    eprintln!("checked Inter: {named}");
}

/// Outlines and metrics from real fonts: heights in order, paths for letters,
/// nothing for a missing character.
#[test]
fn outlines_and_metrics_from_installed_fonts() {
    let mut checked = 0;
    for face in system_faces().into_iter().filter(|f| f.family == "Inter" || f.family == "DejaVu Sans") {
        if face.subfamily != "Regular" {
            continue;
        }
        let o = font_core::outlines(&face, "Hx\u{10FFFD}").unwrap();
        let m = &o.metrics;
        let (x, cap) = (m.x_height.unwrap(), m.cap_height.unwrap());
        assert!(m.descender < 0 && 0 < x && x < cap && cap <= m.ascender, "{m:?}");
        let h = &o.glyphs[0];
        assert!(h.d.starts_with('M') && h.d.ends_with('Z'), "{}", face.file_path.display());
        assert!(h.on_curve.len() >= 12 && h.advance > 0 && h.contours == 1);
        // An H's top is the cap height, to within overshoot-free rounding.
        assert!((h.bbox.unwrap()[3] - cap).abs() <= 2);
        let x_glyph = &o.glyphs[1];
        assert!(x_glyph.bbox.is_some());
        assert!(o.glyphs[2].glyph_id.is_none() && o.glyphs[2].d.is_empty());
        checked += 1;
    }
    eprintln!("checked outlines in {checked} faces");
}
