//! The index.
//!
//! SQLite is a **cache of the filesystem**, never the source of truth. Files
//! move, get deleted, and get installed behind our back. Every query here is
//! written on the assumption that its answer may already be stale, and startup
//! reconciles rather than trusts.

use crate::model::{FaceRecord, FontFormat, VariationAxis};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

pub type Result<T> = rusqlite::Result<T>;

/// Re-exported so the app shell needs no direct `rusqlite` dependency.
pub use rusqlite::Error as DbError;

const SCHEMA: &str = r#"
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS roots (
    path        TEXT PRIMARY KEY,
    added_at    INTEGER NOT NULL
);

-- Every font file seen, keyed by path. Separate from faces because identical
-- files collapse to one face row (face ids are content-addressed): without
-- this table, every duplicate copy looks new and is re-hashed on every scan.
CREATE TABLE IF NOT EXISTS files (
    path          TEXT PRIMARY KEY,
    content_hash  TEXT NOT NULL,
    file_size     INTEGER NOT NULL,
    modified      INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_files_hash ON files(content_hash);

CREATE TABLE IF NOT EXISTS faces (
    face_id          TEXT PRIMARY KEY,
    content_hash     TEXT NOT NULL,
    file_path        TEXT NOT NULL,
    face_index       INTEGER NOT NULL,
    file_size        INTEGER NOT NULL,
    modified         INTEGER NOT NULL,

    postscript_name  TEXT,
    family           TEXT NOT NULL,
    subfamily        TEXT NOT NULL,
    legacy_family    TEXT NOT NULL,
    legacy_subfamily TEXT NOT NULL,
    full_name        TEXT,
    version          TEXT,

    weight           INTEGER NOT NULL,
    width            INTEGER NOT NULL,
    italic           INTEGER NOT NULL,
    oblique          INTEGER NOT NULL,
    monospaced       INTEGER NOT NULL,

    is_variable      INTEGER NOT NULL,
    axes_json        TEXT NOT NULL DEFAULT '[]',

    format           TEXT NOT NULL,
    units_per_em     INTEGER NOT NULL,
    glyph_count      INTEGER NOT NULL,
    codepoint_count  INTEGER NOT NULL
);

-- Grouping and conflict detection are the two hot queries.
CREATE INDEX IF NOT EXISTS idx_faces_family ON faces(family COLLATE NOCASE);
CREATE INDEX IF NOT EXISTS idx_faces_ps ON faces(postscript_name);
CREATE INDEX IF NOT EXISTS idx_faces_path ON faces(file_path);

-- Intent, at face level. The OS records mechanism at file level; the two are
-- reconciled at startup, never assumed to agree.
CREATE TABLE IF NOT EXISTS activation (
    face_id           TEXT PRIMARY KEY REFERENCES faces(face_id) ON DELETE CASCADE,
    activated_at      INTEGER NOT NULL,
    registered_path   TEXT NOT NULL,
    materialized      INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS collections (
    id        INTEGER PRIMARY KEY,
    name      TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL
);

-- Replaced by collection_members; never held data.
DROP TABLE IF EXISTS collection_faces;

-- Membership is by face id with no foreign key to faces, on purpose: a face
-- leaves the index while its drive is unplugged or its folder is briefly
-- removed, and should be back in its collections when it returns. Queries
-- join against faces, so absent members are simply not listed.
CREATE TABLE IF NOT EXISTS collection_members (
    collection_id INTEGER NOT NULL REFERENCES collections(id) ON DELETE CASCADE,
    face_id       TEXT NOT NULL,
    added_at      INTEGER NOT NULL,
    PRIMARY KEY (collection_id, face_id)
);

-- Tags describe typefaces, so they belong to families, not styles. Keyed by
-- the family's grouping key (its lowercased name), which survives rescans,
-- moves and updated files alike.
CREATE TABLE IF NOT EXISTS tags (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL UNIQUE,
    created_at  INTEGER NOT NULL,
    -- Tags imported from Google Fonts: source 'google', the group Google
    -- lists them under ("Sans", "Expressive"), and Google's own path for
    -- matching on re-import. Your own tags are source 'user'.
    grp         TEXT,
    source      TEXT NOT NULL DEFAULT 'user',
    google_path TEXT
);
CREATE TABLE IF NOT EXISTS family_tags (
    tag_id     INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    family     TEXT NOT NULL,
    -- Who put the tag on the family: an import replaces only its own links.
    source     TEXT NOT NULL DEFAULT 'user',
    PRIMARY KEY (tag_id, family)
);
-- Google tags you took off a family, so re-importing doesn't put them back.
CREATE TABLE IF NOT EXISTS tag_dismissals (
    tag_id     INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    family     TEXT NOT NULL,
    PRIMARY KEY (tag_id, family)
);

-- Style viewer pairings. Settings are the viewer's own JSON, opaque here.
CREATE TABLE IF NOT EXISTS pairings (
    id         INTEGER PRIMARY KEY,
    name       TEXT NOT NULL,
    settings   TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
"#;

/// A named group of faces. Only faces currently in the index are listed.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: i64,
    pub name: String,
    pub face_ids: Vec<String>,
}

/// A saved style viewer pairing.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pairing {
    pub id: i64,
    pub name: String,
    /// The viewer's settings as JSON.
    pub settings: String,
    pub updated_at: i64,
}

/// A tag and the families it's on, by grouping key (lowercased name).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub families: Vec<String>,
    /// Google's group for an imported tag, such as "Sans".
    pub group: Option<String>,
    /// "user" or "google".
    pub source: String,
}

/// One Google tag to apply on import: its path, display parts and the
/// library families (grouping keys) that qualify.
#[derive(Debug, Clone)]
pub struct GoogleTagging {
    pub path: String,
    pub group: String,
    pub name: String,
    pub families: Vec<String>,
}

/// What an import left in the library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleTagCounts {
    pub tags: usize,
    pub families: usize,
}

/// Files with identical bytes, and the faces they hold.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub content_hash: String,
    pub file_size: u64,
    /// "Family Style" for each face in the file, in face order.
    pub faces: Vec<String>,
    pub copies: Vec<DuplicateCopy>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateCopy {
    pub path: PathBuf,
    pub modified: i64,
    /// The copy the index previews and activates from.
    pub in_use: bool,
}

/// Why a name was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NameError {
    #[error("Give it a name.")]
    Empty,
    #[error("“{0}” is already taken. Choose another name.")]
    Taken(String),
    #[error("Names can be up to {MAX_NAME} characters.")]
    TooLong,
    #[error("It no longer exists.")]
    Missing,
}

pub const MAX_NAME: usize = 80;

/// Trim and check a user-supplied name.
fn clean_name(name: &str) -> std::result::Result<String, NameError> {
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        Err(NameError::Empty)
    } else if name.chars().count() > MAX_NAME {
        Err(NameError::TooLong)
    } else {
        Ok(name)
    }
}

pub struct Index {
    conn: Connection,
}

impl Index {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA)?;
        migrate(&conn)?;
        Ok(Self { conn })
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Self { conn })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    // -- roots -------------------------------------------------------------

    pub fn add_root(&self, path: &Path) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO roots (path, added_at) VALUES (?1, ?2)",
            params![path.to_string_lossy(), now()],
        )?;
        Ok(())
    }

    /// Stop indexing a folder: forget its files and the faces only it held.
    /// Files on disk are untouched. Refuses while any face under it is active,
    /// returning how many, since removing them would strand registrations
    /// the app could no longer undo.
    pub fn remove_root(&mut self, root: &Path) -> Result<std::result::Result<usize, usize>> {
        let under = |p: &str| Path::new(p).starts_with(root);
        let active_here = {
            let mut stmt = self.conn.prepare(
                "SELECT f.file_path FROM activation a JOIN faces f USING (face_id)",
            )?;
            let paths: Vec<String> = stmt.query_map([], |r| r.get(0))?.collect::<Result<_>>()?;
            paths.iter().filter(|p| under(p)).count()
        };
        if active_here > 0 {
            return Ok(Err(active_here));
        }
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM roots WHERE path = ?1", params![root.to_string_lossy()])?;
        let files: Vec<String> = {
            let mut stmt = tx.prepare("SELECT path FROM files")?;
            let v: Vec<String> = stmt.query_map([], |r| r.get(0))?.collect::<Result<_>>()?;
            v.into_iter().filter(|p| under(p)).collect()
        };
        {
            let mut del = tx.prepare("DELETE FROM files WHERE path = ?1")?;
            for p in &files {
                del.execute(params![p])?;
            }
        }
        // Faces with a copy in another folder survive, repointed there.
        let removed = repair(&tx)?;
        tx.commit()?;
        Ok(Ok(removed))
    }

    pub fn roots(&self) -> Result<Vec<PathBuf>> {
        let mut stmt = self.conn.prepare("SELECT path FROM roots ORDER BY path")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.map(|r| r.map(PathBuf::from)).collect()
    }

    // -- faces -------------------------------------------------------------

    /// Insert or replace a batch of faces in one transaction.
    ///
    /// A scan of 20k faces as 20k autocommitted statements takes minutes; as
    /// one transaction it takes under a second.
    pub fn upsert_faces(&mut self, faces: &[FaceRecord]) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO faces (
                    face_id, content_hash, file_path, face_index, file_size, modified,
                    postscript_name, family, subfamily, legacy_family, legacy_subfamily,
                    full_name, version, weight, width, italic, oblique, monospaced,
                    is_variable, axes_json, format, units_per_em, glyph_count, codepoint_count
                 ) VALUES (
                    ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                    ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24
                 )
                 -- Never INSERT OR REPLACE: REPLACE deletes the row first,
                 -- which cascades and silently drops the activation ledger.
                 ON CONFLICT(face_id) DO UPDATE SET
                    content_hash = excluded.content_hash,
                    file_path = excluded.file_path,
                    face_index = excluded.face_index,
                    file_size = excluded.file_size,
                    modified = excluded.modified,
                    postscript_name = excluded.postscript_name,
                    family = excluded.family,
                    subfamily = excluded.subfamily,
                    legacy_family = excluded.legacy_family,
                    legacy_subfamily = excluded.legacy_subfamily,
                    full_name = excluded.full_name,
                    version = excluded.version,
                    weight = excluded.weight,
                    width = excluded.width,
                    italic = excluded.italic,
                    oblique = excluded.oblique,
                    monospaced = excluded.monospaced,
                    is_variable = excluded.is_variable,
                    axes_json = excluded.axes_json,
                    format = excluded.format,
                    units_per_em = excluded.units_per_em,
                    glyph_count = excluded.glyph_count,
                    codepoint_count = excluded.codepoint_count",
            )?;

            for f in faces {
                stmt.execute(params![
                    f.face_id,
                    f.content_hash,
                    f.file_path.to_string_lossy(),
                    f.face_index,
                    f.file_size,
                    f.modified,
                    f.postscript_name,
                    f.family,
                    f.subfamily,
                    f.legacy_family,
                    f.legacy_subfamily,
                    f.full_name,
                    f.version,
                    f.weight,
                    f.width,
                    f.italic,
                    f.oblique,
                    f.monospaced,
                    f.is_variable,
                    serde_json::to_string(&f.axes).unwrap_or_else(|_| "[]".into()),
                    format_name(f.format),
                    f.units_per_em,
                    f.glyph_count,
                    f.codepoint_count,
                ])?;
            }

            // Record every file scanned, including duplicate copies whose
            // faces collapsed into rows that point at another path.
            let mut file = tx.prepare(
                "INSERT INTO files (path, content_hash, file_size, modified)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(path) DO UPDATE SET
                    content_hash = excluded.content_hash,
                    file_size = excluded.file_size,
                    modified = excluded.modified",
            )?;
            for f in faces {
                file.execute(params![
                    f.file_path.to_string_lossy(),
                    f.content_hash,
                    f.file_size,
                    f.modified
                ])?;
            }
        }
        // A file that changed on disk now has a new hash; its old identity
        // either survives in another copy or is gone.
        repair(&tx)?;
        tx.commit()
    }

    /// Size and mtime of every known file, for an incremental scan.
    pub fn known_stats(&self) -> Result<crate::scan::KnownStats> {
        let mut stmt = self.conn.prepare("SELECT path, file_size, modified FROM files")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                PathBuf::from(r.get::<_, String>(0)?),
                (r.get::<_, i64>(1)? as u64, r.get::<_, i64>(2)?),
            ))
        })?;
        rows.collect()
    }

    /// Forget files that no longer exist, optionally only under `scope`, and
    /// repair the faces that pointed at them. Returns how many faces were
    /// removed outright; faces with a surviving copy elsewhere are repointed.
    /// Active faces are never removed: their row is the only record of a
    /// registration that still has to be undone.
    pub fn remove_missing(&mut self, scope: Option<&Path>) -> Result<usize> {
        let tx = self.conn.transaction()?;
        let gone: Vec<String> = {
            let mut stmt = tx.prepare("SELECT path FROM files")?;
            let paths: Vec<String> = stmt.query_map([], |r| r.get(0))?.collect::<Result<_>>()?;
            paths
                .into_iter()
                .filter(|p| {
                    let p = Path::new(p);
                    scope.is_none_or(|s| p.starts_with(s)) && !p.exists()
                })
                .collect()
        };
        {
            let mut del = tx.prepare("DELETE FROM files WHERE path = ?1")?;
            for p in &gone {
                del.execute(params![p])?;
            }
        }
        let removed = repair(&tx)?;
        tx.commit()?;
        Ok(removed)
    }

    /// Faces whose PostScript name is shared with a face of different bytes.
    /// Indexed, so it costs a few milliseconds instead of loading the library.
    pub fn conflict_candidates(&self) -> Result<Vec<FaceRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT * FROM faces WHERE postscript_name IN (
                SELECT postscript_name FROM faces
                WHERE postscript_name IS NOT NULL
                GROUP BY postscript_name
                HAVING COUNT(DISTINCT content_hash) > 1
             )",
        )?;
        let rows = stmt.query_map([], row_to_face)?;
        rows.collect()
    }

    /// Files whose bytes also exist at another path.
    pub fn duplicate_file_count(&self) -> Result<usize> {
        self.conn
            .query_row(
                "SELECT COALESCE(SUM(n - 1), 0) FROM
                    (SELECT COUNT(*) AS n FROM files GROUP BY content_hash HAVING n > 1)",
                [],
                |r| r.get::<_, i64>(0),
            )
            .map(|n| n as usize)
    }

    /// Every family in the index by grouping key (lowercased name).
    pub fn family_keys(&self) -> Result<std::collections::HashSet<String>> {
        let mut stmt = self.conn.prepare("SELECT DISTINCT family FROM faces")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.map(|r| r.map(|f| crate::family::group_key(&f))).collect()
    }

    /// Font files known, counting duplicate copies separately.
    pub fn file_count(&self) -> Result<usize> {
        self.conn
            .query_row("SELECT COUNT(*) FROM files", [], |r| r.get::<_, i64>(0))
            .map(|n| n as usize)
    }

    pub fn face_count(&self) -> Result<usize> {
        self.conn
            .query_row("SELECT COUNT(*) FROM faces", [], |r| r.get::<_, i64>(0))
            .map(|n| n as usize)
    }

    pub fn all_faces(&self) -> Result<Vec<FaceRecord>> {
        let mut stmt = self.conn.prepare("SELECT * FROM faces")?;
        let rows = stmt.query_map([], row_to_face)?;
        rows.collect()
    }

    pub fn face(&self, face_id: &str) -> Result<Option<FaceRecord>> {
        self.conn
            .query_row("SELECT * FROM faces WHERE face_id = ?1", params![face_id], row_to_face)
            .optional()
    }

    // -- activation ledger -------------------------------------------------

    pub fn record_activation(
        &self,
        face_id: &str,
        registered_path: &Path,
        materialized: bool,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO activation (face_id, activated_at, registered_path, materialized)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(face_id) DO UPDATE SET
                registered_path = excluded.registered_path,
                materialized    = excluded.materialized",
            params![face_id, now(), registered_path.to_string_lossy(), materialized],
        )?;
        Ok(())
    }

    pub fn clear_activation(&self, face_id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM activation WHERE face_id = ?1", params![face_id])?;
        Ok(())
    }

    pub fn active_face_ids(&self) -> Result<std::collections::HashSet<String>> {
        let mut stmt = self.conn.prepare("SELECT face_id FROM activation")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.collect()
    }

    pub fn active_handles(&self) -> Result<Vec<crate::activate::ActivationHandle>> {
        let mut stmt = self
            .conn
            .prepare("SELECT face_id, registered_path, materialized FROM activation")?;
        let rows = stmt.query_map([], |r| {
            Ok(crate::activate::ActivationHandle {
                face_id: r.get(0)?,
                registered_path: PathBuf::from(r.get::<_, String>(1)?),
                materialized: r.get(2)?,
            })
        })?;
        rows.collect()
    }
}

// -- collections and pairings ----------------------------------------------

/// Names are unique ignoring case, so "Display" and "display" can't coexist.
fn name_taken(conn: &Connection, table: &str, name: &str, except: Option<i64>) -> Result<bool> {
    conn.query_row(
        &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE name = ?1 COLLATE NOCASE AND id IS NOT ?2)"),
        params![name, except],
        |r| r.get(0),
    )
}

impl Index {
    pub fn collections(&self) -> Result<Vec<Collection>> {
        let mut out: Vec<Collection> = {
            let mut stmt = self
                .conn
                .prepare("SELECT id, name FROM collections ORDER BY name COLLATE NOCASE")?;
            let rows = stmt.query_map([], |r| {
                Ok(Collection { id: r.get(0)?, name: r.get(1)?, face_ids: Vec::new() })
            })?;
            rows.collect::<Result<_>>()?
        };
        let mut stmt = self.conn.prepare(
            "SELECT m.collection_id, m.face_id FROM collection_members m
             JOIN faces f ON f.face_id = m.face_id
             ORDER BY m.added_at, m.face_id",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
        let by_id: std::collections::HashMap<i64, usize> =
            out.iter().enumerate().map(|(i, c)| (c.id, i)).collect();
        for row in rows {
            let (cid, face) = row?;
            if let Some(&i) = by_id.get(&cid) {
                out[i].face_ids.push(face);
            }
        }
        Ok(out)
    }

    fn collection(&self, id: i64) -> Result<Option<Collection>> {
        Ok(self.collections()?.into_iter().find(|c| c.id == id))
    }

    pub fn create_collection(
        &mut self,
        name: &str,
        face_ids: &[String],
    ) -> Result<std::result::Result<Collection, NameError>> {
        let name = match clean_name(name) {
            Ok(n) => n,
            Err(e) => return Ok(Err(e)),
        };
        if name_taken(&self.conn, "collections", &name, None)? {
            return Ok(Err(NameError::Taken(name)));
        }
        self.conn.execute(
            "INSERT INTO collections (name, created_at) VALUES (?1, ?2)",
            params![name, now()],
        )?;
        let id = self.conn.last_insert_rowid();
        self.set_collection_members(id, face_ids, true)?;
        Ok(self.collection(id)?.ok_or(NameError::Missing))
    }

    pub fn rename_collection(
        &self,
        id: i64,
        name: &str,
    ) -> Result<std::result::Result<(), NameError>> {
        let name = match clean_name(name) {
            Ok(n) => n,
            Err(e) => return Ok(Err(e)),
        };
        if name_taken(&self.conn, "collections", &name, Some(id))? {
            return Ok(Err(NameError::Taken(name)));
        }
        let n = self
            .conn
            .execute("UPDATE collections SET name = ?1 WHERE id = ?2", params![name, id])?;
        Ok(if n == 0 { Err(NameError::Missing) } else { Ok(()) })
    }

    /// Deletes the collection, not its fonts.
    pub fn delete_collection(&self, id: i64) -> Result<bool> {
        Ok(self.conn.execute("DELETE FROM collections WHERE id = ?1", params![id])? > 0)
    }

    /// Add faces to a collection, or take them out. Returns false if the
    /// collection doesn't exist. Adding a face already there is a no-op.
    pub fn set_collection_members(
        &mut self,
        id: i64,
        face_ids: &[String],
        member: bool,
    ) -> Result<bool> {
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM collections WHERE id = ?1)",
            params![id],
            |r| r.get(0),
        )?;
        if !exists {
            return Ok(false);
        }
        let tx = self.conn.transaction()?;
        if member {
            let mut stmt = tx.prepare(
                "INSERT OR IGNORE INTO collection_members (collection_id, face_id, added_at)
                 VALUES (?1, ?2, ?3)",
            )?;
            let t = now();
            for f in face_ids {
                stmt.execute(params![id, f, t])?;
            }
        } else {
            let mut stmt = tx.prepare(
                "DELETE FROM collection_members WHERE collection_id = ?1 AND face_id = ?2",
            )?;
            for f in face_ids {
                stmt.execute(params![id, f])?;
            }
        }
        tx.commit()?;
        Ok(true)
    }

    /// Most recently changed first.
    pub fn pairings(&self) -> Result<Vec<Pairing>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, settings, updated_at FROM pairings ORDER BY updated_at DESC, id DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(Pairing { id: r.get(0)?, name: r.get(1)?, settings: r.get(2)?, updated_at: r.get(3)? })
        })?;
        rows.collect()
    }

    /// Create a pairing (`id` None) or overwrite one.
    pub fn save_pairing(
        &self,
        id: Option<i64>,
        name: &str,
        settings: &str,
    ) -> Result<std::result::Result<Pairing, NameError>> {
        let name = match clean_name(name) {
            Ok(n) => n,
            Err(e) => return Ok(Err(e)),
        };
        if name_taken(&self.conn, "pairings", &name, id)? {
            return Ok(Err(NameError::Taken(name)));
        }
        let t = now();
        let id = match id {
            Some(id) => {
                let n = self.conn.execute(
                    "UPDATE pairings SET name = ?1, settings = ?2, updated_at = ?3 WHERE id = ?4",
                    params![name, settings, t, id],
                )?;
                if n == 0 {
                    return Ok(Err(NameError::Missing));
                }
                id
            }
            None => {
                self.conn.execute(
                    "INSERT INTO pairings (name, settings, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?3)",
                    params![name, settings, t],
                )?;
                self.conn.last_insert_rowid()
            }
        };
        Ok(Ok(Pairing { id, name, settings: settings.to_owned(), updated_at: t }))
    }

    pub fn delete_pairing(&self, id: i64) -> Result<bool> {
        Ok(self.conn.execute("DELETE FROM pairings WHERE id = ?1", params![id])? > 0)
    }

    // -- tags ------------------------------------------------------------------

    /// Every tag, by name, with the families it's on. Families that aren't in
    /// the library right now are kept and listed; the frontend matches them
    /// against what's there.
    pub fn tags(&self) -> Result<Vec<Tag>> {
        let mut out: Vec<Tag> = {
            let mut stmt = self.conn.prepare(
                "SELECT id, name, grp, source FROM tags ORDER BY name COLLATE NOCASE",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok(Tag {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    families: Vec::new(),
                    group: r.get(2)?,
                    source: r.get(3)?,
                })
            })?;
            rows.collect::<Result<_>>()?
        };
        let by_id: std::collections::HashMap<i64, usize> =
            out.iter().enumerate().map(|(i, t)| (t.id, i)).collect();
        let mut stmt = self.conn.prepare("SELECT tag_id, family FROM family_tags ORDER BY family")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
        for row in rows {
            let (tid, fam) = row?;
            if let Some(&i) = by_id.get(&tid) {
                out[i].families.push(fam);
            }
        }
        Ok(out)
    }

    pub fn create_tag(&mut self, name: &str, families: &[String]) -> Result<std::result::Result<Tag, NameError>> {
        let name = match clean_name(name) {
            Ok(n) => n,
            Err(e) => return Ok(Err(e)),
        };
        if name_taken(&self.conn, "tags", &name, None)? {
            return Ok(Err(NameError::Taken(name)));
        }
        self.conn
            .execute("INSERT INTO tags (name, created_at) VALUES (?1, ?2)", params![name, now()])?;
        let id = self.conn.last_insert_rowid();
        self.set_tag_families(id, families, true)?;
        Ok(self.tags()?.into_iter().find(|t| t.id == id).ok_or(NameError::Missing))
    }

    pub fn rename_tag(&self, id: i64, name: &str) -> Result<std::result::Result<(), NameError>> {
        let name = match clean_name(name) {
            Ok(n) => n,
            Err(e) => return Ok(Err(e)),
        };
        if name_taken(&self.conn, "tags", &name, Some(id))? {
            return Ok(Err(NameError::Taken(name)));
        }
        let n = self.conn.execute("UPDATE tags SET name = ?1 WHERE id = ?2", params![name, id])?;
        Ok(if n == 0 { Err(NameError::Missing) } else { Ok(()) })
    }

    pub fn delete_tag(&self, id: i64) -> Result<bool> {
        Ok(self.conn.execute("DELETE FROM tags WHERE id = ?1", params![id])? > 0)
    }

    /// Tag families (by display name or key; either works) or untag them.
    /// Returns false if the tag doesn't exist.
    pub fn set_tag_families(&mut self, id: i64, families: &[String], tagged: bool) -> Result<bool> {
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM tags WHERE id = ?1)",
            params![id],
            |r| r.get(0),
        )?;
        if !exists {
            return Ok(false);
        }
        let tx = self.conn.transaction()?;
        for f in families {
            let key = crate::family::group_key(f);
            if tagged {
                tx.execute(
                    "INSERT OR IGNORE INTO family_tags (tag_id, family, source) VALUES (?1, ?2, 'user')",
                    params![id, key],
                )?;
                tx.execute("DELETE FROM tag_dismissals WHERE tag_id = ?1 AND family = ?2", params![id, key])?;
            } else {
                // Taking off a tag an import put on: remember, so the next
                // import respects it.
                tx.execute(
                    "INSERT OR IGNORE INTO tag_dismissals (tag_id, family)
                     SELECT tag_id, family FROM family_tags
                     WHERE tag_id = ?1 AND family = ?2 AND source = 'google'",
                    params![id, key],
                )?;
                tx.execute("DELETE FROM family_tags WHERE tag_id = ?1 AND family = ?2", params![id, key])?;
            }
        }
        tx.commit()?;
        Ok(true)
    }

    /// Replace the tags an earlier import put on families with these.
    ///
    /// Tags are matched to earlier imports by Google's path, so a tag you
    /// renamed keeps your name. A new tag takes Google's name, or, if that's
    /// taken, the name with its group in brackets. Your own tags, and links
    /// you made by hand, are never touched; Google tags you removed from a
    /// family stay removed. Imported tags left on no family are deleted.
    pub fn import_google_tags(&mut self, taggings: &[GoogleTagging]) -> Result<GoogleTagCounts> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM family_tags WHERE source = 'google'", [])?;
        for t in taggings {
            if t.families.is_empty() {
                continue;
            }
            let existing: Option<i64> = tx
                .query_row("SELECT id FROM tags WHERE google_path = ?1", params![t.path], |r| r.get(0))
                .optional()?;
            let id = match existing {
                Some(id) => {
                    tx.execute("UPDATE tags SET grp = ?1 WHERE id = ?2", params![t.group, id])?;
                    id
                }
                None => {
                    let mut candidates = vec![t.name.clone()];
                    if !t.group.is_empty() {
                        candidates.push(format!("{} ({})", t.name, t.group));
                    }
                    candidates.push(format!("{} (Google)", t.name));
                    let mut name = None;
                    for c in candidates.into_iter().chain((2..).map(|n| format!("{} (Google {n})", t.name))) {
                        if !name_taken(&tx, "tags", &c, None)? {
                            name = Some(c);
                            break;
                        }
                    }
                    tx.execute(
                        "INSERT INTO tags (name, created_at, grp, source, google_path)
                         VALUES (?1, ?2, ?3, 'google', ?4)",
                        params![name, now(), t.group, t.path],
                    )?;
                    tx.last_insert_rowid()
                }
            };
            let mut link = tx.prepare_cached(
                "INSERT OR IGNORE INTO family_tags (tag_id, family, source)
                 SELECT ?1, ?2, 'google'
                 WHERE NOT EXISTS (SELECT 1 FROM tag_dismissals WHERE tag_id = ?1 AND family = ?2)",
            )?;
            for f in &t.families {
                link.execute(params![id, crate::family::group_key(f)])?;
            }
        }
        tx.execute(
            "DELETE FROM tags WHERE source = 'google'
               AND id NOT IN (SELECT tag_id FROM family_tags)",
            [],
        )?;
        tx.commit()?;
        self.google_tag_counts()
    }

    pub fn google_tag_counts(&self) -> Result<GoogleTagCounts> {
        let tags: i64 =
            self.conn.query_row("SELECT COUNT(*) FROM tags WHERE source = 'google'", [], |r| r.get(0))?;
        let families: i64 = self.conn.query_row(
            "SELECT COUNT(DISTINCT f.family) FROM family_tags f JOIN tags t ON t.id = f.tag_id
             WHERE t.source = 'google'",
            [],
            |r| r.get(0),
        )?;
        Ok(GoogleTagCounts { tags: tags as usize, families: families as usize })
    }

    /// Delete every imported tag, wherever it is. Your own tags stay.
    pub fn remove_google_tags(&self) -> Result<usize> {
        self.conn.execute("DELETE FROM tags WHERE source = 'google'", [])
    }

    // -- duplicates --------------------------------------------------------------

    /// Every set of files with identical bytes, largest waste first.
    pub fn duplicate_groups(&self) -> Result<Vec<DuplicateGroup>> {
        use std::collections::HashMap;
        let mut groups: Vec<DuplicateGroup> = Vec::new();
        let mut index: HashMap<String, usize> = HashMap::new();
        {
            let mut stmt = self.conn.prepare(
                "SELECT f.content_hash, f.path, f.file_size, f.modified FROM files f
                 JOIN (SELECT content_hash FROM files GROUP BY content_hash HAVING COUNT(*) > 1) d
                   ON d.content_hash = f.content_hash
                 ORDER BY f.content_hash, f.path",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?, r.get::<_, i64>(3)?))
            })?;
            for row in rows {
                let (hash, path, size, modified) = row?;
                let i = *index.entry(hash.clone()).or_insert_with(|| {
                    groups.push(DuplicateGroup {
                        content_hash: hash.clone(),
                        file_size: size as u64,
                        faces: Vec::new(),
                        copies: Vec::new(),
                    });
                    groups.len() - 1
                });
                groups[i].copies.push(DuplicateCopy { path: PathBuf::from(path), modified, in_use: false });
            }
        }
        let mut stmt = self.conn.prepare(
            "SELECT content_hash, family, subfamily, file_path FROM faces
             WHERE content_hash IN (SELECT content_hash FROM files GROUP BY content_hash HAVING COUNT(*) > 1)
             ORDER BY content_hash, face_index",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?))
        })?;
        for row in rows {
            let (hash, family, subfamily, path) = row?;
            if let Some(&i) = index.get(&hash) {
                let g = &mut groups[i];
                g.faces.push(format!("{family} {subfamily}"));
                for c in &mut g.copies {
                    if c.path == Path::new(&path) {
                        c.in_use = true;
                    }
                }
            }
        }
        // Most space wasted first; ties by name so the order is stable.
        groups.sort_by(|a, b| {
            let waste = |g: &DuplicateGroup| g.file_size * (g.copies.len() as u64 - 1);
            waste(b).cmp(&waste(a)).then_with(|| a.faces.cmp(&b.faces))
        });
        Ok(groups)
    }

    /// Forget files that were removed on purpose, repointing or dropping the
    /// faces they held, as for files found missing by a scan.
    pub fn forget_files(&mut self, paths: &[PathBuf]) -> Result<usize> {
        let tx = self.conn.transaction()?;
        {
            let mut del = tx.prepare("DELETE FROM files WHERE path = ?1")?;
            for p in paths {
                del.execute(params![p.to_string_lossy()])?;
            }
        }
        let removed = repair(&tx)?;
        tx.commit()?;
        Ok(removed)
    }
}

/// Point each face at a file that still holds its bytes, or drop it.
///
/// Covers three situations with one rule: a file that changed content, a
/// duplicate copy that was deleted while another survives, and a font that is
/// gone entirely. Returns the number of faces deleted.
fn repair(conn: &Connection) -> Result<usize> {
    conn.execute(
        "UPDATE faces SET file_path = (
            SELECT path FROM files
            WHERE files.content_hash = faces.content_hash
            ORDER BY path LIMIT 1)
         WHERE NOT EXISTS (
            SELECT 1 FROM files
            WHERE files.path = faces.file_path AND files.content_hash = faces.content_hash)
           AND EXISTS (SELECT 1 FROM files WHERE files.content_hash = faces.content_hash)",
        [],
    )?;
    conn.execute(
        "DELETE FROM faces
         WHERE NOT EXISTS (SELECT 1 FROM files WHERE files.content_hash = faces.content_hash)
           AND face_id NOT IN (SELECT face_id FROM activation)",
        [],
    )
}

/// Indexes written before the files table existed: seed it from faces, or
/// the first repair would treat every face as orphaned.
fn migrate(conn: &Connection) -> Result<()> {
    // Columns added after a table was first created.
    let has = |table: &str, col: &str| -> Result<bool> {
        let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
        let names: Vec<String> = stmt.query_map([], |r| r.get(1))?.collect::<Result<_>>()?;
        Ok(names.iter().any(|n| n == col))
    };
    for (table, col, def) in [
        ("tags", "grp", "TEXT"),
        ("tags", "source", "TEXT NOT NULL DEFAULT 'user'"),
        ("tags", "google_path", "TEXT"),
        ("family_tags", "source", "TEXT NOT NULL DEFAULT 'user'"),
    ] {
        if !has(table, col)? {
            conn.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {col} {def}"))?;
        }
    }

    let files: i64 = conn.query_row("SELECT COUNT(*) FROM files", [], |r| r.get(0))?;
    if files == 0 {
        conn.execute(
            "INSERT OR IGNORE INTO files (path, content_hash, file_size, modified)
             SELECT file_path, content_hash, file_size, modified FROM faces",
            [],
        )?;
    }
    Ok(())
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn format_name(f: FontFormat) -> &'static str {
    match f {
        FontFormat::TrueType => "truetype",
        FontFormat::OpenType => "opentype",
        FontFormat::Collection => "collection",
        FontFormat::Unsupported => "unsupported",
    }
}

fn parse_format(s: &str) -> FontFormat {
    match s {
        "truetype" => FontFormat::TrueType,
        "opentype" => FontFormat::OpenType,
        "collection" => FontFormat::Collection,
        _ => FontFormat::Unsupported,
    }
}

fn row_to_face(r: &rusqlite::Row) -> rusqlite::Result<FaceRecord> {
    let axes_json: String = r.get("axes_json")?;
    let axes: Vec<VariationAxis> = serde_json::from_str(&axes_json).unwrap_or_default();
    Ok(FaceRecord {
        face_id: r.get("face_id")?,
        content_hash: r.get("content_hash")?,
        file_path: PathBuf::from(r.get::<_, String>("file_path")?),
        face_index: r.get("face_index")?,
        file_size: r.get::<_, i64>("file_size")? as u64,
        modified: r.get("modified")?,
        postscript_name: r.get("postscript_name")?,
        family: r.get("family")?,
        subfamily: r.get("subfamily")?,
        legacy_family: r.get("legacy_family")?,
        legacy_subfamily: r.get("legacy_subfamily")?,
        full_name: r.get("full_name")?,
        version: r.get("version")?,
        weight: r.get("weight")?,
        width: r.get("width")?,
        italic: r.get("italic")?,
        oblique: r.get("oblique")?,
        monospaced: r.get("monospaced")?,
        is_variable: r.get("is_variable")?,
        axes,
        format: parse_format(&r.get::<_, String>("format")?),
        units_per_em: r.get("units_per_em")?,
        glyph_count: r.get("glyph_count")?,
        codepoint_count: r.get("codepoint_count")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::FontFormat;

    fn face(id: &str, path: &str) -> FaceRecord {
        FaceRecord {
            face_id: id.into(),
            content_hash: id.into(),
            file_path: path.into(),
            face_index: 0,
            file_size: 10,
            modified: 1,
            postscript_name: Some("Test-Regular".into()),
            family: "Test".into(),
            subfamily: "Regular".into(),
            legacy_family: "Test".into(),
            legacy_subfamily: "Regular".into(),
            full_name: None,
            version: None,
            weight: 400,
            width: 5,
            italic: false,
            oblique: false,
            monospaced: false,
            is_variable: true,
            axes: vec![VariationAxis {
                tag: "wght".into(),
                min: 100.0,
                default: 400.0,
                max: 900.0,
                name: Some("Weight".into()),
                hidden: false,
            }],
            format: FontFormat::OpenType,
            units_per_em: 1000,
            glyph_count: 3,
            codepoint_count: 2,
        }
    }

    #[test]
    fn round_trips_a_face_including_axes() {
        let mut idx = Index::open_in_memory().unwrap();
        let f = face("a", "/f/Test.otf");
        idx.upsert_faces(std::slice::from_ref(&f)).unwrap();
        assert_eq!(idx.face("a").unwrap().unwrap(), f);
    }

    #[test]
    fn rescan_updates_metadata_without_dropping_activation() {
        let mut idx = Index::open_in_memory().unwrap();
        idx.upsert_faces(&[face("a", "/f/Test.otf")]).unwrap();
        idx.record_activation("a", Path::new("/f/Test.otf"), false).unwrap();

        let mut fixed = face("a", "/f/Test.otf");
        fixed.family = "Test Corrected".into();
        idx.upsert_faces(&[fixed]).unwrap();

        assert_eq!(idx.face("a").unwrap().unwrap().family, "Test Corrected");
        assert!(idx.active_face_ids().unwrap().contains("a"), "ledger row survived");
    }

    #[test]
    fn changed_file_replaces_its_stale_identity() {
        let mut idx = Index::open_in_memory().unwrap();
        idx.upsert_faces(&[face("old", "/f/Test.otf")]).unwrap();
        idx.upsert_faces(&[face("new", "/f/Test.otf")]).unwrap();
        assert!(idx.face("old").unwrap().is_none());
        assert_eq!(idx.face_count().unwrap(), 1);
    }

    #[test]
    fn changed_file_keeps_stale_identity_while_active() {
        let mut idx = Index::open_in_memory().unwrap();
        idx.upsert_faces(&[face("old", "/f/Test.otf")]).unwrap();
        idx.record_activation("old", Path::new("/f/Test.otf"), false).unwrap();
        idx.upsert_faces(&[face("new", "/f/Test.otf")]).unwrap();
        assert!(idx.face("old").unwrap().is_some(), "still needed to undo registration");
    }

    #[test]
    fn duplicate_copies_are_all_known_so_none_is_rehashed() {
        let mut idx = Index::open_in_memory().unwrap();
        let a = face("same", "/lib/a/Font.otf");
        let b = face("same", "/lib/b/Font.otf");
        idx.upsert_faces(&[a, b]).unwrap();
        assert_eq!(idx.face_count().unwrap(), 1, "identical bytes are one face");
        let stats = idx.known_stats().unwrap();
        assert!(stats.contains_key(Path::new("/lib/a/Font.otf")));
        assert!(stats.contains_key(Path::new("/lib/b/Font.otf")));
        assert_eq!(idx.duplicate_file_count().unwrap(), 1);
    }

    #[test]
    fn deleting_one_copy_repoints_the_face_to_the_other() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.otf");
        let b = dir.path().join("b.otf");
        std::fs::write(&a, b"x").unwrap();
        std::fs::write(&b, b"x").unwrap();
        let mut idx = Index::open_in_memory().unwrap();
        idx.upsert_faces(&[face("same", a.to_str().unwrap()), face("same", b.to_str().unwrap())])
            .unwrap();

        let current = idx.face("same").unwrap().unwrap().file_path;
        std::fs::remove_file(&current).unwrap();
        assert_eq!(idx.remove_missing(None).unwrap(), 0, "nothing removed");
        let now = idx.face("same").unwrap().unwrap().file_path;
        assert_ne!(now, current);
        assert!(now.exists());
    }

    #[test]
    fn scoped_removal_leaves_other_folders_alone() {
        let mut idx = Index::open_in_memory().unwrap();
        // Neither file exists on disk.
        idx.upsert_faces(&[face("a", "/nowhere/one/A.otf"), face("b", "/nowhere/two/B.otf")])
            .unwrap();
        assert_eq!(idx.remove_missing(Some(Path::new("/nowhere/one"))).unwrap(), 1);
        assert!(idx.face("a").unwrap().is_none());
        assert!(idx.face("b").unwrap().is_some(), "outside the scope, untouched");
    }

    #[test]
    fn scope_matches_whole_path_components() {
        let mut idx = Index::open_in_memory().unwrap();
        idx.upsert_faces(&[face("b", "/nowhere/one-more/B.otf")]).unwrap();
        // "/nowhere/one" is a string prefix of "/nowhere/one-more" but not a parent.
        assert_eq!(idx.remove_missing(Some(Path::new("/nowhere/one"))).unwrap(), 0);
    }

    #[test]
    fn conflict_candidates_are_found_by_query() {
        let mut idx = Index::open_in_memory().unwrap();
        let mut x = face("x", "/f/X.otf");
        x.postscript_name = Some("Shared".into());
        let mut y = face("y", "/f/Y.otf");
        y.postscript_name = Some("Shared".into());
        let mut z = face("z", "/f/Z.otf");
        z.postscript_name = Some("Unique".into());
        idx.upsert_faces(&[x, y, z]).unwrap();
        let mut ids: Vec<String> =
            idx.conflict_candidates().unwrap().into_iter().map(|f| f.face_id).collect();
        ids.sort();
        assert_eq!(ids, ["x", "y"]);
    }

    #[test]
    fn existing_indexes_are_migrated_without_losing_faces() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("i.sqlite");
        {
            let mut idx = Index::open(&path).unwrap();
            idx.upsert_faces(&[face("a", "/f/A.otf")]).unwrap();
            // Simulate an index from before the files table.
            idx.conn.execute("DELETE FROM files", []).unwrap();
        }
        let mut idx = Index::open(&path).unwrap();
        idx.upsert_faces(&[face("b", "/f/B.otf")]).unwrap();
        assert!(idx.face("a").unwrap().is_some());
    }

    #[test]
    fn removing_a_folder_keeps_faces_that_live_elsewhere_too() {
        let mut idx = Index::open_in_memory().unwrap();
        idx.add_root(Path::new("/lib/a")).unwrap();
        idx.add_root(Path::new("/lib/b")).unwrap();
        idx.upsert_faces(&[
            face("only-a", "/lib/a/One.otf"),
            face("both", "/lib/a/Two.otf"),
            face("both", "/lib/b/Two.otf"),
        ])
        .unwrap();
        assert_eq!(idx.remove_root(Path::new("/lib/a")).unwrap(), Ok(1));
        assert!(idx.face("only-a").unwrap().is_none());
        assert_eq!(idx.face("both").unwrap().unwrap().file_path, Path::new("/lib/b/Two.otf"));
        assert_eq!(idx.roots().unwrap(), vec![PathBuf::from("/lib/b")]);
    }

    #[test]
    fn removing_a_folder_with_active_fonts_is_refused() {
        let mut idx = Index::open_in_memory().unwrap();
        idx.add_root(Path::new("/lib/a")).unwrap();
        idx.upsert_faces(&[face("x", "/lib/a/X.otf")]).unwrap();
        idx.record_activation("x", Path::new("/lib/a/X.otf"), false).unwrap();
        assert_eq!(idx.remove_root(Path::new("/lib/a")).unwrap(), Err(1));
        assert!(idx.face("x").unwrap().is_some());
        assert_eq!(idx.roots().unwrap().len(), 1);
    }

    fn ids(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn collections_hold_faces_and_refuse_duplicate_names() {
        let mut idx = Index::open_in_memory().unwrap();
        idx.upsert_faces(&[face("a", "/f/A.otf"), face("b", "/f/B.otf")]).unwrap();
        let c = idx.create_collection("  Display   faces ", &ids(&["a"])).unwrap().unwrap();
        assert_eq!(c.name, "Display faces", "whitespace tidied");
        assert_eq!(c.face_ids, ["a"]);

        assert_eq!(
            idx.create_collection("display FACES", &[]).unwrap(),
            Err(NameError::Taken("display FACES".into()))
        );
        assert_eq!(idx.create_collection("   ", &[]).unwrap(), Err(NameError::Empty));

        assert!(idx.set_collection_members(c.id, &ids(&["b", "a"]), true).unwrap());
        assert_eq!(idx.collections().unwrap()[0].face_ids, ["a", "b"], "no duplicates");
        idx.set_collection_members(c.id, &ids(&["a"]), false).unwrap();
        assert_eq!(idx.collections().unwrap()[0].face_ids, ["b"]);

        assert_eq!(idx.rename_collection(c.id, "Headlines").unwrap(), Ok(()));
        assert_eq!(idx.rename_collection(c.id, "headlines").unwrap(), Ok(()), "own name, new case");
        assert!(idx.delete_collection(c.id).unwrap());
        assert!(idx.collections().unwrap().is_empty());
        assert!(idx.face("b").unwrap().is_some(), "deleting a collection keeps its fonts");
        assert!(!idx.set_collection_members(c.id, &ids(&["a"]), true).unwrap());
    }

    #[test]
    fn membership_survives_a_face_leaving_and_returning() {
        let mut idx = Index::open_in_memory().unwrap();
        idx.upsert_faces(&[face("a", "/nowhere/A.otf")]).unwrap();
        let c = idx.create_collection("Kept", &ids(&["a"])).unwrap().unwrap();

        // The file is missing (say, its drive is unplugged): the face goes.
        idx.remove_missing(None).unwrap();
        assert!(idx.face("a").unwrap().is_none());
        assert!(idx.collections().unwrap()[0].face_ids.is_empty(), "absent faces aren't listed");

        // Plugged back in and rescanned: same bytes, same id, same collection.
        idx.upsert_faces(&[face("a", "/nowhere/A.otf")]).unwrap();
        assert_eq!(idx.collections().unwrap()[0].face_ids, ["a"]);
        assert_eq!(idx.collections().unwrap()[0].id, c.id);
    }

    #[test]
    fn pairings_save_overwrite_and_delete() {
        let idx = Index::open_in_memory().unwrap();
        let p = idx.save_pairing(None, "Editorial", r#"{"base":16}"#).unwrap().unwrap();
        assert_eq!(
            idx.save_pairing(None, "editorial", "{}").unwrap(),
            Err(NameError::Taken("editorial".into()))
        );
        let p2 = idx.save_pairing(Some(p.id), "Editorial", r#"{"base":18}"#).unwrap().unwrap();
        assert_eq!(p2.id, p.id);
        let all = idx.pairings().unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].settings, r#"{"base":18}"#);
        assert_eq!(idx.save_pairing(Some(999), "Gone", "{}").unwrap(), Err(NameError::Missing));
        assert!(idx.delete_pairing(p.id).unwrap());
        assert!(idx.pairings().unwrap().is_empty());
    }

    #[test]
    fn old_collection_table_is_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("i.sqlite");
        {
            let c = Connection::open(&path).unwrap();
            c.execute_batch(
                "CREATE TABLE collections (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE, created_at INTEGER NOT NULL);
                 CREATE TABLE collection_faces (collection_id INTEGER, face_id TEXT);",
            )
            .unwrap();
        }
        let mut idx = Index::open(&path).unwrap();
        let gone: bool = idx
            .conn
            .query_row(
                "SELECT NOT EXISTS(SELECT 1 FROM sqlite_master WHERE name = 'collection_faces')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(gone);
        assert!(idx.create_collection("Works", &[]).unwrap().is_ok());
    }

    #[test]
    fn tags_are_on_families_whatever_the_case() {
        let mut idx = Index::open_in_memory().unwrap();
        let t = idx.create_tag("Geometric", &ids(&["Futura PT"])).unwrap().unwrap();
        assert_eq!(t.families, ["futura pt"]);
        idx.set_tag_families(t.id, &ids(&["FUTURA PT", "Inter"]), true).unwrap();
        assert_eq!(idx.tags().unwrap()[0].families, ["futura pt", "inter"], "no duplicate by case");
        idx.set_tag_families(t.id, &ids(&["Futura PT"]), false).unwrap();
        assert_eq!(idx.tags().unwrap()[0].families, ["inter"]);
        assert_eq!(idx.create_tag("geometric", &[]).unwrap(), Err(NameError::Taken("geometric".into())));
        assert_eq!(idx.rename_tag(t.id, "Geometric sans").unwrap(), Ok(()));
        assert!(idx.delete_tag(t.id).unwrap());
        assert!(idx.tags().unwrap().is_empty());
        let n: i64 = idx.conn.query_row("SELECT COUNT(*) FROM family_tags", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 0, "deleting a tag removes it from families");
    }

    fn gt(path: &str, fams: &[&str]) -> GoogleTagging {
        let (group, name) = crate::google::split_tag_path(path).unwrap();
        GoogleTagging { path: path.into(), group, name, families: ids(fams) }
    }

    #[test]
    fn google_tags_import_alongside_your_own() {
        let mut idx = Index::open_in_memory().unwrap();
        let mine = idx.create_tag("Humanist", &ids(&["Gill Sans"])).unwrap().unwrap();
        let counts = idx
            .import_google_tags(&[gt("/Sans/Humanist", &["Open Sans", "Gill Sans"]), gt("/Serif/Humanist", &["Lora"])])
            .unwrap();
        assert_eq!(counts, GoogleTagCounts { tags: 2, families: 3 });
        let tags = idx.tags().unwrap();
        let names: Vec<_> = tags.iter().map(|t| (t.name.as_str(), t.source.as_str())).collect();
        assert!(names.contains(&("Humanist", "user")), "your tag is untouched");
        assert!(names.contains(&("Humanist (Sans)", "google")), "a clash takes the group");
        assert!(names.contains(&("Humanist (Serif)", "google")));
        assert_eq!(tags.iter().find(|t| t.id == mine.id).unwrap().families, ["gill sans"]);
    }

    #[test]
    fn reimporting_replaces_links_and_respects_removals() {
        let mut idx = Index::open_in_memory().unwrap();
        idx.import_google_tags(&[gt("/Expressive/Calm", &["Lora", "Inter"])]).unwrap();
        let calm = idx.tags().unwrap()[0].clone();
        assert_eq!(calm.group.as_deref(), Some("Expressive"));

        // Renamed by you, and taken off Inter by you.
        idx.rename_tag(calm.id, "Quiet").unwrap().unwrap();
        idx.set_tag_families(calm.id, &ids(&["Inter"]), false).unwrap();
        // Also put on Merriweather by hand.
        idx.set_tag_families(calm.id, &ids(&["Merriweather"]), true).unwrap();

        idx.import_google_tags(&[gt("/Expressive/Calm", &["Lora", "Inter", "Roboto"])]).unwrap();
        let t = idx.tags().unwrap().into_iter().find(|t| t.id == calm.id).unwrap();
        assert_eq!(t.name, "Quiet", "your rename survives");
        assert_eq!(t.families, ["lora", "merriweather", "roboto"], "Inter stays removed; hand links stay");

        // Tags that no longer apply to anything go.
        idx.import_google_tags(&[gt("/Expressive/Calm", &[])]).unwrap();
        let left = idx.tags().unwrap();
        assert_eq!(left.len(), 1, "kept: it still has your hand-made link");
        assert_eq!(left[0].families, ["merriweather"]);

        assert_eq!(idx.remove_google_tags().unwrap(), 1);
        assert!(idx.tags().unwrap().is_empty());
    }

    #[test]
    fn tag_columns_are_added_to_older_indexes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("i.sqlite");
        {
            let c = Connection::open(&path).unwrap();
            c.execute_batch(
                "CREATE TABLE tags (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE, created_at INTEGER NOT NULL);
                 CREATE TABLE family_tags (tag_id INTEGER NOT NULL, family TEXT NOT NULL, PRIMARY KEY (tag_id, family));
                 INSERT INTO tags VALUES (1, 'Old', 0); INSERT INTO family_tags VALUES (1, 'lora');",
            )
            .unwrap();
        }
        let mut idx = Index::open(&path).unwrap();
        let t = &idx.tags().unwrap()[0];
        assert_eq!((t.name.as_str(), t.source.as_str()), ("Old", "user"));
        assert!(idx.import_google_tags(&[gt("/Sans/Grotesque", &["Inter"])]).is_ok());
    }

    #[test]
    fn duplicate_groups_list_every_copy_and_the_one_in_use() {
        let mut idx = Index::open_in_memory().unwrap();
        idx.upsert_faces(&[
            face("same", "/lib/b/Font.otf"),
            face("same", "/lib/a/Font.otf"),
            face("solo", "/lib/a/Other.otf"),
        ])
        .unwrap();
        let groups = idx.duplicate_groups().unwrap();
        assert_eq!(groups.len(), 1);
        let g = &groups[0];
        assert_eq!(g.faces, ["Test Regular"]);
        assert_eq!(g.copies.len(), 2);
        let used: Vec<_> = g.copies.iter().filter(|c| c.in_use).collect();
        assert_eq!(used.len(), 1, "exactly one copy is in use");

        let spare = g.copies.iter().find(|c| !c.in_use).unwrap().path.clone();
        assert_eq!(idx.forget_files(&[spare]).unwrap(), 0, "the face lives on in the other copy");
        assert!(idx.duplicate_groups().unwrap().is_empty());
        assert!(idx.face("same").unwrap().is_some());
    }

    #[test]
    fn known_stats_reports_each_file_once() {
        let mut idx = Index::open_in_memory().unwrap();
        let mut a = face("a", "/f/Coll.ttc");
        let mut b = face("b", "/f/Coll.ttc");
        a.face_index = 0;
        b.face_index = 1;
        idx.upsert_faces(&[a, b]).unwrap();
        let stats = idx.known_stats().unwrap();
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[Path::new("/f/Coll.ttc")], (10, 1));
    }
}
