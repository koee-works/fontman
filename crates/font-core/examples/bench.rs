//! Times each stage the app runs, against a real library on disk.
//!
//!     cargo run -p font-core --example bench --release -- <library-dir> [subfolder]
//!
//! Uses a throwaway index file so the numbers include real SQLite I/O.

use font_core::db::Index;
use font_core::{family, scan};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

fn ms(t: Instant) -> String {
    format!("{:>8.1} ms", t.elapsed().as_secs_f64() * 1000.0)
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().expect("library dir"));
    let sub = args.next().map(PathBuf::from);

    let dir = tempfile::tempdir()?;
    let mut idx = Index::open(&dir.path().join("index.sqlite"))?;
    idx.add_root(&root)?;
    let roots = vec![root.clone()];

    let t = Instant::now();
    let report = scan::scan(&roots, &Default::default());
    println!("cold scan            {}  ({} files, {} faces)", ms(t), report.files_seen, report.faces.len());

    let t = Instant::now();
    idx.upsert_faces(&report.faces)?;
    println!("index write          {}", ms(t));

    let t = Instant::now();
    let known = idx.known_stats()?;
    let again = scan::scan(&roots, &known);
    println!("incremental scan     {}  ({} skipped, {} re-read)", ms(t), again.files_skipped, again.files_parsed);

    let t = Instant::now();
    let removed = idx.remove_missing(None)?;
    println!("remove missing       {}  ({removed} removed)", ms(t));

    if let Some(sub) = sub {
        let t = Instant::now();
        let r = scan::scan(std::slice::from_ref(&sub), &Default::default());
        println!("full scan of subfolder {}  ({} files)", ms(t), r.files_seen);
    }

    let t = Instant::now();
    let faces = idx.all_faces()?;
    let load = t.elapsed();
    let t2 = Instant::now();
    let fams = family::group(faces.clone(), &HashSet::new());
    let group = t2.elapsed();
    let t3 = Instant::now();
    let json = serde_json::to_vec(&fams)?;
    println!(
        "list families        {}  (load {:.0} ms, group {:.0} ms, json {:.0} ms; {} families, {:.1} MB)",
        ms(t),
        load.as_secs_f64() * 1e3,
        group.as_secs_f64() * 1e3,
        t3.elapsed().as_secs_f64() * 1e3,
        fams.len(),
        json.len() as f64 / 1e6
    );

    let t = Instant::now();
    let c = family::conflicts(&idx.conflict_candidates()?);
    println!("conflicts            {}  ({} found)", ms(t), c.len());
    println!("duplicate files      {}", idx.duplicate_file_count()?);

    let t = Instant::now();
    let n = idx.face_count()?;
    let _ = idx.active_face_ids()?;
    println!("status               {}  ({n} faces)", ms(t));
    Ok(())
}
