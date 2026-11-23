//! Runs the real pipeline over a font directory and reports what it finds.
//!
//!     cargo run -p font-core --example probe --release -- /usr/share/fonts [extract-dir]
//!
//! With an extract dir, every face of every collection is materialised there,
//! so the output can be validated by an independent parser (fontTools).

use font_core::{family, scan, sfnt};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().unwrap_or_else(|| "/usr/share/fonts".into()));
    let extract_dir = args.next().map(PathBuf::from);

    let t = Instant::now();
    let report = scan::scan(std::slice::from_ref(&root), &Default::default());
    let scan_ms = t.elapsed().as_millis();

    println!("root:            {}", root.display());
    println!("files seen:      {}", report.files_seen);
    println!("files parsed:    {}", report.files_parsed);
    println!("faces:           {}", report.faces.len());
    println!("parse failures:  {}", report.failures.len());
    for (p, e) in report.failures.iter().take(5) {
        println!("    {} — {e}", p.display());
    }
    println!("scan time:       {scan_ms} ms");

    let variable = report.faces.iter().filter(|f| f.is_variable).count();
    let collection = report.faces.iter().filter(|f| f.needs_extraction()).count();
    println!("variable faces:  {variable}");
    println!("in collections:  {collection}");

    let t = Instant::now();
    let families = family::group(report.faces.clone(), &HashSet::new());
    println!("families:        {} (grouped in {} µs)", families.len(), t.elapsed().as_micros());

    println!("\nlargest families:");
    let mut by_size: Vec<_> = families.iter().collect();
    by_size.sort_by_key(|f| std::cmp::Reverse(f.faces.len()));
    for fam in by_size.iter().take(6) {
        let styles: Vec<String> = fam
            .faces
            .iter()
            .take(8)
            .map(|f| format!("{}({})", f.subfamily, f.weight))
            .collect();
        println!("  {:<28} {:>3} faces  {}", fam.name, fam.faces.len(), styles.join(", "));
    }

    let conflicts = family::conflicts(&report.faces);
    let dupes = family::duplicates(&report.faces);
    println!("\nPS-name conflicts: {}   exact duplicates: {}", conflicts.len(), dupes.len());
    for c in conflicts.iter().take(5) {
        println!("  {} × {}", c.postscript_name, c.face_ids.len());
    }

    if let Some(dir) = extract_dir {
        std::fs::create_dir_all(&dir)?;
        let t = Instant::now();
        let mut written = 0;
        let mut seen_files = HashSet::new();
        for face in report.faces.iter().filter(|f| f.needs_extraction()) {
            seen_files.insert(face.file_path.clone());
            let data = std::fs::read(&face.file_path)?;
            let out = sfnt::extract_face(&data, face.face_index)?;

            // Round-trip through our own parser: the extracted single-face
            // file must report the same identity as face N of the original.
            let reparsed = ttf_parser::Face::parse(&out, 0)
                .map_err(|e| anyhow::anyhow!("{}#{}: reparse failed: {e:?}", face.file_path.display(), face.face_index))?;
            anyhow::ensure!(
                reparsed.number_of_glyphs() == face.glyph_count,
                "glyph count changed for {}#{}",
                face.file_path.display(),
                face.face_index
            );

            let stem = face.file_path.file_stem().unwrap().to_string_lossy();
            let name = format!("{stem}__{}.{}", face.face_index, if out[0..4] == *b"OTTO" { "otf" } else { "ttf" });
            std::fs::write(dir.join(&name), &out)?;
            written += 1;
        }
        println!(
            "\nextracted {written} faces from {} collections in {} ms → {}",
            seen_files.len(),
            t.elapsed().as_millis(),
            dir.display()
        );
    }

    Ok(())
}
