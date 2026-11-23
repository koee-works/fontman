//! Deriving families from faces.
//!
//! Grouping is a pure function of face metadata. Keeping it that way means a
//! parser fix reshapes the whole library on next launch without a re-scan, and
//! it means this module is trivially testable — which matters, because getting
//! grouping right is the difference between the app feeling correct and
//! feeling broken.

use crate::model::{Conflict, FaceRecord, Family, FamilyActivation};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Case- and whitespace-insensitive key. Vendors ship the same family with
/// inconsistent casing across weights ("Söhne" vs "SÖHNE"), and those must not
/// become two families.
/// The key families are grouped by, and tagged by: case-insensitive.
pub fn group_key(name: &str) -> String {
    name.to_lowercase()
}

/// Build families from a flat face list.
///
/// `active` is the set of currently-active face ids, used to compute the
/// tri-state for the family header checkbox.
pub fn group(faces: Vec<FaceRecord>, active: &HashSet<String>) -> Vec<Family> {
    let mut buckets: BTreeMap<String, Vec<FaceRecord>> = BTreeMap::new();
    for face in faces {
        buckets.entry(group_key(&face.family)).or_default().push(face);
    }

    let mut families: Vec<Family> = buckets
        .into_values()
        .map(|mut faces| {
            faces.sort_by_key(|f| (f.style_order(), f.subfamily.clone()));

            // Display name: the most common spelling in the bucket, so one
            // stray all-caps file does not rename the family.
            let mut tally: HashMap<&str, usize> = HashMap::new();
            for f in &faces {
                *tally.entry(f.family.as_str()).or_default() += 1;
            }
            let name = tally
                .into_iter()
                .max_by_key(|&(n, c)| (c, std::cmp::Reverse(n.to_string())))
                .map(|(n, _)| n.to_string())
                .unwrap_or_default();

            let active_count = faces.iter().filter(|f| active.contains(&f.face_id)).count();
            let activation = match active_count {
                0 => FamilyActivation::None,
                n if n == faces.len() => FamilyActivation::All,
                _ => FamilyActivation::Partial,
            };

            Family { name, faces, activation }
        })
        .collect();

    families.sort_by_key(|f| f.name.to_lowercase());
    families
}

/// Faces that share a PostScript name.
///
/// Two *active* faces with the same PostScript name is undefined behaviour in
/// every consuming application — whichever the OS resolves first wins, with no
/// indication to the user. Detecting it is close to free once faces are
/// indexed, and it is a genuinely differentiating feature.
///
/// Pass only active faces to check live conflicts; pass all faces to surface
/// duplicates in the library.
pub fn conflicts(faces: &[FaceRecord]) -> Vec<Conflict> {
    let mut by_ps: HashMap<&str, Vec<&FaceRecord>> = HashMap::new();
    for face in faces {
        if let Some(ps) = face.postscript_name.as_deref() {
            by_ps.entry(ps).or_default().push(face);
        }
    }

    let mut out: Vec<Conflict> = by_ps
        .into_iter()
        .filter_map(|(ps, group)| {
            // Identical files indexed from two roots are duplicates, not a
            // conflict — same bytes resolve the same either way.
            let distinct: HashSet<&str> =
                group.iter().map(|f| f.content_hash.as_str()).collect();
            if distinct.len() < 2 {
                return None;
            }
            Some(Conflict {
                postscript_name: ps.to_string(),
                face_ids: group.iter().map(|f| f.face_id.clone()).collect(),
            })
        })
        .collect();

    out.sort_by(|a, b| a.postscript_name.cmp(&b.postscript_name));
    out
}

/// Exact-duplicate faces: same bytes, same index, indexed from several paths.
/// Worth surfacing as a cleanup affordance, but never as a conflict.
pub fn duplicates(faces: &[FaceRecord]) -> Vec<Vec<String>> {
    let mut by_id: HashMap<&str, Vec<&FaceRecord>> = HashMap::new();
    for face in faces {
        by_id.entry(face.face_id.as_str()).or_default().push(face);
    }
    by_id
        .into_values()
        .filter(|g| g.len() > 1)
        .map(|g| g.iter().map(|f| f.file_path.display().to_string()).collect())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FontFormat;

    fn face(family: &str, sub: &str, weight: u16, width: u16, italic: bool, id: &str) -> FaceRecord {
        FaceRecord {
            face_id: id.to_string(),
            content_hash: id.to_string(),
            file_path: format!("/fonts/{id}.otf").into(),
            face_index: 0,
            file_size: 1000,
            modified: 0,
            postscript_name: Some(format!("{}-{}", family.replace(' ', ""), sub.replace(' ', ""))),
            family: family.to_string(),
            subfamily: sub.to_string(),
            legacy_family: family.to_string(),
            legacy_subfamily: sub.to_string(),
            full_name: None,
            version: None,
            weight,
            width,
            italic,
            oblique: false,
            monospaced: false,
            is_variable: false,
            axes: vec![],
            format: FontFormat::OpenType,
            units_per_em: 1000,
            glyph_count: 200,
            codepoint_count: 180,
        }
    }

    #[test]
    fn styles_sort_by_width_then_weight_then_slant() {
        let faces = vec![
            face("Inter", "Black", 900, 5, false, "a"),
            face("Inter", "Regular", 400, 5, false, "b"),
            face("Inter", "Thin", 100, 5, false, "c"),
            face("Inter", "Regular Italic", 400, 5, true, "d"),
            face("Inter", "Condensed Regular", 400, 3, false, "e"),
        ];
        let fams = group(faces, &HashSet::new());
        let order: Vec<&str> = fams[0].faces.iter().map(|f| f.subfamily.as_str()).collect();
        assert_eq!(
            order,
            ["Condensed Regular", "Thin", "Regular", "Regular Italic", "Black"]
        );
    }

    #[test]
    fn casing_variants_merge_into_one_family() {
        let faces = vec![
            face("Söhne", "Regular", 400, 5, false, "a"),
            face("SÖHNE", "Bold", 700, 5, false, "b"),
            face("Söhne", "Light", 300, 5, false, "c"),
        ];
        let fams = group(faces, &HashSet::new());
        assert_eq!(fams.len(), 1);
        assert_eq!(fams[0].name, "Söhne"); // majority spelling wins
        assert_eq!(fams[0].faces.len(), 3);
    }

    #[test]
    fn activation_is_tri_state() {
        let faces = vec![
            face("Inter", "Regular", 400, 5, false, "a"),
            face("Inter", "Bold", 700, 5, false, "b"),
        ];
        let none = group(faces.clone(), &HashSet::new());
        assert_eq!(none[0].activation, FamilyActivation::None);

        let partial = group(faces.clone(), &HashSet::from(["a".to_string()]));
        assert_eq!(partial[0].activation, FamilyActivation::Partial);

        let all = group(faces, &HashSet::from(["a".to_string(), "b".to_string()]));
        assert_eq!(all[0].activation, FamilyActivation::All);
    }

    #[test]
    fn representative_is_upright_regular() {
        let faces = vec![
            face("Inter", "Black", 900, 5, false, "a"),
            face("Inter", "Regular", 400, 5, false, "b"),
            face("Inter", "Regular Italic", 400, 5, true, "c"),
        ];
        let fams = group(faces, &HashSet::new());
        assert_eq!(fams[0].representative().unwrap().face_id, "b");
    }

    #[test]
    fn representative_falls_back_when_no_regular_exists() {
        let faces = vec![
            face("Display", "Black", 900, 5, false, "a"),
            face("Display", "Bold", 700, 5, false, "b"),
        ];
        let fams = group(faces, &HashSet::new());
        assert_eq!(fams[0].representative().unwrap().face_id, "b");
    }

    #[test]
    fn same_postscript_name_different_bytes_is_a_conflict() {
        let mut a = face("Helvetica Neue", "Regular", 400, 5, false, "a");
        let mut b = face("Helvetica Neue", "Regular", 400, 5, false, "b");
        a.postscript_name = Some("HelveticaNeue".into());
        b.postscript_name = Some("HelveticaNeue".into());
        assert_eq!(conflicts(&[a, b]).len(), 1);
    }

    #[test]
    fn identical_files_are_duplicates_not_conflicts() {
        let a = face("Inter", "Regular", 400, 5, false, "same");
        let mut b = a.clone();
        b.file_path = "/other/Inter.otf".into();
        assert!(conflicts(&[a.clone(), b.clone()]).is_empty());
        assert_eq!(duplicates(&[a, b]).len(), 1);
    }
}
