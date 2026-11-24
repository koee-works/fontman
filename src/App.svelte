<script lang="ts">
  import { onMount } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { revealItemInDir } from '@tauri-apps/plugin-opener';
  import { Tooltip } from 'bits-ui';
  import { listen } from '@tauri-apps/api/event';
  import {
    api,
    type Collection,
    type ConflictReport,
    type Tag,
    type FaceDetail,
    type FaceSummary,
    type FamilyActivation,
    type FamilySummary,
    type LibraryStatus,
    type RootInfo,
    type ScanProgress,
    type ScanSummary,
  } from './lib/api';
  import FamilyRow from './lib/FamilyRow.svelte';
  import FamilyCard from './lib/FamilyCard.svelte';
  import StyleViewer from './lib/viewer/StyleViewer.svelte';
  import { viewer } from './lib/viewer/state.svelte';
  import type { Role } from './lib/viewer/model';
  import Segmented from './lib/ui/Segmented.svelte';
  import Inspector from './lib/Inspector.svelte';
  import VirtualList from './lib/VirtualList.svelte';
  import Settings from './lib/Settings.svelte';
  import SettingsWindow from './lib/SettingsWindow.svelte';
  import GoogleFonts from './lib/GoogleFonts.svelte';
  import { liveStatus, mark, painted, perfEnabled } from './lib/perf';
  import { loadStats, setFaceSizes } from './lib/fonts';
  import Button from './lib/ui/Button.svelte';
  import Callout from './lib/ui/Callout.svelte';
  import ConfirmDialog from './lib/ui/ConfirmDialog.svelte';
  import PromptDialog from './lib/ui/PromptDialog.svelte';
  import CollectionMenu from './lib/ui/CollectionMenu.svelte';
  import Duplicates from './lib/Duplicates.svelte';
  import Compare from './lib/Compare.svelte';
  import FamilyContextMenu from './lib/FamilyContextMenu.svelte';
  import OutlinePanel from './lib/OutlinePanel.svelte';
  import { drawer, motion } from './lib/motion.svelte';
  import { compare } from './lib/compare-state.svelte';
  import { onDrop, type DragPayload } from './lib/drag';
  import TagMenu from './lib/ui/TagMenu.svelte';
  import CommandPalette from './lib/CommandPalette.svelte';
  import { fuzzy, type Command } from './lib/palette';
  import { theme } from './lib/theme.svelte';
  import Checkbox from './lib/ui/Checkbox.svelte';
  import Icon, { type IconName } from './lib/ui/Icon.svelte';
  import IconButton from './lib/ui/IconButton.svelte';
  import Menu from './lib/ui/Menu.svelte';
  import SearchField from './lib/ui/SearchField.svelte';
  import Slider from './lib/ui/Slider.svelte';
  import NumberField from './lib/ui/NumberField.svelte';
  import FilterBar from './lib/filters/FilterBar.svelte';
  import { libraryFacets, librarySorts } from './lib/facets';
  import { QueryState } from './lib/query-state.svelte';
  import {
    applyFacets,
    closeMatches,
    isEmptyQuery,
    matchQuery,
    parseQuery,
    relevance,
    sortItems,
    type ParsedQuery,
  } from './lib/query';

  type Filter = 'all' | 'library' | 'active' | 'variable' | 'conflicts' | 'installed' | 'collection' | 'tag';
  type View = 'library' | 'google' | 'viewer' | 'duplicates' | 'compare';

  let view = $state<View>('library');
  let settingsOpen = $state(false);
  let paletteOpen = $state(false);
  /** The sidebar can be hidden (Ctrl/⌘+B), as in Zed. Remembered. */
  let railOpen = $state(
    (() => {
      try {
        return localStorage.getItem('rail') !== 'hidden';
      } catch {
        return true;
      }
    })(),
  );
  function setRail(open: boolean) {
    railOpen = open;
    try {
      localStorage.setItem('rail', open ? 'shown' : 'hidden');
    } catch {
      /* not persisted */
    }
  }
  /**
   * The inspector can widen over most of the window for a close look at one
   * font, leaving a strip of the list (its width set by dragging the divider)
   * in view. A click in that strip only brings back the usual widths.
   */
  let wide = $state(false);
  const NARROW_DEFAULT = 320;
  const NARROW_MIN = 200;
  let narrow = $state(
    (() => {
      try {
        const n = Number(localStorage.getItem('center-narrow'));
        return Number.isFinite(n) && n >= NARROW_MIN ? n : NARROW_DEFAULT;
      } catch {
        return NARROW_DEFAULT;
      }
    })(),
  );
  function setNarrow(px: number) {
    // Leave the inspector room to be worth widening.
    const max = Math.max(NARROW_MIN, window.innerWidth - (railOpen ? 240 : 0) - 420);
    narrow = Math.round(Math.min(max, Math.max(NARROW_MIN, px)));
    try {
      localStorage.setItem('center-narrow', String(narrow));
    } catch {
      /* not persisted */
    }
  }
  /**
   * Outlines & metrics, docked under the font list as Zed docks its terminal
   * (Ctrl/⌘+J). Open or not, and its height, are remembered.
   */
  const stored = (k: string) => {
    try {
      return localStorage.getItem(k);
    } catch {
      return null;
    }
  };
  let outlineOpen = $state(stored('outline-open') === 'open');
  let outlineHeight = $state(Number(stored('outline-height')) || 300);
  function setOutlineOpen(open: boolean) {
    outlineOpen = open;
    try {
      localStorage.setItem('outline-open', open ? 'open' : 'closed');
    } catch {
      /* not persisted */
    }
  }
  function setOutlineHeight(px: number) {
    // Leave the list above some room, and the panel enough for one glyph.
    outlineHeight = Math.round(Math.min(window.innerHeight - 200, Math.max(160, px)));
    try {
      localStorage.setItem('outline-height', String(outlineHeight));
    } catch {
      /* not persisted */
    }
  }
  let googleKeySaved = $state(false);

  // Raw, not deep state: 10k faces don't need per-property proxies, and
  // replacing only the families that changed keeps every other row untouched.
  let families = $state.raw<FamilySummary[]>([]);
  let status = $state<LibraryStatus | null>(null);
  let conflicts = $state.raw<ConflictReport>({ active: [], library: [] });
  let detail = $state<FaceDetail | null>(null);
  /** The face last open in the inspector, which its toggle reopens. */
  let lastFaceId = $state<string | null>((() => {
    try {
      return localStorage.getItem('last-face');
    } catch {
      return null;
    }
  })());
  $effect(() => {
    if (!detail) {
      wide = false;
      return;
    }
    lastFaceId = detail.faceId;
    try {
      localStorage.setItem('last-face', detail.faceId);
    } catch {
      /* not persisted */
    }
  });
  let collections = $state.raw<Collection[]>([]);
  /** The collection being shown, when `filter` is 'collection'. */
  let collectionId = $state<number | null>(null);
  let tags = $state.raw<Tag[]>([]);
  /** The tag being shown, when `filter` is 'tag'. */
  let tagId = $state<number | null>(null);

  let sample = $state('');
  let size = $state(40);
  let query = $state('');
  let filter = $state<Filter>('all');
  let expanded = $state.raw(new Set<string>());

  // List or grid, remembered between launches. Storage may be unavailable.
  type Layout = 'list' | 'grid';
  let layout = $state<Layout>(
    (() => {
      try {
        return localStorage.getItem('library-layout') === 'grid' ? 'grid' : 'list';
      } catch {
        return 'list';
      }
    })(),
  );
  function setLayout(next: Layout) {
    layout = next;
    try {
      localStorage.setItem('library-layout', next);
    } catch {
      /* not persisted */
    }
  }

  // Grid geometry: as many columns as fit at a minimum card width.
  const CARD_MIN = 260;
  const GRID_GAP = 16;
  const GRID_PAD = 20;
  let listWidth = $state(0);
  const columns = $derived(
    Math.max(1, Math.floor((listWidth - GRID_PAD * 2 + GRID_GAP) / (CARD_MIN + GRID_GAP))),
  );
  const gridRows = $derived.by(() => {
    const rows: { key: string; families: FamilySummary[]; last: boolean }[] = [];
    for (let i = 0; i < shown.length; i += columns) {
      const families = shown.slice(i, i + columns);
      rows.push({ key: families[0]!.name, families, last: i + columns >= shown.length });
    }
    return rows;
  });
  /** Card height, matching FamilyCard: borders, padding, two specimen lines, footer. */
  const estimateGridRow = () => GRID_GAP + 2 + 28 + size * 2.4 + 1 + 24 + 36;

  // The family the inspector's face belongs to, for its style switcher.
  const detailFamily = $derived(
    detail ? families.find((f) => f.faces.some((x) => x.id === detail!.faceId)) : undefined,
  );
  const detailStyles = $derived(
    detailFamily?.faces.map((f) => ({ value: f.id, label: f.subfamily })) ?? [],
  );

  const inspectorShown = $derived(detail !== null && view === 'library');

  /** The font the outlines panel shows: the open one, else the last opened. */
  const outlineFace = $derived.by(() => {
    if (!outlineOpen) return null;
    const id = detail?.faceId ?? lastFaceId;
    if (!id) return null;
    for (const f of families) {
      const face = f.faces.find((x) => x.id === id);
      if (face) return { id, title: `${f.name} ${face.subfamily}`, variable: face.variable };
    }
    return null;
  });

  /** Show or hide the inspector, reopening the font last viewed. */
  async function toggleInspector() {
    if (inspectorShown) {
      detail = null;
      return;
    }
    view = 'library';
    if (detail) return;
    const known = (id: string | null) => !!id && families.some((f) => f.faces.some((x) => x.id === id));
    const id = known(lastFaceId) ? lastFaceId! : (shown[0] ?? families[0])?.representative;
    if (id) await select(id);
  }

  /** Divider between the list strip and the widened inspector. */
  function dividerDrag(e: PointerEvent) {
    if (e.button !== 0) return;
    // No text selection while dragging.
    e.preventDefault();
    const el = e.currentTarget as HTMLElement;
    const left = (el.parentElement as HTMLElement).getBoundingClientRect().left;
    el.setPointerCapture(e.pointerId);
    document.body.classList.add('fm-resizing');
    const move = (ev: PointerEvent) => setNarrow(ev.clientX - left);
    const up = () => {
      el.removeEventListener('pointermove', move);
      el.removeEventListener('pointerup', up);
      el.removeEventListener('pointercancel', up);
      document.body.classList.remove('fm-resizing');
    };
    el.addEventListener('pointermove', move);
    el.addEventListener('pointerup', up);
    el.addEventListener('pointercancel', up);
  }
  function dividerKey(e: KeyboardEvent) {
    const by = e.shiftKey ? 64 : 16;
    if (e.key === 'ArrowLeft') setNarrow(narrow - by);
    else if (e.key === 'ArrowRight') setNarrow(narrow + by);
    else if (e.key === 'Home') setNarrow(NARROW_MIN);
    else if (e.key === 'Enter') setNarrow(NARROW_DEFAULT);
    else return;
    e.preventDefault();
  }

  /** While widened, a click in the list strip only restores the widths. */
  function stripPointer(e: PointerEvent) {
    if (!wide || (e.target as Element).closest('.pane-divider')) return;
    // Nothing in the strip reacts: no drag starts, no menu opens on press.
    if (e.button === 0) e.stopPropagation();
  }
  function stripClick(e: MouseEvent) {
    if (!wide || (e.target as Element).closest('.pane-divider')) return;
    e.stopPropagation();
    e.preventDefault();
    wide = false;
  }

  /** Compare the style open in the inspector with another. */
  function compareOpen() {
    if (!detail || !detailFamily) return;
    compare.set('left', { family: detailFamily.name, faceId: detail.faceId });
    view = 'compare';
  }

  /** From a family's context menu: compare it with another of its styles. */
  function compareFamily(f: FamilySummary) {
    const face = f.faces.find((x) => x.id === f.representative) ?? f.faces[0];
    if (!face) return;
    compare.set('left', { family: f.name, faceId: face.id });
    const other = f.faces.find((x) => x.id !== face.id);
    if (other) compare.set('right', { family: f.name, faceId: other.id });
    view = 'compare';
  }

  /** From a family's context menu: try it in the style viewer. */
  function viewerFamily(role: Role, f: FamilySummary) {
    viewer.setFamily(role, f);
    view = 'viewer';
  }

  async function copyFamilyName(f: FamilySummary) {
    try {
      await navigator.clipboard.writeText(f.name);
      notice = { tone: 'info', text: `Copied “${f.name}”.` };
    } catch {
      notice = { tone: 'error', text: "Couldn't copy to the clipboard." };
    }
  }

  async function revealFamily(f: FamilySummary) {
    try {
      reveal((await api.faceDetail(f.representative)).filePath);
    } catch (e) {
      notice = { tone: 'error', text: String(e) };
    }
  }

  /** From the inspector: use the open face in the style viewer. */
  function tryInViewer(role: Role) {
    if (!detail || !detailFamily) return;
    viewer.setFont(role, { family: detailFamily.name, faceId: detail.faceId });
    view = 'viewer';
  }

  /** The folder being scanned, or '' for the whole library. Null when idle. */
  let scanning = $state<string | null>(null);
  let progress = $state<ScanProgress | null>(null);
  let busy = $state(false);
  let notice = $state<{ tone: 'info' | 'error'; text: string } | null>(null);
  let searchInput = $state<HTMLInputElement>();
  let removing = $state<RootInfo | null>(null);
  let confirmOpen = $state(false);
  let loaded = $state(false);

  // Faces taking part in a live name conflict.
  const conflictIds = $derived(new Set(conflicts.active.flatMap((c) => c.faceIds)));

  const currentCollection = $derived(
    filter === 'collection' ? collections.find((c) => c.id === collectionId) : undefined,
  );
  const familyByName = $derived(new Map(families.map((f) => [f.name, f])));
  const currentTag = $derived(filter === 'tag' ? tags.find((t) => t.id === tagId) : undefined);
  /** Tag names by lowercased family name. */
  const tagsByFamily = $derived.by(() => {
    const m = new Map<string, string[]>();
    for (const t of tags)
      for (const f of t.families) {
        const list = m.get(f);
        if (list) list.push(t.name);
        else m.set(f, [t.name]);
      }
    return m;
  });
  const tagsOf = (name: string) => tagsByFamily.get(name.toLowerCase()) ?? [];
  /** Families tagged with a tag that are in the library now. */
  const taggedCount = (t: Tag) => t.families.filter((f) => familyKeys.has(f)).length;
  const familyKeys = $derived(new Set(families.map((f) => f.name.toLowerCase())));

  const counts = $derived({
    all: families.length,
    library: families.filter((f) => !f.installed).length,
    active: families.filter((f) => !f.installed && f.activation !== 'none').length,
    variable: families.filter((f) => f.variable).length,
    conflicts: families.filter((f) => f.faces.some((x) => conflictIds.has(x.id))).length,
    installed: families.filter((f) => f.installed).length,
    collection: 0,
    tag: 0,
  });

  /**
   * A collection holds styles, not families: each family is narrowed to its
   * member styles, so its toggle and specimen act on those alone.
   */
  function inCollection(c: Collection | undefined, q: ParsedQuery): FamilySummary[] {
    if (!c) return [];
    const members = new Set(c.faceIds);
    const out: FamilySummary[] = [];
    for (const f of families) {
      if (!matches(f, q)) continue;
      const faces = f.faces.filter((x) => members.has(x.id));
      if (!faces.length) continue;
      if (faces.length === f.faces.length) {
        out.push(f);
        continue;
      }
      out.push({
        ...f,
        faces,
        activation: activationOf(faces),
        representative: faces.some((x) => x.id === f.representative) ? f.representative : faces[0]!.id,
        variable: faces.some((x) => x.variable),
        installed: faces.every((x) => x.installed),
      });
    }
    return out;
  }

  /** Search matches a family's name, its styles' names, or its tags. */
  function matches(f: FamilySummary, q: ParsedQuery): boolean {
    if (isEmptyQuery(q)) return true;
    return matchQuery(q, [f.name, ...tagsOf(f.name), ...f.faces.map((x) => x.subfamily)]);
  }

  // Search, then the sidebar's scope, then the filter bar, then the order.
  const parsed = $derived(parseQuery(query));
  const searching = $derived(!isEmptyQuery(parsed) && parsed.include.length > 0);
  const libQuery = new QueryState('library-query', { key: 'name', dir: 'asc' });
  const libFacets = $derived(libraryFacets(tagsOf));
  const libSorts = librarySorts((f) => relevance(parsed, f.name));
  const libSort = $derived(libQuery.effective(searching));
  $effect(() => {
    if (!searching) libQuery.endSearch();
  });

  /** In the sidebar's scope, before search. */
  const scoped = $derived(scope(parseQuery('')));
  /** In scope and matching the search: what the filter bar counts. */
  const searched = $derived(scope(parsed));
  /** Nothing matched exactly: names close to what was typed, typos included. */
  const close = $derived(
    searching && searched.length === 0 ? closeMatches(scoped, parsed, (f) => f.name) : [],
  );
  const shown = $derived.by(() => {
    if (close.length) return close;
    const out = applyFacets(searched, libFacets, libQuery.filters);
    return sortItems(out, libSorts, libSort, (f) => f.name);
  });

  function scope(q: ParsedQuery): FamilySummary[] {
    if (filter === 'collection') return inCollection(currentCollection, q);
    const tagged = currentTag ? new Set(currentTag.families) : null;
    return families.filter((f) => {
      if (!matches(f, q)) return false;
      switch (filter) {
        case 'tag':
          return !!tagged?.has(f.name.toLowerCase());
        case 'library':
          return !f.installed;
        case 'active':
          return !f.installed && f.activation !== 'none';
        case 'variable':
          return f.variable;
        case 'conflicts':
          return f.faces.some((x) => conflictIds.has(x.id));
        case 'installed':
          return f.installed;
        default:
          return true;
      }
    });
  }

  const libraryRoots = $derived(status?.roots.filter((r) => !r.installed) ?? []);
  const systemRoots = $derived(status?.roots.filter((r) => r.installed && r.exists) ?? []);

  const folderName = (path: string) => path.split(/[\\/]/).filter(Boolean).pop() ?? path;

  /** Collapsed row height, matching FamilyRow's layout. Expanded rows are measured. */
  const estimateRow = (f: FamilySummary) =>
    65 + size * 1.2 + (expanded.has(f.name) ? 8 + f.faces.length * (Math.min(size, 28) * 1.25 + 17) : 0);

  async function refresh() {
    const t0 = performance.now();
    const [fams, st, cf, cols, tg] = await Promise.all([
      api.listFamilies(),
      api.libraryStatus(),
      api.conflicts(),
      api.listCollections(),
      api.listTags(),
    ]);
    const t1 = performance.now();
    if (perfEnabled) mark('fetch', t1 - t0);
    setFaceSizes(fams.flatMap((f) => f.faces));
    families = fams;
    if (perfEnabled) painted().then(() => mark('render', performance.now() - t1));
    status = st;
    conflicts = cf;
    collections = cols;
    tags = tg;
    if (detail) {
      detail = await api.faceDetail(detail.faceId).catch(() => null);
    }
  }

  /**
   * Scan the whole library (incremental) or one folder (re-reads every file in
   * it). Only reloads the family list if the scan actually changed something.
   */
  async function scan(folder?: string): Promise<ScanSummary | null> {
    if (scanning !== null) return null;
    scanning = folder ?? '';
    progress = null;
    try {
      const r = await api.scanLibrary({ folder });
      if (r.filesParsed > 0 || r.removed > 0 || r.filesRemoved > 0) await refresh();
      else status = await api.libraryStatus();
      if (r.failures.length) {
        notice = {
          tone: 'error',
          text: `${r.failures.length} ${r.failures.length === 1 ? 'file' : 'files'} could not be read, for example ${r.failures[0]!.path}: ${r.failures[0]!.error}`,
        };
      }
      return r;
    } catch (e) {
      notice = { tone: 'error', text: `Scan failed: ${e}` };
      return null;
    } finally {
      scanning = null;
      progress = null;
    }
  }

  async function rescanFolder(path: string, name = folderName(path)) {
    const r = await scan(path);
    if (r && !r.failures.length) {
      notice = {
        tone: 'info',
        text: `Rescanned ${name}: ${r.filesParsed.toLocaleString()} ${r.filesParsed === 1 ? 'file' : 'files'} read${r.removed ? `, ${r.removed.toLocaleString()} missing ${r.removed === 1 ? 'style' : 'styles'} removed` : ''}.`,
      };
    }
  }

  async function rescanSystem() {
    let read = 0;
    let removed = 0;
    for (const root of systemRoots) {
      const r = await scan(root.path);
      if (!r) return;
      read += r.filesParsed;
      removed += r.removed;
    }
    notice = {
      tone: 'info',
      text: `Rescanned system fonts: ${read.toLocaleString()} files read${removed ? `, ${removed} missing styles removed` : ''}.`,
    };
  }

  async function pickAndRescan() {
    const picked = await open({
      directory: true,
      multiple: false,
      title: 'Rescan a folder',
      defaultPath: libraryRoots[0]?.path,
    });
    if (typeof picked === 'string') await rescanFolder(picked);
  }

  async function addFolder() {
    const picked = await open({ directory: true, multiple: false, title: 'Add a font folder' });
    if (typeof picked !== 'string') return;
    try {
      await api.addRoot(picked);
      filter = 'library';
      await scan(picked);
    } catch (e) {
      notice = { tone: 'error', text: String(e) };
    }
  }

  /** Family state as its on/off toggle shows it: library faces only. */
  function activationOf(faces: FaceSummary[]): FamilyActivation {
    const lib = faces.filter((f) => !f.installed);
    if (!lib.length) return 'all';
    const on = lib.filter((f) => f.active).length;
    return on === 0 ? 'none' : on === lib.length ? 'all' : 'partial';
  }

  async function setActive(ids: string[], active: boolean) {
    if (!ids.length) return;
    const t0 = performance.now();
    busy = true;
    try {
      const results = await api.setFacesActive(ids, active);
      const done = new Set(results.filter((r) => r.ok).map((r) => r.id));

      // Patch only the families that changed instead of reloading all of
      // them: at 10k faces a reload re-sends megabytes for a one-row change.
      families = families.map((f) => {
        if (!f.faces.some((x) => done.has(x.id))) return f;
        const faces = f.faces.map((x) => (done.has(x.id) ? { ...x, active } : x));
        return { ...f, faces, activation: activationOf(faces) };
      });

      const failed = results.filter((r) => !r.ok);
      if (failed.length) {
        notice = {
          tone: 'error',
          text: `${failed.length} of ${results.length} ${active ? 'could not be activated' : 'could not be deactivated'}: ${failed[0]!.error}`,
        };
      } else if (active) {
        notice = {
          tone: 'info',
          text: `Activated ${ids.length} ${ids.length === 1 ? 'style' : 'styles'}. Apps already open may need a restart to see ${ids.length === 1 ? 'it' : 'them'}.`,
        };
      } else {
        notice = null;
      }

      const [st, cf] = await Promise.all([api.libraryStatus(), api.conflicts()]);
      status = st;
      conflicts = cf;
      if (detail && done.has(detail.faceId)) detail = { ...detail, active };
    } catch (e) {
      notice = { tone: 'error', text: String(e) };
    } finally {
      busy = false;
    }
    if (perfEnabled) {
      await painted();
      mark('toggle', performance.now() - t0);
    }
  }

  const toggleFamily = (f: FamilySummary, active: boolean) =>
    setActive(
      f.faces.filter((x) => !x.installed && x.active !== active).map((x) => x.id),
      active,
    );
  const toggleFace = (f: FaceSummary, active: boolean) => setActive([f.id], active);

  async function select(faceId: string) {
    try {
      detail = await api.faceDetail(faceId);
    } catch (e) {
      notice = { tone: 'error', text: String(e) };
    }
  }

  function expand(name: string) {
    const next = new Set(expanded);
    if (next.has(name)) next.delete(name);
    else next.add(name);
    expanded = next;
  }

  function askRemove(root: RootInfo) {
    removing = root;
    confirmOpen = true;
  }

  async function removeFolder() {
    const root = removing;
    if (!root) return;
    try {
      const n = await api.removeRoot(root.path);
      await refresh();
      notice = {
        tone: 'info',
        text: `Removed ${folderName(root.path)} from your library${n ? `; ${n.toLocaleString()} ${n === 1 ? 'style' : 'styles'} no longer listed` : ''}. The files are still on disk.`,
      };
    } catch (e) {
      notice = { tone: 'error', text: String(e) };
    } finally {
      removing = null;
    }
  }

  function reveal(path: string) {
    revealItemInDir(path).catch((e) => (notice = { tone: 'error', text: `Couldn't open the folder: ${e}` }));
  }

  // -- Collections -----------------------------------------------------------

  const byName = (a: Collection, b: Collection) =>
    a.name.localeCompare(b.name, undefined, { sensitivity: 'base' });

  /** Every style of a family, even when the row shows a collection's subset. */
  const familyFaceIds = (name: string) => familyByName.get(name)?.faces.map((f) => f.id) ?? [];

  function showCollection(c: Collection) {
    filter = 'collection';
    collectionId = c.id;
    view = 'library';
  }

  /** Add faces to a collection or take them out. `what` names them in feedback. */
  async function setMembership(c: Collection, faceIds: string[], member: boolean, what: string) {
    try {
      const updated = await api.setCollectionMembers(c.id, faceIds, member);
      collections = collections.map((x) => (x.id === c.id ? updated : x));
      notice = { tone: 'info', text: member ? `Added ${what} to ${c.name}.` : `Removed ${what} from ${c.name}.` };
    } catch (e) {
      notice = { tone: 'error', text: String(e) };
      collections = await api.listCollections().catch(() => collections);
    }
  }

  interface Prompt {
    title: string;
    label: string;
    confirmLabel: string;
    value: string;
    description?: string;
    submit: (value: string) => Promise<string | null>;
  }
  let prompt = $state<Prompt | null>(null);
  let promptOpen = $state(false);
  function ask(p: Prompt) {
    prompt = p;
    promptOpen = true;
  }

  /** Create a collection, optionally holding some faces from the start. */
  function newCollection(faceIds: string[] = [], what?: string) {
    ask({
      title: 'New collection',
      label: 'Name',
      confirmLabel: 'Create collection',
      value: '',
      description: what ? `${what} will be added to it.` : 'Add styles to it from any family or the inspector.',
      submit: async (name) => {
        try {
          const c = await api.createCollection(name, faceIds);
          collections = [...collections, c].sort(byName);
          if (what) notice = { tone: 'info', text: `Added ${what} to ${c.name}.` };
          else showCollection(c);
          return null;
        } catch (e) {
          return String(e);
        }
      },
    });
  }

  function renameCollection(c: Collection) {
    ask({
      title: `Rename ${c.name}`,
      label: 'Name',
      confirmLabel: 'Rename',
      value: c.name,
      submit: async (name) => {
        try {
          await api.renameCollection(c.id, name);
          collections = await api.listCollections();
          return null;
        } catch (e) {
          return String(e);
        }
      },
    });
  }

  let deletingCollection = $state<Collection | null>(null);
  let deleteCollectionOpen = $state(false);
  async function deleteCollection() {
    const c = deletingCollection;
    if (!c) return;
    try {
      await api.deleteCollection(c.id);
      collections = collections.filter((x) => x.id !== c.id);
      if (collectionId === c.id) {
        filter = 'all';
        collectionId = null;
      }
      notice = { tone: 'info', text: `Deleted the collection ${c.name}. Its fonts are still in your library.` };
    } catch (e) {
      notice = { tone: 'error', text: String(e) };
    } finally {
      deletingCollection = null;
    }
  }

  /** Library faces in a collection that a switch would change. */
  function switchable(c: Collection, active: boolean): string[] {
    const members = new Set(c.faceIds);
    return families.flatMap((f) =>
      f.faces.filter((x) => members.has(x.id) && !x.installed && x.active !== active).map((x) => x.id),
    );
  }

  function collectionMenu(c: Collection) {
    const on = switchable(c, true).length;
    const off = switchable(c, false).length;
    return [
      { label: on ? `Activate ${on} ${on === 1 ? 'style' : 'styles'}` : 'All active', icon: 'active' as IconName, disabled: busy || !on, onSelect: () => setActive(switchable(c, true), true) },
      { label: off ? `Deactivate ${off} ${off === 1 ? 'style' : 'styles'}` : 'None active', icon: 'deactivate' as IconName, disabled: busy || !off, onSelect: () => setActive(switchable(c, false), false) },
      { label: 'Rename…', icon: 'edit' as IconName, onSelect: () => renameCollection(c) },
      {
        label: 'Delete collection',
        icon: 'trash' as IconName,
        tone: 'danger' as const,
        onSelect: () => {
          deletingCollection = c;
          deleteCollectionOpen = true;
        },
      },
    ];
  }

  // -- Tags ------------------------------------------------------------------

  const myTags = $derived(tags.filter((t) => t.source === 'user'));
  const googleTags = $derived(tags.filter((t) => t.source === 'google'));
  /** The whole Tags section folds away too, for libraries with many tags. */
  let tagsOpen = $state(
    (() => {
      try {
        return localStorage.getItem('rail-tags') !== 'closed';
      } catch {
        return true;
      }
    })(),
  );
  function setTagsOpen(open: boolean) {
    tagsOpen = open;
    try {
      localStorage.setItem('rail-tags', open ? 'open' : 'closed');
    } catch {
      /* not persisted */
    }
  }
  /** Folded, the tag being viewed stays in sight. */
  const currentTagShown = $derived(
    view === 'library' && filter === 'tag' ? tags.find((t) => t.id === tagId) : undefined,
  );

  /** Google's tags can run to dozens, so they fold away in the sidebar. */
  let googleTagsOpen = $state(
    (() => {
      try {
        return localStorage.getItem('rail-google-tags') === 'open';
      } catch {
        return false;
      }
    })(),
  );
  function setGoogleTagsOpen(open: boolean) {
    googleTagsOpen = open;
    try {
      localStorage.setItem('rail-google-tags', open ? 'open' : 'closed');
    } catch {
      /* not persisted */
    }
  }

  const tagByName = (a: Tag, b: Tag) => a.name.localeCompare(b.name, undefined, { sensitivity: 'base' });

  function showTag(t: Tag) {
    filter = 'tag';
    tagId = t.id;
    view = 'library';
  }

  /** Tag families or untag them. `what` names them in feedback. */
  async function setTagged(t: Tag, families: string[], tagged: boolean, what = families.join(', ')) {
    try {
      const updated = await api.setTagFamilies(t.id, families, tagged);
      tags = tags.map((x) => (x.id === t.id ? updated : x));
      notice = { tone: 'info', text: tagged ? `Added the tag ${t.name} to ${what}.` : `Removed the tag ${t.name} from ${what}.` };
    } catch (e) {
      notice = { tone: 'error', text: String(e) };
      tags = await api.listTags().catch(() => tags);
    }
  }

  /** Create a tag, on families if given. Resolves to an error message, or null. */
  async function createTag(name: string, families: string[] = [], what = families.join(', ')): Promise<string | null> {
    try {
      const t = await api.createTag(name, families);
      tags = [...tags, t].sort(tagByName);
      if (families.length) notice = { tone: 'info', text: `Added the tag ${t.name} to ${what}.` };
      // From the sidebar, show the new tag; from Settings, stay put.
      else if (!settingsOpen) showTag(t);
      return null;
    } catch (e) {
      return String(e);
    }
  }

  /** New tag, straight onto some families. */
  function newTagFor(families: string[], what: string) {
    ask({
      title: 'New tag',
      label: 'Name',
      confirmLabel: 'Create tag',
      value: '',
      description: `${what} will be tagged with it.`,
      submit: (name) => createTag(name, families, what),
    });
  }

  function newTag() {
    ask({
      title: 'New tag',
      label: 'Name',
      confirmLabel: 'Create tag',
      value: '',
      description: 'Tags describe typefaces: a style, a use, a client. Add one to a family from the inspector, or drag a family onto it.',
      submit: (name) => createTag(name),
    });
  }

  function renameTag(t: Tag) {
    ask({
      title: `Rename ${t.name}`,
      label: 'Name',
      confirmLabel: 'Rename',
      value: t.name,
      submit: async (name) => {
        try {
          await api.renameTag(t.id, name);
          tags = (await api.listTags()).sort(tagByName);
          return null;
        } catch (e) {
          return String(e);
        }
      },
    });
  }

  let deletingTag = $state<Tag | null>(null);
  let deleteTagOpen = $state(false);
  async function deleteTag() {
    const t = deletingTag;
    if (!t) return;
    try {
      await api.deleteTag(t.id);
      tags = tags.filter((x) => x.id !== t.id);
      if (tagId === t.id) {
        filter = 'all';
        tagId = null;
      }
      notice = { tone: 'info', text: `Deleted the tag ${t.name}. Its families are untouched.` };
    } catch (e) {
      notice = { tone: 'error', text: String(e) };
    } finally {
      deletingTag = null;
    }
  }

  function tagMenu(t: Tag) {
    return [
      { label: 'Rename…', icon: 'edit' as IconName, onSelect: () => renameTag(t) },
      {
        label: 'Delete tag',
        icon: 'trash' as IconName,
        tone: 'danger' as const,
        onSelect: () => {
          deletingTag = t;
          deleteTagOpen = true;
        },
      },
    ];
  }

  // Families dropped on a collection or tag in the sidebar.
  onDrop((target, p) => {
    const [kind, raw] = target.split(':');
    const id = Number(raw);
    if (kind === 'collection') {
      const c = collections.find((x) => x.id === id);
      if (c) setMembership(c, p.faceIds, true, p.label);
    } else if (kind === 'tag') {
      const t = tags.find((x) => x.id === id);
      if (t) setTagged(t, p.families, true, p.label);
    }
  });

  // -- Selection ---------------------------------------------------------------
  // Checkboxes select styles, for acting on many at once. Selection is never
  // activation: that's the toggle beside each name.

  let selectedFaces = $state.raw(new Set<string>());

  function selectFaces(ids: string[], on: boolean) {
    const next = new Set(selectedFaces);
    for (const id of ids) {
      if (on) next.add(id);
      else next.delete(id);
    }
    selectedFaces = next;
  }
  const clearSelection = () => (selectedFaces = new Set());
  /** Every style shown in the current view. */
  const selectShown = () => selectFaces(shown.flatMap((f) => f.faces.map((x) => x.id)), true);

  /** The selection resolved against the library: faces that exist, by family. */
  const selection = $derived.by(() => {
    const faces: FaceSummary[] = [];
    const fams: string[] = [];
    if (selectedFaces.size)
      for (const f of families) {
        const picked = f.faces.filter((x) => selectedFaces.has(x.id));
        if (picked.length) {
          faces.push(...picked);
          fams.push(f.name);
        }
      }
    return { faces, families: fams, ids: faces.map((f) => f.id) };
  });
  const selectionLabel = $derived(
    selection.families.length === 1
      ? selection.faces.length === familyByName.get(selection.families[0]!)?.faces.length
        ? selection.families[0]!
        : `${selection.faces.length} ${selection.faces.length === 1 ? 'style' : 'styles'} of ${selection.families[0]}`
      : `${selection.families.length} families`,
  );
  const shownFaceCount = $derived(shown.reduce((n, f) => n + f.faces.length, 0));
  const allShownSelected = $derived(
    shownFaceCount > 0 && shown.every((f) => f.faces.every((x) => selectedFaces.has(x.id))),
  );
  const toActivate = $derived(selection.faces.filter((f) => !f.installed && !f.active).map((f) => f.id));
  const toDeactivate = $derived(selection.faces.filter((f) => !f.installed && f.active).map((f) => f.id));

  /** Dragging a selected family carries the whole selection. */
  function dragPayload(family: FamilySummary): DragPayload {
    if (family.faces.some((f) => selectedFaces.has(f.id)) && selection.families.length) {
      return { label: selectionLabel, families: selection.families, faceIds: selection.ids };
    }
    return { label: family.name, families: [family.name], faceIds: familyFaceIds(family.name) };
  }

  // -- Command palette -----------------------------------------------------------

  const mac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform);
  /** A shortcut as the platform writes it: ⇧⌘P on macOS, Ctrl+Shift+P elsewhere. */
  const keys = (k: string, shift = false, alt = false) =>
    mac ? `${alt ? '⌥' : ''}${shift ? '⇧' : ''}⌘${k}` : `Ctrl+${alt ? 'Alt+' : ''}${shift ? 'Shift+' : ''}${k}`;

  function go(f: Filter) {
    filter = f;
    view = 'library';
  }

  const commands = $derived.by((): Command[] => {
    const c: Command[] = [];
    const add = (cmd: Command | false | null | undefined) => {
      if (cmd) c.push(cmd);
    };
    // View
    for (const f of filters) {
      if (f.key === 'conflicts' && !counts.conflicts) continue;
      add({ id: `view.${f.key}`, group: 'View', title: `Show ${f.label.toLowerCase()}`, run: () => go(f.key) });
    }
    add({ id: 'view.google', group: 'View', title: 'Open Google Fonts', keywords: 'download', run: () => (view = 'google') });
    add({ id: 'view.viewer', group: 'View', title: 'Open style viewer', keywords: 'pairing type scale', run: () => (view = 'viewer') });
    add({ id: 'view.compare', group: 'View', title: 'Compare two styles', keywords: 'side by side overlay diff', run: () => (view = 'compare') });
    add(view === 'compare' && { id: 'compare.swap', group: 'Compare', title: 'Swap the two styles', run: () => compare.swap() });
    add({ id: 'view.duplicates', group: 'View', title: 'Open duplicate files', keywords: 'copies trash', run: () => (view = 'duplicates') });
    add({ id: 'view.list', group: 'View', title: 'Use list layout', run: () => { setLayout('list'); view = 'library'; } });
    add({ id: 'view.grid', group: 'View', title: 'Use grid layout', keywords: 'cards', run: () => { setLayout('grid'); view = 'library'; } });
    add({ id: 'view.sidebar', group: 'View', title: railOpen ? 'Hide sidebar' : 'Show sidebar', shortcut: keys('B'), run: () => setRail(!railOpen) });
    add({ id: 'view.search', group: 'View', title: 'Search families', shortcut: keys('F'), run: () => { view = 'library'; setRail(true); queueMicrotask(() => searchInput?.focus()); } });
    add({ id: 'view.outlines', group: 'View', title: outlineOpen ? 'Hide outlines & metrics' : 'Show outlines & metrics', shortcut: keys('J'), keywords: 'glyph curves points baseline x-height cap height bezier', run: () => setOutlineOpen(!outlineOpen) });
    add({ id: 'view.inspector', group: 'View', title: inspectorShown ? 'Hide inspector' : 'Show inspector', shortcut: keys('B', false, true), run: toggleInspector });
    add(inspectorShown && { id: 'view.inspector-wide', group: 'View', title: wide ? 'Restore inspector width' : 'Expand inspector', run: () => (wide = !wide) });
    add({ id: 'app.settings', group: 'App', title: 'Open settings', keywords: 'preferences', shortcut: keys(','), run: () => (settingsOpen = true) });
    // Library
    add({ id: 'library.check', group: 'Library', title: 'Check for changes', keywords: 'rescan refresh scan', run: () => scan() });
    add({ id: 'library.rescan-folder', group: 'Library', title: 'Rescan a folder…', run: pickAndRescan });
    add({ id: 'library.add-folder', group: 'Library', title: 'Add a folder…', keywords: 'import fonts', run: addFolder });
    add(status && {
      id: 'library.watch',
      group: 'Library',
      title: status.watch.enabled ? 'Stop watching folders' : 'Watch folders for changes',
      run: async () => {
        try {
          await api.setWatching(!status!.watch.enabled);
          status = await api.libraryStatus();
        } catch (e) {
          notice = { tone: 'error', text: String(e) };
        }
      },
    });
    // Selection
    add(view === 'library' && { id: 'select.all', group: 'Selection', title: 'Select everything shown', shortcut: keys('A'), run: selectShown });
    if (selection.faces.length) {
      add({ id: 'select.clear', group: 'Selection', title: 'Clear selection', shortcut: 'Esc', run: clearSelection });
      add(toActivate.length > 0 && { id: 'select.activate', group: 'Selection', title: `Activate ${selectionLabel}`, run: () => setActive(toActivate, true) });
      add(toDeactivate.length > 0 && { id: 'select.deactivate', group: 'Selection', title: `Deactivate ${selectionLabel}`, run: () => setActive(toDeactivate, false) });
      add({ id: 'select.collection', group: 'Selection', title: 'New collection from selection…', run: () => newCollection(selection.ids, selectionLabel) });
      add({ id: 'select.tag', group: 'Selection', title: 'Tag selection with a new tag…', run: () => newTagFor(selection.families, selectionLabel) });
    }
    // The font open in the inspector
    if (detail) {
      const d = detail;
      const name = `${d.family} ${d.subfamily}`;
      add(!d.installed && { id: 'font.toggle', group: 'Font', title: `${d.active ? 'Deactivate' : 'Activate'} ${name}`, run: () => setActive([d.faceId], !d.active) });
      add({ id: 'font.compare', group: 'Font', title: `Compare ${name} with another style`, keywords: 'side by side', run: compareOpen });
      add({ id: 'font.heading', group: 'Font', title: `Try ${name} as headings`, keywords: 'style viewer', run: () => tryInViewer('heading') });
      add({ id: 'font.body', group: 'Font', title: `Try ${name} as body text`, keywords: 'style viewer', run: () => tryInViewer('body') });
    }
    // Collections and tags
    add({ id: 'collection.new', group: 'Collections', title: 'New collection…', run: () => newCollection() });
    for (const col of collections) add({ id: `collection.go.${col.id}`, group: 'Collections', title: `Go to ${col.name}`, run: () => showCollection(col) });
    add({ id: 'tag.new', group: 'Tags', title: 'New tag…', run: newTag });
    for (const t of tags) add({ id: `tag.go.${t.id}`, group: 'Tags', title: `Go to ${t.name}`, run: () => showTag(t) });
    // Appearance
    add({ id: 'theme.system', group: 'Theme', title: 'Follow system light or dark', keywords: 'appearance mode', run: () => theme.set('system') });
    add({ id: 'theme.light', group: 'Theme', title: 'Use light mode', keywords: 'appearance', run: () => theme.set('light') });
    add({ id: 'theme.dark', group: 'Theme', title: 'Use dark mode', keywords: 'appearance', run: () => theme.set('dark') });
    // Every theme, as Zed's theme selector lists them. Choosing one also
    // switches to its mode, unless the app is following the system.
    for (const a of ['light', 'dark'] as const)
      for (const o of theme.options(a))
        add({
          id: `theme.use.${a}.${o.id}`,
          group: 'Theme',
          title: o.id === 'default' ? `Use Font Manager ${a}` : `Use ${o.label}`,
          keywords: `${a} colours zed`,
          run: () => {
            theme.choose(a, o.id);
            if (theme.preference !== 'system') theme.set(a);
          },
        });
    add({ id: 'theme.import', group: 'Theme', title: 'Import a Zed theme…', run: () => theme.importFile() });
    add({ id: 'motion.none', group: 'Motion', title: 'Turn off animation', keywords: 'motion none reduce', run: () => motion.set('none') });
    add({ id: 'motion.snappy', group: 'Motion', title: 'Use snappy animation', keywords: 'motion quick fast', run: () => motion.set('snappy') });
    add({ id: 'motion.graceful', group: 'Motion', title: 'Use graceful animation', keywords: 'motion slow smooth', run: () => motion.set('graceful') });
    return c;
  });

  /** Families as palette results: the best few matches for a query. */
  function familyResults(q: string): Command[] {
    if (q.length < 2) return [];
    return families
      .map((f) => ({ f, m: fuzzy(q, f.name) }))
      .filter((x) => x.m)
      .sort((a, b) => b.m!.score - a.m!.score)
      .slice(0, 8)
      .map(({ f }) => ({
        id: `family.${f.name}`,
        group: 'Family',
        title: f.name,
        run: () => {
          view = 'library';
          select(f.representative);
        },
      }));
  }

  function folderMenu(root: RootInfo) {
    return [
      { label: 'Rescan', icon: 'rescan' as IconName, disabled: scanning !== null, onSelect: () => rescanFolder(root.path) },
      { label: 'Show in folder', icon: 'reveal' as IconName, onSelect: () => reveal(root.path) },
      { label: 'Remove from library', icon: 'trash' as IconName, tone: 'danger' as const, onSelect: () => askRemove(root) },
    ];
  }

  /** Global shortcuts. Ignored while typing, except Escape. */
  function onKeydown(e: KeyboardEvent) {
    const mod = e.metaKey || e.ctrlKey;
    const typing = e.target instanceof HTMLElement && e.target.closest('input, textarea, [contenteditable]');
    if (mod && e.shiftKey && e.key.toLowerCase() === 'p') {
      e.preventDefault();
      paletteOpen = !paletteOpen;
    } else if (mod && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 'j') {
      e.preventDefault();
      setOutlineOpen(!outlineOpen);
    } else if (mod && e.altKey && !e.shiftKey && e.code === 'KeyB') {
      e.preventDefault();
      toggleInspector();
    } else if (mod && !e.shiftKey && e.key.toLowerCase() === 'b') {
      e.preventDefault();
      setRail(!railOpen);
    } else if (mod && e.key.toLowerCase() === 'f') {
      e.preventDefault();
      if (view !== 'library') view = 'library';
      if (!railOpen) setRail(true);
      queueMicrotask(() => searchInput?.focus());
    } else if (e.key === '/' && !typing && !mod) {
      e.preventDefault();
      view = 'library';
      queueMicrotask(() => searchInput?.focus());
    } else if (mod && e.key === ',') {
      e.preventDefault();
      settingsOpen = true;
    } else if (mod && e.key.toLowerCase() === 'a' && !typing && view === 'library') {
      e.preventDefault();
      selectShown();
    } else if (e.key === 'Escape' && (detail || selectedFaces.size)) {
      // An open menu, select or dialog takes Escape first. Checked directly in
      // the DOM: Bits UI marks Escape as handled even when nothing is open.
      // Then the inspector closes, then the selection clears.
      const layerOpen = document.querySelector(
        '[role="menu"], [role="listbox"], [role="dialog"], [role="alertdialog"]',
      );
      if (layerOpen) return;
      // Step back one level at a time: widened, then open, then closed.
      if (wide) wide = false;
      else if (detail) detail = null;
      else clearSelection();
    }
  }

  onMount(() => {
    const unlisten = listen<ScanProgress>('scan-progress', (e) => (progress = e.payload));
    // The folder watcher found and indexed changes: show them.
    const unlistenChange = listen<ScanSummary>('library-changed', async (e) => {
      const c = e.payload;
      await refresh();
      const parts = [
        c.filesParsed ? `${c.filesParsed.toLocaleString()} new or changed ${c.filesParsed === 1 ? 'file' : 'files'}` : '',
        c.removed
          ? `${c.removed.toLocaleString()} ${c.removed === 1 ? 'style' : 'styles'} removed`
          : c.filesRemoved
            ? `${c.filesRemoved.toLocaleString()} ${c.filesRemoved === 1 ? 'file' : 'files'} removed`
            : '',
      ].filter(Boolean);
      if (scanning === null) notice = { tone: 'info', text: `Your folders changed: ${parts.join(', ')}.` };
    });
    api.googleSettings().then((g) => (googleKeySaved = g.keySaved)).catch(() => {});
    // Imported themes live in the app's data folder; the chosen ones are
    // already applied from a cache, this reconciles the list.
    theme.loadImported().catch(() => {});
    (async () => {
      await refresh();
      loaded = true;
      if (perfEnabled) {
        await painted();
        mark('firstPaint', performance.now());
        setInterval(
          () =>
            liveStatus(
              `fonts started=${loadStats.started} loaded=${loadStats.loaded} skipped=${loadStats.skipped}`,
            ),
          500,
        );
      }
      // Incremental: unchanged files are skipped, so this is quick after the first run.
      await scan();
    })();
    return () => {
      unlisten.then((f) => f());
      unlistenChange.then((f) => f());
    };
  });

  const filters: { key: Filter; label: string; icon: IconName }[] = [
    { key: 'all', label: 'All fonts', icon: 'all-fonts' },
    { key: 'library', label: 'My library', icon: 'library' },
    { key: 'active', label: 'Active', icon: 'active' },
    { key: 'variable', label: 'Variable', icon: 'variable' },
    { key: 'conflicts', label: 'Name conflicts', icon: 'warning' },
    { key: 'installed', label: 'Installed', icon: 'system' },
  ];
</script>

<svelte:window onkeydown={onKeydown} />

<Tooltip.Provider delayDuration={500}>
<div
  class="app"
  class:has-inspector={inspectorShown}
  class:wide={inspectorShown && wide}
  class:no-rail={!railOpen}
  class:has-outline={outlineOpen}
  style:--center-narrow="{narrow}px"
>
  {#if railOpen}
  <nav class="rail" aria-label="Main" transition:drawer={{ edge: 'left' }}>
    <div class="rail-search">
      <SearchField bind:value={query} bind:ref={searchInput} placeholder="Search families" label="Search families" />
    </div>

    <!-- Only the middle scrolls: search stays at the top, and the footer and
         Settings stay at the bottom however long the lists get. -->
    <div class="rail-scroll">
    <ul class="nav" aria-label="Library">
      {#each filters as f (f.key)}
        {#if f.key !== 'conflicts' || counts.conflicts > 0}
          <li>
            <button
              class="nav-item"
              class:current={view === 'library' && filter === f.key}
              class:alarm={f.key === 'conflicts'}
              aria-current={view === 'library' && filter === f.key ? 'page' : undefined}
              onclick={() => {
                filter = f.key;
                view = 'library';
              }}
            >
              <Icon name={f.icon} />
              <span class="nav-label">{f.label}</span>
              <span class="count">{counts[f.key].toLocaleString()}</span>
            </button>
          </li>
        {/if}
      {/each}
    </ul>

    <section aria-labelledby="sources-heading">
      <h2 id="sources-heading" class="group">Sources</h2>
      <ul class="nav">
        <li>
          <button
            class="nav-item"
            class:current={view === 'google'}
            aria-current={view === 'google' ? 'page' : undefined}
            onclick={() => (view = 'google')}
          >
            <Icon name="globe" />
            <span class="nav-label">Google Fonts</span>
            {#if !googleKeySaved}<span class="tag">Set up</span>{/if}
          </button>
        </li>
      </ul>
    </section>

    <section aria-labelledby="tools-heading">
      <h2 id="tools-heading" class="group">Tools</h2>
      <ul class="nav">
        <li>
          <button
            class="nav-item"
            class:current={view === 'viewer'}
            aria-current={view === 'viewer' ? 'page' : undefined}
            onclick={() => (view = 'viewer')}
          >
            <Icon name="style-viewer" />
            <span class="nav-label">Style viewer</span>
          </button>
        </li>
        <li>
          <button
            class="nav-item"
            class:current={view === 'compare'}
            aria-current={view === 'compare' ? 'page' : undefined}
            onclick={() => (view = 'compare')}
          >
            <Icon name="compare" />
            <span class="nav-label">Compare</span>
          </button>
        </li>
        <li>
          <button
            class="nav-item"
            class:current={view === 'duplicates'}
            aria-current={view === 'duplicates' ? 'page' : undefined}
            onclick={() => (view = 'duplicates')}
          >
            <Icon name="duplicates" />
            <span class="nav-label">Duplicate files</span>
            {#if status?.duplicateFiles}<span class="count">{status.duplicateFiles.toLocaleString()}</span>{/if}
          </button>
        </li>
      </ul>
    </section>

    <section aria-labelledby="collections-heading">
      <div class="group-head">
        <h2 id="collections-heading" class="group">Collections</h2>
        <IconButton icon="bookmark-plus" size="sm" label="New collection" onclick={() => newCollection()} />
      </div>
      {#if collections.length}
        <ul class="folders">
          {#each collections as c (c.id)}
            {@const current = view === 'library' && filter === 'collection' && collectionId === c.id}
            <li class="collection" class:current data-drop="collection:{c.id}">
              <button
                class="collection-btn"
                aria-current={current ? 'page' : undefined}
                onclick={() => showCollection(c)}
              >
                <Icon name="bookmark" size={14} />
                <span class="folder-name">{c.name}</span>
                <span class="count">{c.faceIds.length.toLocaleString()}</span>
              </button>
              <span class="folder-menu"><Menu label="Actions for {c.name}" items={collectionMenu(c)} /></span>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="hint">Group styles you use together, then switch them on and off at once.</p>
      {/if}
    </section>

    <section aria-labelledby="tags-heading">
      <div class="group-head">
        <h2 id="tags-heading" class="group">
          <button
            class="group-toggle"
            aria-expanded={tagsOpen}
            aria-controls="tags-list"
            data-spring={tagsOpen ? undefined : ''}
            onclick={() => setTagsOpen(!tagsOpen)}
          >
            Tags
            <Icon name={tagsOpen ? 'chevron-down' : 'chevron-right'} size={12} />
          </button>
        </h2>
        <IconButton icon="tag-plus" size="sm" label="New tag" onclick={newTag} />
      </div>
      {#snippet tagItem(t: Tag)}
        {@const current = view === 'library' && filter === 'tag' && tagId === t.id}
        <li class="collection" class:current data-drop="tag:{t.id}">
          <button class="collection-btn" aria-current={current ? 'page' : undefined} onclick={() => showTag(t)}>
            <Icon name="tag" size={14} />
            <span class="folder-name">{t.name}</span>
            <span class="count">{taggedCount(t).toLocaleString()}</span>
          </button>
          <span class="folder-menu"><Menu label="Actions for the tag {t.name}" items={tagMenu(t)} /></span>
        </li>
      {/snippet}
      <div id="tags-list" hidden={!tagsOpen}>
        {#if tags.length}
          {#if myTags.length}
            <ul class="folders">
              {#each myTags as t (t.id)}{@render tagItem(t)}{/each}
            </ul>
          {/if}
          {#if googleTags.length}
            <button
              class="disclosure"
              aria-expanded={googleTagsOpen}
              onclick={() => setGoogleTagsOpen(!googleTagsOpen)}
            >
              <Icon name={googleTagsOpen ? 'chevron-down' : 'chevron-right'} size={12} />
              From Google Fonts
              <span class="count">{googleTags.length}</span>
            </button>
            {#if googleTagsOpen}
              <ul class="folders">
                {#each googleTags as t (t.id)}{@render tagItem(t)}{/each}
              </ul>
            {/if}
          {/if}
        {:else}
          <p class="hint">Describe families in your own words, then find them by tag or in search.</p>
        {/if}
      </div>
      {#if !tagsOpen && currentTagShown}
        <ul class="folders">{@render tagItem(currentTagShown)}</ul>
      {/if}
    </section>

    <section aria-labelledby="folders-heading">
      <div class="group-head">
        <h2 id="folders-heading" class="group">Folders</h2>
        <IconButton icon="folder-plus" size="sm" label="Add a folder of fonts" onclick={addFolder} />
        <Menu
          label="More folder actions"
          items={[{ label: 'Rescan a folder…', icon: 'rescan-pick', disabled: scanning !== null, onSelect: pickAndRescan }]}
        />
      </div>
      <ul class="folders">
        {#each libraryRoots as r (r.path)}
          <li class:missing={!r.exists}>
            <Icon name="folder" size={14} />
            <span class="folder-name" title={r.path}>{folderName(r.path)}</span>
            {#if scanning === r.path}
              <span class="busy">Scanning…</span>
            {:else if !r.exists}
              <span class="busy">Not found</span>
            {/if}
            <span class="folder-menu"><Menu label="Actions for {folderName(r.path)}" items={folderMenu(r)} /></span>
          </li>
        {/each}
        {#if systemRoots.length}
          <li>
            <Icon name="system" size={14} />
            <span class="folder-name" title={systemRoots.map((r) => r.path).join('\n')}>System fonts</span>
            {#if systemRoots.some((r) => r.path === scanning)}<span class="busy">Scanning…</span>{/if}
            <span class="folder-menu">
              <Menu
                label="Actions for system fonts"
                items={[{ label: 'Rescan', icon: 'rescan', disabled: scanning !== null, onSelect: rescanSystem }]}
              />
            </span>
          </li>
        {/if}
      </ul>
      {#if libraryRoots.length === 0}
        <p class="hint">Add a folder to manage its fonts here. Fonts in your library can be switched on and off.</p>
      {/if}
    </section>

    </div>

  </nav>
  {/if}

  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <main onpointerdowncapture={stripPointer} onclickcapture={stripClick}>
    {#if inspectorShown && wide}
      <!-- A focusable separator is the ARIA window-splitter pattern. -->
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <div
        class="pane-divider"
        role="separator"
        aria-orientation="vertical"
        aria-label="Width of the font list"
        aria-valuenow={narrow}
        aria-valuemin={NARROW_MIN}
        tabindex="0"
        onpointerdown={dividerDrag}
        ondblclick={() => setNarrow(NARROW_DEFAULT)}
        onkeydown={dividerKey}
      ></div>
    {/if}
    {#if view === 'compare'}
      <Compare
        {families}
        preferred={detail && detailFamily ? { family: detailFamily.name, faceId: detail.faceId } : undefined}
      />
    {:else if view === 'duplicates'}
      <Duplicates onChanged={refresh} />
    {:else if view === 'viewer'}
      <StyleViewer {families} fallback={detailFamily} />
    {:else if view === 'google'}
      <GoogleFonts
        keySaved={googleKeySaved}
        onOpenSettings={() => (settingsOpen = true)}
        onDownloaded={refresh}
        library={families}
        {busy}
        onToggleFamily={toggleFamily}
        bind:sample
        bind:size
        {layout}
        onLayout={setLayout}
      />
    {:else}
      <div class="tester">
        <label class="sample">
          <span class="visually-hidden">Sample text</span>
          <input
            type="text"
            placeholder="Type to preview. Empty shows each family's name."
            bind:value={sample}
            spellcheck="false"
          />
        </label>
        <Segmented
          label="Layout"
          iconOnly
          items={[
            { value: 'list', label: 'List', icon: 'list' },
            { value: 'grid', label: 'Grid', icon: 'grid' },
          ]}
          value={layout}
          onValueChange={setLayout}
        />
        <div class="size">
          <Slider bind:value={size} min={12} max={120} label="Preview size" />
          <NumberField bind:value={size} min={12} max={120} unit="px" label="Preview size" />
        </div>
      </div>

      <FilterBar
        items={searched}
        facets={libFacets}
        query={libQuery}
        sorts={libSorts}
        sort={libSort}
        onSort={(x) => libQuery.pick(x, searching)}
        {searching}
        shown={shown.length}
        noun={['family', 'families']}
      />

      {#if close.length}
        <Callout>
          No family matches “{query.trim()}”. Showing names close to it.
          {#snippet action()}
            <Button size="sm" variant="ghost" onclick={() => (query = '')}>Clear search</Button>
          {/snippet}
        </Callout>
      {/if}

      {#if notice}
        <Callout tone={notice.tone} onDismiss={() => (notice = null)}>{notice.text}</Callout>
      {/if}

      {#if currentCollection}
        {@const c = currentCollection}
        {@const on = switchable(c, true)}
        {@const off = switchable(c, false)}
        <div class="collection-bar">
          <Icon name="bookmark" />
          <h2>{c.name}</h2>
          <span class="collection-meta">
            {c.faceIds.length.toLocaleString()} {c.faceIds.length === 1 ? 'style' : 'styles'}
          </span>
          <span class="spacer"></span>
          {#if off.length}
            <Button size="sm" variant="outline" disabled={busy} onclick={() => setActive(off, false)}>
              Deactivate all
            </Button>
          {/if}
          {#if on.length}
            <Button size="sm" variant="solid" tone="accent" disabled={busy} onclick={() => setActive(on, true)}>
              Activate all
            </Button>
          {/if}
          <Menu label="Actions for {c.name}" items={collectionMenu(c).slice(2)} />
        </div>
      {/if}

      {#if currentTag}
        {@const t = currentTag}
        <div class="collection-bar">
          <Icon name="tag" />
          <h2>{t.name}</h2>
          <span class="collection-meta">
            {taggedCount(t).toLocaleString()} {taggedCount(t) === 1 ? 'family' : 'families'}
          </span>
          <span class="spacer"></span>
          <Menu label="Actions for the tag {t.name}" items={tagMenu(t)} />
        </div>
      {/if}

      {#if selection.faces.length}
        <div class="selection-bar" role="region" aria-label="Selection">
          <Checkbox
            checked={allShownSelected}
            indeterminate={!allShownSelected}
            label={allShownSelected ? 'Clear the selection' : 'Select everything shown'}
            onCheckedChange={(on) => (on ? selectShown() : clearSelection())}
          />
          <span class="selection-count">
            {selection.faces.length.toLocaleString()}
            {selection.faces.length === 1 ? 'style' : 'styles'} in
            {selection.families.length.toLocaleString()}
            {selection.families.length === 1 ? 'family' : 'families'} selected
          </span>
          <Button
            size="sm"
            variant="solid"
            tone="accent"
            disabled={busy || !toActivate.length}
            onclick={() => setActive(toActivate, true)}
          >
            Activate{toActivate.length && toActivate.length !== selection.faces.length ? ` ${toActivate.length}` : ''}
          </Button>
          <Button
            size="sm"
            variant="outline"
            disabled={busy || !toDeactivate.length}
            onclick={() => setActive(toDeactivate, false)}
          >
            Deactivate{toDeactivate.length && toDeactivate.length !== selection.faces.length ? ` ${toDeactivate.length}` : ''}
          </Button>
          <CollectionMenu
            {collections}
            faceIds={selection.ids}
            label="Add the selection to a collection"
            text="Collection"
            onToggle={(c, member) => setMembership(c, selection.ids, member, selectionLabel)}
            onCreate={() => newCollection(selection.ids, selectionLabel)}
          />
          <TagMenu
            {tags}
            families={selection.families}
            label="Tag the selected families"
            text="Tag"
            onToggle={(t, on) => setTagged(t, selection.families, on, selectionLabel)}
            onCreate={() => newTagFor(selection.families, selectionLabel)}
          />
          <Button size="sm" variant="ghost" onclick={clearSelection}>Clear</Button>
        </div>
      {/if}

      {#snippet familyActions(family: FamilySummary)}
        {@const ids = familyFaceIds(family.name)}
        <CollectionMenu
          {collections}
          faceIds={ids}
          label="Add {family.name} to a collection"
          onToggle={(c, member) => setMembership(c, ids, member, family.name)}
          onCreate={() => newCollection(ids, family.name)}
        />
      {/snippet}

      {#if shown.length}
        <FamilyContextMenu
          familyOf={(n) => familyByName.get(n)}
          {busy}
          canExpand={layout === 'list'}
          isExpanded={(n) => expanded.has(n)}
          onExpand={expand}
          {selectedFaces}
          onSelectFaces={selectFaces}
          onOpen={select}
          onToggleFamily={toggleFamily}
          {collections}
          onCollection={setMembership}
          onNewCollection={newCollection}
          {tags}
          onTag={setTagged}
          onNewTag={newTagFor}
          onCompare={compareFamily}
          onViewer={viewerFamily}
          onCopyName={copyFamilyName}
          onReveal={revealFamily}
          collection={currentCollection}
        >
          <div class="results" bind:clientWidth={listWidth}>
            {#if layout === 'list'}
              <VirtualList
                items={shown}
                key={(f) => f.name}
                estimate={estimateRow}
                resetScroll={`${filter}|${query}|list|${JSON.stringify(libQuery.filters)}|${libSort.key}${libSort.dir}`}
                resetMeasure={size}
                label="Font families"
              >
                {#snippet row(family)}
                  <FamilyRow
                    {family}
                    {sample}
                    {size}
                    {busy}
                    {conflictIds}
                    expanded={expanded.has(family.name)}
                    selectedFaceId={detail?.faceId ?? null}
                    onToggleFamily={toggleFamily}
                    onToggleFace={toggleFace}
                    onSelect={select}
                    onExpand={expand}
                    actions={familyActions}
                    {selectedFaces}
                    onSelectFaces={selectFaces}
                    {dragPayload}
                    tags={tagsOf(family.name)}
                  />
                {/snippet}
              </VirtualList>
            {:else}
              <VirtualList
                items={gridRows}
                key={(r) => `${columns}:${r.key}`}
                estimate={estimateGridRow}
                resetScroll={`${filter}|${query}|grid|${JSON.stringify(libQuery.filters)}|${libSort.key}${libSort.dir}`}
                resetMeasure={`${size}|${columns}`}
                label="Font families"
              >
                {#snippet row(r)}
                  <div
                    class="grid-row"
                    class:last={r.last}
                    style:grid-template-columns="repeat({columns}, minmax(0, 1fr))"
                  >
                    {#each r.families as family (family.name)}
                      <FamilyCard
                        {family}
                        {sample}
                        {size}
                        {busy}
                        current={family.faces.some((f) => f.id === detail?.faceId)}
                        conflict={family.faces.some((f) => conflictIds.has(f.id))}
                        onToggle={toggleFamily}
                        onSelect={select}
                        actions={familyActions}
                        {selectedFaces}
                        onSelectFaces={selectFaces}
                        {dragPayload}
                        tags={tagsOf(family.name)}
                      />
                    {/each}
                  </div>
                {/snippet}
              </VirtualList>
            {/if}
          </div>
        </FamilyContextMenu>
      {:else}
        <div class="empty">
          {#if !loaded || (scanning !== null && families.length === 0)}
            <p>Indexing your fonts. Large libraries take a few seconds the first time.</p>
          {:else if filter === 'library' && libraryRoots.length === 0}
            <span class="empty-icon"><Icon name="library" size={20} /></span>
            <h2>Your library is empty</h2>
            <p>Add a folder of fonts to switch them on and off from here.</p>
            <Button variant="solid" tone="accent" icon="folder-plus" onclick={addFolder}>Add folder…</Button>
          {:else if filter === 'tag' && currentTag && !query}
            <span class="empty-icon"><Icon name="tag" size={20} /></span>
            <h2>No families tagged {currentTag.name}</h2>
            <p>Add the tag from a family's inspector, or drag a family onto the tag in the sidebar.</p>
            <Button variant="ghost" onclick={() => (filter = 'all')}>Browse all fonts</Button>
          {:else if filter === 'collection' && currentCollection && !query}
            <span class="empty-icon"><Icon name="bookmark" size={20} /></span>
            <h2>Nothing in {currentCollection.name} yet</h2>
            <p>
              Drag a family onto the collection in the sidebar, use the bookmark button on its row,
              or add a single style from the inspector.
            </p>
            <Button variant="ghost" onclick={() => (filter = 'all')}>Browse all fonts</Button>
          {:else if searched.length && !shown.length}
            <p>
              {searched.length.toLocaleString()}
              {searched.length === 1 ? 'family is' : 'families are'} here, but the filters hide
              {searched.length === 1 ? 'it' : 'all of them'}.
            </p>
            <Button variant="ghost" onclick={() => libQuery.clear()}>Clear filters</Button>
          {:else if query}
            <p>No family matches “{query}”.</p>
            <Button variant="ghost" onclick={() => (query = '')}>Clear search</Button>
          {:else if filter === 'active'}
            <span class="empty-icon"><Icon name="active" size={20} /></span>
            <h2>Nothing active yet</h2>
            <p>Switch a family on with the circle beside its name in My library.</p>
          {:else}
            <p>No fonts here yet.</p>
          {/if}
        </div>
      {/if}
    {/if}
  </main>

  {#if detail && view === 'library'}
    <div class="inspector-slot" transition:drawer={{ edge: 'right' }}>
    <Inspector
      face={detail}
      {sample}
      {busy}
      conflict={conflictIds.has(detail.faceId)}
      styles={detailStyles}
      onSelectStyle={select}
      onTryInViewer={tryInViewer}
      onCompare={compareOpen}
      onToggle={(active) => detail && setActive([detail.faceId], active)}
      onClose={() => (detail = null)}
      {collections}
      onCollection={(c, member) =>
        detail && setMembership(c, [detail.faceId], member, `${detail.family} ${detail.subfamily}`)}
      onNewCollection={() =>
        detail && newCollection([detail.faceId], `${detail.family} ${detail.subfamily}`)}
      familyName={detailFamily?.name ?? detail.family}
      {tags}
      onTag={(t, tagged) => setTagged(t, [detailFamily?.name ?? detail!.family], tagged)}
      onCreateTag={(name) => createTag(name, [detailFamily?.name ?? detail!.family])}
    />
    </div>
  {/if}

  {#if outlineOpen}
    <div class="panel-slot" transition:drawer={{ edge: 'bottom' }}>
    <OutlinePanel
      faceId={outlineFace?.id ?? null}
      title={outlineFace?.title ?? ''}
      variable={outlineFace?.variable ?? false}
      height={outlineHeight}
      onResize={setOutlineHeight}
      onClose={() => setOutlineOpen(false)}
    />
    </div>
  {/if}

  <!-- Zed-style status bar: what the app is doing, and the counts, always in
       view; with the sidebar toggle, palette and settings. -->
  <footer class="status-bar">
    <IconButton
      icon={railOpen ? 'sidebar-open' : 'sidebar'}
      size="sm"
      label="{railOpen ? 'Hide' : 'Show'} sidebar ({keys('B')})"
      pressed={railOpen}
      onclick={() => setRail(!railOpen)}
    />
    <IconButton
      icon={outlineOpen ? 'panel-bottom-open' : 'panel-bottom'}
      size="sm"
      label="{outlineOpen ? 'Hide' : 'Show'} outlines & metrics ({keys('J')})"
      pressed={outlineOpen}
      onclick={() => setOutlineOpen(!outlineOpen)}
    />
    {#if scanning !== null}
      <span class="status-item" role="status">
        {#if progress && progress.toRead > 0}
          Reading {progress.read.toLocaleString()} of {progress.toRead.toLocaleString()}
          {progress.toRead === 1 ? 'file' : 'files'}
          <progress max={progress.toRead} value={progress.read}></progress>
        {:else}
          Checking {scanning ? folderName(scanning) : 'for changes'}…
        {/if}
      </span>
    {:else if status}
      <button class="status-item link" onclick={() => (settingsOpen = true)}>
        <Icon name={status.watch.enabled && !status.watch.error ? 'eye' : 'eye-off'} size={14} />
        {#if !status.watch.enabled}
          Not watching folders
        {:else if status.watch.error}
          Some folders not watched
        {:else}
          Watching {status.watch.folders} {status.watch.folders === 1 ? 'folder' : 'folders'}
        {/if}
      </button>
      <IconButton icon="rescan" size="sm" label="Check for changes" onclick={() => scan()} />
    {/if}
    <span class="spacer"></span>
    {#if status}
      <button class="status-item link" onclick={() => go('all')}>
        {status.faceCount.toLocaleString()} styles
      </button>
      <button class="status-item link" onclick={() => go('active')}>
        <Icon name="active" size={14} />
        {status.activeCount.toLocaleString()} active
      </button>
      {#if status.duplicateFiles}
        <button class="status-item link" onclick={() => (view = 'duplicates')}>
          <Icon name="duplicates" size={14} />
          {status.duplicateFiles.toLocaleString()}
          {status.duplicateFiles === 1 ? 'duplicate' : 'duplicates'}
        </button>
      {/if}
    {/if}
    <IconButton icon="command" size="sm" label="Command palette ({keys('P', true)})" onclick={() => (paletteOpen = true)} />
    <IconButton icon="gear" size="sm" label="Settings ({keys(',')})" onclick={() => (settingsOpen = true)} />
    <span class="panes">
      {#if inspectorShown}
        <span class="expand" class:wide>
          <IconButton
            icon="expand"
            size="sm"
            label={wide ? 'Restore inspector width' : 'Expand inspector'}
            onclick={() => (wide = !wide)}
          />
        </span>
      {/if}
      <IconButton
        icon={inspectorShown ? 'sidebar-right-open' : 'sidebar-right'}
        size="sm"
        label="{inspectorShown ? 'Hide' : 'Show'} inspector ({keys('B', false, true)})"
        pressed={inspectorShown}
        disabled={!families.length}
        onclick={toggleInspector}
      />
    </span>
  </footer>
</div>
</Tooltip.Provider>

<CommandPalette bind:open={paletteOpen} {commands} extra={familyResults} />

<ConfirmDialog
  bind:open={confirmOpen}
  title={removing ? `Remove ${folderName(removing.path)} from your library?` : ''}
  confirmLabel="Remove folder"
  onConfirm={removeFolder}
>
  Its fonts will no longer be listed here. The files stay on disk, and you can add the folder
  again at any time.
</ConfirmDialog>

<ConfirmDialog
  bind:open={deleteCollectionOpen}
  title={deletingCollection ? `Delete the collection ${deletingCollection.name}?` : ''}
  confirmLabel="Delete collection"
  onConfirm={deleteCollection}
>
  Only the collection goes. Its fonts stay in your library, and any that are active stay active.
</ConfirmDialog>

<SettingsWindow bind:open={settingsOpen}>
  <Settings
    onKeyChanged={(g) => (googleKeySaved = g.keySaved)}
    {tags}
    tagCount={taggedCount}
    onNewTag={newTag}
    onRenameTag={renameTag}
    onDeleteTag={(t) => {
      deletingTag = t;
      deleteTagOpen = true;
    }}
    onTagsChanged={async () => (tags = await api.listTags())}
  />
</SettingsWindow>

<ConfirmDialog
  bind:open={deleteTagOpen}
  title={deletingTag ? `Delete the tag ${deletingTag.name}?` : ''}
  confirmLabel="Delete tag"
  onConfirm={deleteTag}
>
  The tag comes off every family that has it. The fonts themselves are untouched.
</ConfirmDialog>

{#if prompt}
  <PromptDialog
    bind:open={promptOpen}
    title={prompt.title}
    label={prompt.label}
    confirmLabel={prompt.confirmLabel}
    value={prompt.value}
    description={prompt.description}
    onSubmit={prompt.submit}
  />
{/if}

<style>
  /* Columns: sidebar, list, inspector. The side columns are `auto`, sized by
     their panes, so a pane sliding in or out (lib/motion.svelte.ts) carries
     its column with it and the list follows smoothly. Rows: the panes, the
     outlines panel under the list (when open), the status bar. The side
     panes span both upper rows, so the panel sits under the list alone, as
     Zed's bottom dock sits under the editor. */
  .app {
    --main-rule: var(--pane-rule-width);
    --rail-w: var(--rail);
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    grid-template-rows: minmax(0, 1fr) auto auto;
    height: 100%;
    /* Panes slide past the window's edges. */
    overflow: hidden;
  }
  .app.no-rail {
    --main-rule: 0px;
    --rail-w: 0px;
  }
  .app > :global(.rail) {
    grid-column: 1;
    grid-row: 1 / 3;
    width: var(--rail);
  }
  .app > main {
    grid-column: 2;
    grid-row: 1;
  }
  .panel-slot {
    grid-column: 2;
    grid-row: 2;
    min-width: 0;
  }
  .app.no-rail :global(.panel) {
    border-left: 0;
  }
  .inspector-slot {
    grid-column: 3;
    grid-row: 1 / 3;
    display: flex;
    min-height: 0;
    width: var(--inspector);
    /* Widening and restoring glide at the pane speed. */
    transition: width var(--dur-pane) var(--ease);
  }
  .inspector-slot > :global(.inspector) {
    flex: 1;
    min-width: 0;
  }
  /* Widened: the list keeps a strip, the inspector takes the rest. */
  .app.wide .inspector-slot {
    width: calc(100vw - var(--rail-w) - var(--center-narrow));
  }
  /* While a divider is dragged the width follows the pointer exactly. */
  :global(body.fm-resizing) .inspector-slot {
    transition: none;
  }
  .pane-divider {
    position: absolute;
    top: 0;
    bottom: 0;
    /* From the list's left edge, so it sits on the boundary whether the
       inspector is a column or, in small windows, an overlay. Just past the
       edge, over the inspector's margin, so the list's scrollbar stays
       grabbable. */
    left: calc(var(--center-narrow) - var(--main-rule));
    width: 7px;
    z-index: calc(var(--z-panel) + 1);
    cursor: col-resize;
    touch-action: none;
  }
  .pane-divider::after {
    content: '';
    position: absolute;
    inset: 0 5px 0 0;
    background: transparent;
    transition: background-color var(--dur-fast) var(--ease);
  }
  .pane-divider:hover::after,
  .pane-divider:focus-visible::after,
  :global(body.fm-resizing) .pane-divider::after {
    background: var(--focus);
  }
  .pane-divider:focus-visible {
    outline: none;
  }
  :global(body.fm-resizing) {
    cursor: col-resize;
    user-select: none;
  }
  /* The strip is only for glancing at: hover effects would suggest its rows
     do something, when a click there just brings the list back. */
  .app.wide main :global(.family),
  .app.wide main :global(.card) {
    pointer-events: none;
  }
  .app.wide main {
    cursor: pointer;
  }

  /* The right pane's icons, at the far right as the sidebar's is at the left. */
  .panes {
    display: flex;
    align-items: center;
    gap: 2px;
    margin-left: var(--s-2);
    padding-left: var(--s-2);
    border-left: 1px solid var(--faint);
  }
  .expand :global(svg) {
    transition: transform var(--dur) var(--ease);
  }
  .expand.wide :global(svg) {
    transform: rotate(180deg);
  }

  /* Status bar ----------------------------------------------------------- */
  .status-bar {
    grid-column: 1 / -1;
    grid-row: 3;
    /* The bottom panel slides out beneath it. */
    position: relative;
    z-index: 2;
    display: flex;
    align-items: center;
    gap: var(--s-1);
    height: 28px;
    padding: 0 var(--s-2);
    border-top: 1px solid var(--faint);
    background: var(--lightbox);
    color: var(--muted);
    font-size: var(--text-1);
  }
  /* Pane toggles show their state in the icon itself, a filled slice when
     open, so no pressed background on top; they keep the brighter ink. */
  .status-bar :global(.fm-icon-btn[aria-pressed='true']:not(:hover)) {
    background: none;
  }
  .status-item {
    display: inline-flex;
    align-items: center;
    gap: var(--s-1);
    height: 22px;
    padding: 0 var(--s-2);
    border-radius: var(--r-1);
    white-space: nowrap;
    color: inherit;
  }
  .status-item.link:hover {
    background: var(--hover);
    color: var(--ink);
  }
  .status-item progress {
    width: 120px;
    height: 4px;
    margin-left: var(--s-2);
    accent-color: var(--cyan-solid);
  }

  /* Rail ----------------------------------------------------------------- */
  .rail {
    display: flex;
    flex-direction: column;
    gap: var(--s-3);
    padding: var(--s-3) var(--s-2) var(--s-2);
    min-height: 0;
    overflow: hidden;
  }
  /* The scroll area runs to the rail's edges, so its scrollbar sits at the
     edge and focus rings aren't clipped; the padding restores the inset. */
  .rail-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: var(--s-4);
    margin: 0 calc(var(--s-2) * -1);
    padding: var(--s-1) var(--s-2);
  }
  .rail-search {
    padding: 0 var(--s-1);
  }
  .nav {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 1px;
  }
  .nav-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: var(--s-2);
    height: 30px;
    padding: 0 var(--s-2);
    border-radius: var(--r-2);
    color: var(--ink);
    text-align: left;
  }
  .nav-item :global(svg) {
    color: var(--muted);
  }
  .nav-item:hover {
    background: var(--pressed);
  }
  .nav-item.current {
    background: var(--sheet);
    font-weight: 500;
    box-shadow: 0 1px 2px rgb(0 0 0 / 0.06);
  }
  .nav-item.current :global(svg) {
    color: var(--cyan);
  }
  .nav-item.alarm,
  .nav-item.alarm :global(svg) {
    color: var(--magenta);
  }
  .nav-label {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .count {
    color: var(--muted);
    font-size: var(--text-1);
    font-weight: 400;
  }
  .tag {
    font-size: var(--text-1);
    color: var(--cyan);
    font-weight: 500;
  }

  .group {
    font-size: var(--text-1);
    font-weight: var(--weight-strong);
    color: var(--muted);
    margin: 0 0 var(--s-1);
    padding: 0 var(--s-2);
  }
  .group-head {
    display: flex;
    align-items: center;
    gap: 2px;
    padding-right: var(--s-1);
    margin-bottom: var(--s-1);
  }
  .group-head .group {
    flex: 1;
    margin: 0;
  }
  /* A heading that folds its section. The chevron sits after the name, so
     the name lines up with headings that don't fold. */
  .group:has(.group-toggle) {
    padding: 0;
  }
  .group-toggle {
    display: flex;
    align-items: center;
    gap: var(--s-1);
    width: 100%;
    height: 24px;
    padding: 0 var(--s-1) 0 var(--s-2);
    border-radius: var(--r-2);
    color: inherit;
    font: inherit;
    text-align: left;
  }
  .group-toggle:hover,
  .group-toggle:global([data-spring-over]) {
    background: var(--pressed);
    color: var(--ink);
  }

  .folders {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 1px;
  }
  .folders li {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    height: 30px;
    padding: 0 var(--s-1) 0 var(--s-2);
    border-radius: var(--r-2);
    color: var(--ink);
  }
  .folders li > :global(svg) {
    color: var(--muted);
  }
  .folders li:hover,
  .folders li:focus-within {
    background: var(--pressed);
  }
  .folders li.missing {
    color: var(--muted);
  }
  .folder-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .busy {
    font-size: var(--text-1);
    color: var(--muted);
    white-space: nowrap;
  }
  /* The menu button is revealed on hover or focus so the list stays calm,
     and is always visible on touch screens. */
  .folder-menu {
    opacity: 0;
  }
  .folders li:hover .folder-menu,
  .folders li:focus-within .folder-menu {
    opacity: 1;
  }
  @media (hover: none) {
    .folder-menu {
      opacity: 1;
    }
  }
  /* A collection or tag row: one hover surface holding the link and its
     menu. The menu sits over the count rather than beside it, so every count
     in the sidebar shares one right edge; the count steps aside while the
     menu is showing. */
  .folders li.collection {
    position: relative;
    padding: 0;
  }
  .collection .folder-menu {
    position: absolute;
    right: var(--s-1);
    top: 50%;
    translate: 0 -50%;
  }
  .collection:hover .count,
  .collection:focus-within .count,
  .collection:has(:global([data-state='open'])) .count {
    visibility: hidden;
  }
  .folders li:has(:global([data-state='open'])) .folder-menu {
    opacity: 1;
  }
  .folders li.collection.current {
    background: var(--sheet);
    box-shadow: 0 1px 2px rgb(0 0 0 / 0.06);
  }
  .collection.current .collection-btn {
    font-weight: 500;
  }
  .collection.current .collection-btn :global(svg) {
    color: var(--cyan);
  }
  .collection-btn {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: var(--s-2);
    height: 30px;
    padding: 0 var(--s-2);
    border-radius: var(--r-2);
    text-align: left;
    color: var(--ink);
  }
  /* Touch screens always show the menu, so leave it room. */
  @media (hover: none) {
    .collection-btn {
      padding-right: 36px;
    }
  }
  .collection-btn :global(svg) {
    color: var(--muted);
    flex: none;
  }
  /* Bulk actions for the checked styles. Floats over the bottom of the list
     rather than pushing it down, so nothing moves under the pointer when the
     first box is ticked. */
  .selection-bar {
    position: absolute;
    left: 50%;
    bottom: var(--s-5);
    translate: -50% 0;
    z-index: var(--z-bar);
    display: flex;
    align-items: center;
    gap: var(--s-2);
    max-width: calc(100% - 2 * var(--s-5));
    padding: var(--s-2) var(--s-2) var(--s-2) var(--s-4);
    border-radius: var(--r-3);
    background: var(--raised);
    box-shadow: var(--shadow-float);
    animation: float-in var(--dur) var(--ease);
  }
  .selection-count {
    margin: 0 var(--s-2) 0 var(--s-1);
    white-space: nowrap;
    font-weight: 500;
  }
  .collection-bar {
    display: flex;
    align-items: center;
    gap: var(--s-2);
    min-height: 48px;
    padding: var(--s-2) var(--s-4) var(--s-2) var(--s-5);
    border-bottom: 1px solid var(--faint);
    color: var(--muted);
  }
  .collection-bar :global(svg) {
    color: var(--cyan);
  }
  .collection-bar h2 {
    margin: 0;
    font-size: var(--text-3);
    font-weight: var(--weight-strong);
    color: var(--ink);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .collection-meta {
    font-size: var(--text-1);
    white-space: nowrap;
  }
  .spacer {
    flex: 1;
  }
  .hint {
    margin: var(--s-1) var(--s-2) 0;
    color: var(--muted);
    font-size: var(--text-1);
    line-height: 1.45;
  }

  .disclosure {
    display: flex;
    align-items: center;
    gap: var(--s-1);
    width: 100%;
    height: 26px;
    margin-top: 2px;
    padding: 0 var(--s-1) 0 var(--s-2);
    border-radius: var(--r-2);
    color: var(--muted);
    font-size: var(--text-1);
    text-align: left;
  }
  .disclosure .count {
    margin-left: auto;
    padding-right: var(--s-1);
  }
  .disclosure:hover {
    background: var(--pressed);
    color: var(--ink);
  }

  /* Main ----------------------------------------------------------------- */
  main {
    position: relative;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: var(--sheet);
    /* Meets the sidebar; with the sidebar hidden it meets the window edge,
       which needs no line. */
    border-left: var(--main-rule) solid var(--faint);
  }
  .tester {
    display: flex;
    align-items: center;
    gap: var(--s-5);
    height: 52px;
    padding: 0 var(--s-5);
    border-bottom: 1px solid var(--faint);
  }
  .sample {
    flex: 1;
    min-width: 0;
  }
  .results {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  /* Rows of cards. The top padding is each row's gap; the last row also gets
     bottom padding so the final cards don't sit on the window edge. */
  .grid-row {
    display: grid;
    gap: 16px;
    padding: 16px 20px 0;
  }
  .grid-row.last {
    padding-bottom: 20px;
  }
  .sample input {
    width: 100%;
    border: 0;
    background: none;
    font-size: var(--text-3);
    padding: var(--s-1) 0;
    outline-offset: 4px;
  }
  .sample input::placeholder {
    color: var(--muted);
  }
  .size {
    display: flex;
    align-items: center;
    gap: var(--s-3);
    width: 200px;
    flex: none;
    color: var(--muted);
  }

  .empty {
    padding: var(--s-8) var(--s-5);
    color: var(--muted);
    max-width: 52ch;
  }
  .empty p {
    margin: 0 0 var(--s-4);
    line-height: 1.55;
  }
  .empty h2 {
    color: var(--ink);
    font-size: var(--text-4);
    margin: 0 0 var(--s-2);
  }
  .empty-icon {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    margin-bottom: var(--s-4);
    border-radius: var(--r-full);
    background: var(--cyan-soft);
    color: var(--cyan);
  }

  @media (max-width: 1100px) {
    /* The inspector floats over the list, so it takes no column. */
    .inspector-slot {
      width: 0;
    }
    .app.has-inspector :global(.inspector) {
      position: fixed;
      /* Clear of the status bar. */
      inset: 0 0 29px auto;
      width: min(var(--inspector), 100vw);
      box-shadow: var(--shadow-float);
      z-index: var(--z-panel);
    }
    /* Widened, the overlay leaves the strip of list in view. */
    .app.wide .inspector-slot {
      width: 0;
    }
    .app.has-inspector :global(.inspector) {
      transition: width var(--dur-pane) var(--ease);
    }
    .app.wide :global(.inspector) {
      width: calc(100vw - var(--center-narrow) - var(--rail-w));
    }
  }
</style>
