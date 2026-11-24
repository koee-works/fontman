// Typed wrappers over the Tauri commands in src-tauri/src/lib.rs.
// Field names mirror the Rust DTOs, which serialize camelCase.

import { invoke } from '@tauri-apps/api/core';

export type FontFormat = 'truetype' | 'opentype' | 'collection' | 'unsupported';
export type FamilyActivation = 'none' | 'partial' | 'all';

export interface FaceSummary {
  id: string;
  subfamily: string;
  fullName: string | null;
  postscriptName: string | null;
  weight: number;
  width: number;
  italic: boolean;
  variable: boolean;
  format: FontFormat;
  active: boolean;
  installed: boolean;
  /** Approximate bytes held once loaded, for the preview memory budget. */
  bytes: number;
  /** Pins every axis to its default; null for static faces. */
  variation: string | null;
  /** File modification time, Unix seconds. */
  modified: number;
  /** Variation axis tags, such as "wght". */
  axes: string[];
  /** The wght axis range of a variable face. */
  weightRange: [number, number] | null;
}

export interface FamilySummary {
  name: string;
  activation: FamilyActivation;
  representative: string;
  variable: boolean;
  monospaced: boolean;
  installed: boolean;
  faces: FaceSummary[];
}

export interface RootInfo {
  path: string;
  installed: boolean;
  exists: boolean;
}

export interface LibraryStatus {
  roots: RootInfo[];
  faceCount: number;
  activeCount: number;
  duplicateFiles: number;
  scanning: boolean;
  platform: string;
  watch: WatchStatus;
}

export interface WatchStatus {
  enabled: boolean;
  /** Library folders being watched right now. */
  folders: number;
  error: string | null;
}

export interface ScanProgress {
  seen: number;
  toRead: number;
  read: number;
}

export interface ScanSummary {
  filesSeen: number;
  filesParsed: number;
  filesSkipped: number;
  facesIndexed: number;
  removed: number;
  /** Files gone from disk, including duplicate copies. */
  filesRemoved: number;
  failures: { path: string; error: string }[];
}

export interface VariationAxis {
  tag: string;
  min: number;
  default: number;
  max: number;
  name: string | null;
  hidden: boolean;
}

/** An OpenType feature, with the designer's name for stylistic sets and
 *  character variants, and the characters a variant changes. */
export interface FontFeature {
  tag: string;
  name: string | null;
  chars: string[];
}

/** A face's vertical metrics, in font units. */
export interface FontMetrics {
  unitsPerEm: number;
  ascender: number;
  descender: number;
  lineGap: number;
  xHeight: number | null;
  capHeight: number | null;
  /** The heights were measured from H and x; the font doesn't state them. */
  measured: boolean;
}

/** One character's glyph: path data in font units (y up), its points, handles and spacing. */
export interface GlyphOutline {
  ch: string;
  glyphId: number | null;
  name: string | null;
  advance: number;
  lsb: number;
  bbox: [number, number, number, number] | null;
  d: string;
  onCurve: [number, number][];
  offCurve: [number, number][];
  handles: [number, number, number, number][];
  contours: number;
}

export interface Outlines {
  metrics: FontMetrics;
  glyphs: GlyphOutline[];
}

export interface FaceDetail {
  faceId: string;
  filePath: string;
  faceIndex: number;
  fileSize: number;
  postscriptName: string | null;
  family: string;
  subfamily: string;
  legacyFamily: string;
  legacySubfamily: string;
  fullName: string | null;
  version: string | null;
  weight: number;
  width: number;
  italic: boolean;
  oblique: boolean;
  monospaced: boolean;
  isVariable: boolean;
  axes: VariationAxis[];
  format: FontFormat;
  unitsPerEm: number;
  glyphCount: number;
  codepointCount: number;
  active: boolean;
  installed: boolean;
  cssFamily: string;
}

export interface ToggleResult {
  id: string;
  ok: boolean;
  error: string | null;
}

export interface Conflict {
  postscriptName: string;
  faceIds: string[];
}

export interface ConflictReport {
  active: Conflict[];
  library: Conflict[];
}

export type KeyStorage = 'keychain' | 'file';

export interface GoogleSettings {
  keySaved: boolean;
  /** Last four characters only; the key itself never reaches the UI. */
  keyEnding: string | null;
  storage: KeyStorage | null;
  catalogFamilies: number | null;
  catalogFetchedAt: number | null;
  downloadFolder: string;
}

export interface SaveKeyResult {
  settings: GoogleSettings;
  verified: boolean;
  warning: string | null;
}

export type GoogleStatus = 'available' | 'downloaded' | 'update';

export interface GoogleFamilySummary {
  family: string;
  category: string;
  styles: number;
  variable: boolean;
  axes: string[];
  scripts: number;
  /** Google's subset names, such as "latin" or "cyrillic-ext". */
  subsets: string[];
  italic: boolean;
  /** Lightest and heaviest weight available. */
  weightRange: [number, number];
  /** Google's tag paths, such as "/Sans/Humanist". */
  tags: string[];
  /** Position in Google's popularity order, from 0. */
  rank: number;
  version: string;
  lastModified: string;
  status: GoogleStatus;
  /** The library family it was indexed as, once downloaded. */
  library: string | null;
}

export interface CatalogView {
  families: GoogleFamilySummary[];
  fetchedAt: number;
  stale: string | null;
}

export interface DownloadResult {
  family: string;
  ok: boolean;
  error: string | null;
}

export interface DownloadProgress {
  family: string;
  done: number;
  total: number;
}

export interface Collection {
  id: number;
  name: string;
  /** Members currently in the library, oldest first. */
  faceIds: string[];
}

export interface Pairing {
  id: number;
  name: string;
  /** The style viewer's settings, as JSON. */
  settings: string;
  updatedAt: number;
}

export type ExportKind = 'css' | 'json';

/** An imported Zed theme family, as stored by the app. */
export interface UserTheme {
  file: string;
  family: import('./themes/zed').ZedFamily;
}

export interface Tag {
  id: number;
  name: string;
  /** Tagged families by lowercased name, including any not in the library now. */
  families: string[];
  /** Google's group for an imported tag, such as "Sans". */
  group: string | null;
  source: 'user' | 'google';
}

export interface GoogleTagStatus {
  enabled: boolean;
  /** 1–100: how strongly Google must rate a tag for it to apply. */
  minWeight: number;
  importedAt: number | null;
  /** Imported tags in use, and the families they're on. */
  tags: number;
  families: number;
  keySaved: boolean;
}

export interface DuplicateCopy {
  path: string;
  modified: number;
  /** The copy the app previews and activates from. */
  inUse: boolean;
  /** In a system font folder; can't be trashed here. */
  installed: boolean;
  /** An active style is registered from this file. */
  active: boolean;
  exists: boolean;
}

export interface DuplicateGroup {
  contentHash: string;
  fileSize: number;
  /** "Family Style" for each face in the file. */
  faces: string[];
  copies: DuplicateCopy[];
}

export interface TrashReport {
  results: { path: string; ok: boolean; error: string | null }[];
  groups: DuplicateGroup[];
}

export const api = {
  libraryStatus: () => invoke<LibraryStatus>('library_status'),
  /** Whole library (incremental), or one folder (re-reads every file in it). */
  scanLibrary: (opts: { full?: boolean; folder?: string } = {}) =>
    invoke<ScanSummary>('scan_library', { full: opts.full ?? false, folder: opts.folder ?? null }),
  addRoot: (path: string) => invoke<void>('add_root', { path }),
  /** Returns how many styles left the library. Refused while any are active. */
  removeRoot: (path: string) => invoke<number>('remove_root', { path }),
  listFamilies: () => invoke<FamilySummary[]>('list_families'),
  faceDetail: (id: string) => invoke<FaceDetail>('face_detail', { id }),
  faceCoverage: (id: string) => invoke<[number, number][]>('face_coverage', { id }),
  /** The OpenType features a face offers, read from its file. */
  faceFeatures: (id: string) => invoke<FontFeature[]>('face_features', { id }),
  /** Outlines of up to 32 characters, and the face's vertical metrics. */
  glyphOutlines: (id: string, text: string) => invoke<Outlines>('glyph_outlines', { id, text }),
  setFacesActive: (ids: string[], active: boolean) =>
    invoke<ToggleResult[]>('set_faces_active', { ids, active }),
  conflicts: () => invoke<ConflictReport>('conflicts'),

  googleSettings: () => invoke<GoogleSettings>('google_settings'),
  /** Checks the key with Google before saving; a rejected key is not saved. */
  saveGoogleKey: (key: string) => invoke<SaveKeyResult>('save_google_key', { key }),
  removeGoogleKey: () => invoke<GoogleSettings>('remove_google_key'),
  googleCatalog: (refresh = false) => invoke<CatalogView>('google_catalog', { refresh }),
  googleDownload: (families: string[], activate = false) =>
    invoke<DownloadResult[]>('google_download', { families, activate }),

  listCollections: () => invoke<Collection[]>('list_collections'),
  createCollection: (name: string, faceIds: string[] = []) =>
    invoke<Collection>('create_collection', { name, faceIds }),
  renameCollection: (id: number, name: string) => invoke<void>('rename_collection', { id, name }),
  /** The collection only; its fonts stay in the library. */
  deleteCollection: (id: number) => invoke<void>('delete_collection', { id }),
  setCollectionMembers: (id: number, faceIds: string[], member: boolean) =>
    invoke<Collection>('set_collection_members', { id, faceIds, member }),

  listPairings: () => invoke<Pairing[]>('list_pairings'),
  /** Create (id null) or overwrite. */
  savePairing: (id: number | null, name: string, settings: object) =>
    invoke<Pairing>('save_pairing', { id, name, settings }),
  deletePairing: (id: number) => invoke<void>('delete_pairing', { id }),
  listTags: () => invoke<Tag[]>('list_tags'),
  createTag: (name: string, families: string[] = []) => invoke<Tag>('create_tag', { name, families }),
  renameTag: (id: number, name: string) => invoke<void>('rename_tag', { id, name }),
  deleteTag: (id: number) => invoke<void>('delete_tag', { id }),
  setTagFamilies: (id: number, families: string[], tagged: boolean) =>
    invoke<Tag>('set_tag_families', { id, families, tagged }),

  googleTagsStatus: () => invoke<GoogleTagStatus>('google_tags_status'),
  /** Imports Google's tags and keeps them up to date. May fetch the catalog. */
  importGoogleTags: (minWeight: number) => invoke<GoogleTagStatus>('import_google_tags', { minWeight }),
  /** Removes every imported tag; your own tags stay. */
  removeGoogleTags: () => invoke<GoogleTagStatus>('remove_google_tags'),

  duplicateGroups: () => invoke<DuplicateGroup[]>('duplicate_groups'),
  /** Moves copies to the Trash. Refuses system fonts, copies in use by an active style, and last copies. */
  trashDuplicates: (paths: string[]) => invoke<TrashReport>('trash_duplicates', { paths }),

  listUserThemes: () => invoke<UserTheme[]>('list_user_themes'),
  /** Opens a file dialog; resolves to the imported family, or null if cancelled. */
  importZedTheme: () => invoke<UserTheme | null>('import_zed_theme'),
  removeUserTheme: (file: string) => invoke<void>('remove_user_theme', { file }),

  setWatching: (enabled: boolean) => invoke<WatchStatus>('set_watching', { enabled }),

  /** Opens a native save dialog. Resolves to the saved path, or null if cancelled. */
  saveTextFile: (kind: ExportKind, fileName: string, contents: string) =>
    invoke<string | null>('save_text_file', { kind, fileName, contents }),
};
