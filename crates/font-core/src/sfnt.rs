//! Materialising a single face out of a TrueType/OpenType Collection.
//!
//! Every activation mechanism we have — `AddFontResourceExW`,
//! `CTFontManagerRegisterFontsForURL`, a fontconfig symlink — operates on
//! **files**. So does `@font-face`. A `.ttc` holds several faces in one file
//! with shared tables, which means face-level activation and webview preview
//! both fail on collections for the same reason: there is no way to say
//! "face 2 of this file".
//!
//! The fix is one component used by both: given `(bytes, face_index)`, emit a
//! valid standalone SFNT containing just that face's tables.
//!
//! This is not subsetting. No glyph data is touched. We copy the table
//! directory entries the face points at, repack them contiguously, and fix up
//! the two things that are position-dependent: table offsets and the `head`
//! checksum adjustment.

use std::io::Write;

const TAG_TTCF: u32 = u32::from_be_bytes(*b"ttcf");
const TAG_HEAD: u32 = u32::from_be_bytes(*b"head");
const TAG_DSIG: u32 = u32::from_be_bytes(*b"DSIG");

const SFNT_TRUETYPE: u32 = 0x0001_0000;
const SFNT_CFF: u32 = u32::from_be_bytes(*b"OTTO");
const SFNT_TRUE: u32 = u32::from_be_bytes(*b"true");

/// Magic constant the `head` checksum adjustment is derived from (OpenType spec).
const CHECKSUM_MAGIC: u32 = 0xB1B0_AFBA;

#[derive(Debug, thiserror::Error)]
pub enum SfntError {
    #[error("data ends at {len} but {need} bytes were needed at offset {at}")]
    Truncated { at: usize, need: usize, len: usize },
    #[error("unrecognised sfnt version {0:#010x}")]
    BadSfntVersion(u32),
    #[error("face index {index} out of range, collection holds {count}")]
    FaceIndexOutOfRange { index: u32, count: u32 },
    #[error("font declares {0} tables, which is not plausible")]
    AbsurdTableCount(u16),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

type Result<T> = std::result::Result<T, SfntError>;

// ---------------------------------------------------------------------------
// Primitive reads. Bounds-checked, because font files are attacker-controlled
// input as far as this crate is concerned — users drag in fonts from anywhere.
// ---------------------------------------------------------------------------

fn u16_at(data: &[u8], at: usize) -> Result<u16> {
    let end = at.checked_add(2).ok_or(SfntError::Truncated { at, need: 2, len: data.len() })?;
    let slice = data.get(at..end).ok_or(SfntError::Truncated { at, need: 2, len: data.len() })?;
    Ok(u16::from_be_bytes([slice[0], slice[1]]))
}

fn u32_at(data: &[u8], at: usize) -> Result<u32> {
    let end = at.checked_add(4).ok_or(SfntError::Truncated { at, need: 4, len: data.len() })?;
    let slice = data.get(at..end).ok_or(SfntError::Truncated { at, need: 4, len: data.len() })?;
    Ok(u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

/// Number of faces in the file. Returns 1 for a plain single-face font.
pub fn face_count(data: &[u8]) -> Result<u32> {
    if u32_at(data, 0)? == TAG_TTCF {
        let n = u32_at(data, 8)?;
        // A collection with tens of thousands of faces is a malformed or
        // hostile file; refuse rather than allocate against it.
        if n == 0 || n > 4096 {
            return Err(SfntError::FaceIndexOutOfRange { index: 0, count: n });
        }
        Ok(n)
    } else {
        Ok(1)
    }
}

pub fn is_collection(data: &[u8]) -> bool {
    u32_at(data, 0).map(|t| t == TAG_TTCF).unwrap_or(false)
}

/// Byte offset of the table directory for `index`.
fn table_directory_offset(data: &[u8], index: u32) -> Result<usize> {
    if !is_collection(data) {
        return if index == 0 {
            Ok(0)
        } else {
            Err(SfntError::FaceIndexOutOfRange { index, count: 1 })
        };
    }
    let count = face_count(data)?;
    if index >= count {
        return Err(SfntError::FaceIndexOutOfRange { index, count });
    }
    // TTC header: tag(4) version(4) numFonts(4), then u32 offsets.
    let at = 12 + (index as usize) * 4;
    Ok(u32_at(data, at)? as usize)
}

#[derive(Debug, Clone, Copy)]
struct TableRecord {
    tag: u32,
    checksum: u32,
    offset: u32,
    length: u32,
}

fn read_directory(data: &[u8], dir_at: usize) -> Result<(u32, Vec<TableRecord>)> {
    let sfnt_version = u32_at(data, dir_at)?;
    if !matches!(sfnt_version, SFNT_TRUETYPE | SFNT_CFF | SFNT_TRUE) {
        return Err(SfntError::BadSfntVersion(sfnt_version));
    }
    let num_tables = u16_at(data, dir_at + 4)?;
    if num_tables == 0 || num_tables > 512 {
        return Err(SfntError::AbsurdTableCount(num_tables));
    }

    let mut records = Vec::with_capacity(num_tables as usize);
    for i in 0..num_tables as usize {
        let at = dir_at + 12 + i * 16;
        let rec = TableRecord {
            tag: u32_at(data, at)?,
            checksum: u32_at(data, at + 4)?,
            offset: u32_at(data, at + 8)?,
            length: u32_at(data, at + 12)?,
        };
        // Validate before we trust it as a slice range later.
        let end = (rec.offset as usize)
            .checked_add(rec.length as usize)
            .ok_or(SfntError::Truncated { at: rec.offset as usize, need: rec.length as usize, len: data.len() })?;
        if end > data.len() {
            return Err(SfntError::Truncated {
                at: rec.offset as usize,
                need: rec.length as usize,
                len: data.len(),
            });
        }
        records.push(rec);
    }
    Ok((sfnt_version, records))
}

/// OpenType table checksum: sum of big-endian u32 words, zero-padded to a
/// 4-byte boundary, wrapping.
fn checksum(data: &[u8]) -> u32 {
    let mut sum: u32 = 0;
    let mut chunks = data.chunks_exact(4);
    for c in &mut chunks {
        sum = sum.wrapping_add(u32::from_be_bytes([c[0], c[1], c[2], c[3]]));
    }
    let rest = chunks.remainder();
    if !rest.is_empty() {
        let mut last = [0u8; 4];
        last[..rest.len()].copy_from_slice(rest);
        sum = sum.wrapping_add(u32::from_be_bytes(last));
    }
    sum
}

/// `searchRange` / `entrySelector` / `rangeShift` from the offset table.
///
/// Nothing modern reads these, but a wrong value here makes some validators
/// and older rasterisers reject the file outright, so compute them properly.
fn binary_search_params(num_tables: u16) -> (u16, u16, u16) {
    let mut selector = 0u16;
    while (1u32 << (selector + 1)) <= num_tables as u32 {
        selector += 1;
    }
    let search_range = (1u32 << selector) as u16 * 16;
    let range_shift = num_tables * 16 - search_range;
    (search_range, selector, range_shift)
}

/// Extract face `index` from `data` as a standalone SFNT.
///
/// The output is byte-identical in glyph content to the source; only the
/// table directory and `head.checkSumAdjustment` differ.
pub fn extract_face(data: &[u8], index: u32) -> Result<Vec<u8>> {
    let dir_at = table_directory_offset(data, index)?;
    let (sfnt_version, mut records) = read_directory(data, dir_at)?;

    // A digital signature covers the whole original file. Once we repack, it
    // is meaningless — and on Windows an invalid DSIG is worse than none.
    records.retain(|r| r.tag != TAG_DSIG);

    // The spec requires table records in ascending tag order.
    records.sort_unstable_by_key(|r| r.tag);

    let num_tables = records.len() as u16;
    let header_len = 12 + records.len() * 16;

    // Lay out table data 4-byte aligned, in directory order.
    let mut out = Vec::with_capacity(data.len().min(8 * 1024 * 1024));
    out.resize(header_len, 0);

    let mut new_records = Vec::with_capacity(records.len());
    let mut head_data_offset: Option<usize> = None;

    for rec in &records {
        while out.len() % 4 != 0 {
            out.push(0);
        }
        let new_offset = out.len();
        let start = rec.offset as usize;
        let end = start + rec.length as usize;
        out.write_all(&data[start..end])?;

        if rec.tag == TAG_HEAD {
            head_data_offset = Some(new_offset);
        }
        new_records.push(TableRecord {
            tag: rec.tag,
            checksum: rec.checksum,
            offset: new_offset as u32,
            length: rec.length,
        });
    }
    while out.len() % 4 != 0 {
        out.push(0);
    }

    // `head.checkSumAdjustment` lives at byte 8 of the table and must be zero
    // while checksums are computed. Zero it before we checksum `head` itself.
    if let Some(head_at) = head_data_offset {
        let field = head_at + 8;
        if field + 4 > out.len() {
            return Err(SfntError::Truncated { at: field, need: 4, len: out.len() });
        }
        out[field..field + 4].copy_from_slice(&[0, 0, 0, 0]);

        // Recompute head's stored checksum, since we just changed its bytes.
        let head_rec = new_records
            .iter_mut()
            .find(|r| r.tag == TAG_HEAD)
            .expect("head record present when head_data_offset is Some");
        let start = head_rec.offset as usize;
        let end = start + head_rec.length as usize;
        head_rec.checksum = checksum(&out[start..end]);
    }

    // Write the offset table.
    let (search_range, entry_selector, range_shift) = binary_search_params(num_tables);
    out[0..4].copy_from_slice(&sfnt_version.to_be_bytes());
    out[4..6].copy_from_slice(&num_tables.to_be_bytes());
    out[6..8].copy_from_slice(&search_range.to_be_bytes());
    out[8..10].copy_from_slice(&entry_selector.to_be_bytes());
    out[10..12].copy_from_slice(&range_shift.to_be_bytes());

    for (i, rec) in new_records.iter().enumerate() {
        let at = 12 + i * 16;
        out[at..at + 4].copy_from_slice(&rec.tag.to_be_bytes());
        out[at + 4..at + 8].copy_from_slice(&rec.checksum.to_be_bytes());
        out[at + 8..at + 12].copy_from_slice(&rec.offset.to_be_bytes());
        out[at + 12..at + 16].copy_from_slice(&rec.length.to_be_bytes());
    }

    // Finally: adjustment = MAGIC - checksum(entire file with adjustment = 0).
    if let Some(head_at) = head_data_offset {
        let file_sum = checksum(&out);
        let adjustment = CHECKSUM_MAGIC.wrapping_sub(file_sum);
        let field = head_at + 8;
        out[field..field + 4].copy_from_slice(&adjustment.to_be_bytes());
    }

    Ok(out)
}

/// Extract only if needed. Single-face files pass through untouched — there is
/// no reason to rewrite an `.otf` just to activate it, and rewriting would
/// break font vendors' own signatures.
pub fn materialize(data: &[u8], index: u32) -> Result<Option<Vec<u8>>> {
    if is_collection(data) {
        Ok(Some(extract_face(data, index)?))
    } else if index == 0 {
        Ok(None)
    } else {
        Err(SfntError::FaceIndexOutOfRange { index, count: 1 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal two-face collection sharing one table, so we can prove
    /// extraction picks the right subset and repacks it validly.
    fn synthetic_ttc() -> Vec<u8> {
        // Table payloads.
        let head: Vec<u8> = {
            let mut t = vec![0u8; 54];
            t[0..4].copy_from_slice(&0x0001_0000u32.to_be_bytes()); // version
            t[8..12].copy_from_slice(&0xDEAD_BEEFu32.to_be_bytes()); // bogus adjustment
            t
        };
        let cmap = vec![0xAAu8; 20];
        let glyf_a = vec![0x11u8; 13]; // deliberately unaligned length
        let glyf_b = vec![0x22u8; 8];

        let mut body = Vec::new();
        let push = |body: &mut Vec<u8>, d: &[u8]| -> (u32, u32) {
            while body.len() % 4 != 0 {
                body.push(0);
            }
            let off = body.len() as u32;
            body.extend_from_slice(d);
            (off, d.len() as u32)
        };

        // Reserve space for headers; we build body separately then splice.
        let num_faces = 2usize;
        let ttc_header_len = 12 + num_faces * 4;
        let dir_len = 12 + 3 * 16; // 3 tables per face
        let base = ttc_header_len + dir_len * 2;
        body.resize(base, 0);

        let (head_off, head_len) = push(&mut body, &head);
        let (cmap_off, cmap_len) = push(&mut body, &cmap);
        let (ga_off, ga_len) = push(&mut body, &glyf_a);
        let (gb_off, gb_len) = push(&mut body, &glyf_b);

        let write_dir = |buf: &mut Vec<u8>, at: usize, tables: &[(u32, u32, u32)]| {
            buf[at..at + 4].copy_from_slice(&0x0001_0000u32.to_be_bytes());
            buf[at + 4..at + 6].copy_from_slice(&(tables.len() as u16).to_be_bytes());
            buf[at + 6..at + 12].copy_from_slice(&[0, 0, 0, 0, 0, 0]);
            for (i, (tag, off, len)) in tables.iter().enumerate() {
                let r = at + 12 + i * 16;
                buf[r..r + 4].copy_from_slice(&tag.to_be_bytes());
                buf[r + 4..r + 8].copy_from_slice(&0u32.to_be_bytes());
                buf[r + 8..r + 12].copy_from_slice(&off.to_be_bytes());
                buf[r + 12..r + 16].copy_from_slice(&len.to_be_bytes());
            }
        };

        let dir0 = ttc_header_len;
        let dir1 = ttc_header_len + dir_len;
        let t = |s: &[u8; 4]| u32::from_be_bytes(*s);
        // Both faces share head + cmap; each has its own glyf.
        write_dir(&mut body, dir0, &[
            (t(b"cmap"), cmap_off, cmap_len),
            (t(b"glyf"), ga_off, ga_len),
            (TAG_HEAD, head_off, head_len),
        ]);
        write_dir(&mut body, dir1, &[
            (t(b"cmap"), cmap_off, cmap_len),
            (t(b"glyf"), gb_off, gb_len),
            (TAG_HEAD, head_off, head_len),
        ]);

        body[0..4].copy_from_slice(&TAG_TTCF.to_be_bytes());
        body[4..8].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        body[8..12].copy_from_slice(&(num_faces as u32).to_be_bytes());
        body[12..16].copy_from_slice(&(dir0 as u32).to_be_bytes());
        body[16..20].copy_from_slice(&(dir1 as u32).to_be_bytes());
        body
    }

    #[test]
    fn counts_faces() {
        assert_eq!(face_count(&synthetic_ttc()).unwrap(), 2);
    }

    #[test]
    fn extracts_the_requested_face_not_the_first() {
        let ttc = synthetic_ttc();
        let out = extract_face(&ttc, 1).unwrap();
        assert!(!is_collection(&out));
        let (_, recs) = read_directory(&out, 0).unwrap();
        let glyf = recs.iter().find(|r| r.tag == u32::from_be_bytes(*b"glyf")).unwrap();
        let start = glyf.offset as usize;
        // Face 1 owns the 0x22-filled glyf, face 0 the 0x11-filled one.
        assert_eq!(out[start], 0x22);
        assert_eq!(glyf.length, 8);
    }

    #[test]
    fn tables_are_sorted_and_aligned() {
        let out = extract_face(&synthetic_ttc(), 0).unwrap();
        let (_, recs) = read_directory(&out, 0).unwrap();
        assert!(recs.windows(2).all(|w| w[0].tag < w[1].tag));
        assert!(recs.iter().all(|r| r.offset % 4 == 0));
    }

    #[test]
    fn checksum_adjustment_validates() {
        let out = extract_face(&synthetic_ttc(), 0).unwrap();
        let (_, recs) = read_directory(&out, 0).unwrap();
        let head = recs.iter().find(|r| r.tag == TAG_HEAD).unwrap();
        let field = head.offset as usize + 8;

        let stored = u32::from_be_bytes(out[field..field + 4].try_into().unwrap());
        let mut zeroed = out.clone();
        zeroed[field..field + 4].copy_from_slice(&[0, 0, 0, 0]);
        assert_eq!(stored, CHECKSUM_MAGIC.wrapping_sub(checksum(&zeroed)));
    }

    #[test]
    fn per_table_checksums_are_correct() {
        let out = extract_face(&synthetic_ttc(), 0).unwrap();
        let (_, recs) = read_directory(&out, 0).unwrap();
        let head = recs.iter().find(|r| r.tag == TAG_HEAD).unwrap();
        let s = head.offset as usize;
        let e = s + head.length as usize;
        let mut body = out[s..e].to_vec();
        body[8..12].copy_from_slice(&[0, 0, 0, 0]);
        assert_eq!(head.checksum, checksum(&body));
    }

    #[test]
    fn rejects_out_of_range_face() {
        assert!(extract_face(&synthetic_ttc(), 7).is_err());
    }

    #[test]
    fn rejects_truncated_input() {
        let ttc = synthetic_ttc();
        assert!(extract_face(&ttc[..ttc.len() / 2], 0).is_err());
    }

    #[test]
    fn single_face_passes_through() {
        let out = extract_face(&synthetic_ttc(), 0).unwrap();
        assert!(materialize(&out, 0).unwrap().is_none());
    }

    #[test]
    fn binary_search_params_match_spec() {
        assert_eq!(binary_search_params(9), (128, 3, 16));
        assert_eq!(binary_search_params(16), (256, 4, 0));
        assert_eq!(binary_search_params(1), (16, 0, 0));
    }
}
