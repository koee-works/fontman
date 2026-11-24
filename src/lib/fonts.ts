// Preview font loading.
//
// Three rules this module exists to enforce:
//
// 1. Never inject @font-face CSS. Use the FontFace API so a failed load is
//    *detectable*. A failed face silently falls back to a system font and the
//    preview looks plausible, which is the worst failure a font manager can have.
// 2. Synthetic family names only (`pf-<id>`). The real name would collide with
//    an installed copy of the same font, and you would be judging the wrong file.
// 3. Bounded memory. Loaded faces are evicted LRU — never while on screen —
//    by count and by approximate bytes. In testing, scrolling repeatedly
//    through ~80 CJK collection faces plateaued at ~620 MB of webview memory
//    with or without the byte limit, so WebKit evidently doesn't hold raw font
//    bytes in proportion; the byte limit is a safeguard, not a measured fix.
// 4. Don't load what's flying past. A row must stay on screen briefly before
//    its font loads, so flinging through thousands of families costs nothing.

import { convertFileSrc } from '@tauri-apps/api/core';

export type LoadState = 'loading' | 'loaded' | 'error';

/** Prefix for Google Fonts catalog previews: `gf:<family name>`. */
export const GOOGLE_PREFIX = 'gf:';

export const googlePreviewId = (family: string) => GOOGLE_PREFIX + family;

/** Prefix for a Google family's regular style in full: `gff:<family name>`.
 *  Name-only previews can't show other text, so typed samples use these. */
export const GOOGLE_FULL_PREFIX = 'gff:';

export const googleFullId = (family: string) => GOOGLE_FULL_PREFIX + family;

/** Short stable hash, so a family name becomes a safe CSS identifier. */
function hash(s: string): string {
  let h = 0x811c9dc5;
  for (let i = 0; i < s.length; i++) h = Math.imul(h ^ s.charCodeAt(i), 0x01000193);
  return (h >>> 0).toString(16);
}

/** Must match `FaceRecord::css_family` in font-core for library faces. */
export function cssFamily(faceId: string): string {
  if (faceId.startsWith(GOOGLE_PREFIX) || faceId.startsWith(GOOGLE_FULL_PREFIX)) return `pf-gf-${hash(faceId)}`;
  return `pf-${faceId.slice(0, 16)}`;
}

/**
 * `font://localhost/<id>` on macOS/Linux, `http://font.localhost/<id>` on
 * Windows. Google previews are served by the Rust side too, at `gf/<family>`,
 * so the webview never contacts Google directly.
 */
export function fontUrl(faceId: string): string {
  if (faceId.startsWith(GOOGLE_PREFIX)) {
    return convertFileSrc(`gf/${faceId.slice(GOOGLE_PREFIX.length)}`, 'font');
  }
  if (faceId.startsWith(GOOGLE_FULL_PREFIX)) {
    return convertFileSrc(`gf-full/${faceId.slice(GOOGLE_FULL_PREFIX.length)}`, 'font');
  }
  return convertFileSrc(faceId, 'font');
}

/** CSS font-family for a face. The fallback is a font no one would mistake for a specimen. */
export function fontStack(faceId: string): string {
  return `"${cssFamily(faceId)}", "Adobe NotDef", sans-serif`;
}

const MAX_LOADED = 400;
const MAX_BYTES = 192 * 1024 * 1024;
const MAX_CONCURRENT = 6;
/** How long a row must stay visible before its font loads. */
const DWELL_MS = 120;
const DEFAULT_BYTES = 256 * 1024;

const sizes = new Map<string, number>();
let heldBytes = 0;

/** Tell the loader how big each face is, so eviction can budget memory. */
export function setFaceSizes(faces: Iterable<{ id: string; bytes: number }>) {
  for (const f of faces) sizes.set(f.id, f.bytes);
}

/** Loaded and still cached: can be shown with no dwell. */
export function isLoaded(faceId: string): boolean {
  return entries.get(faceId)?.state === 'loaded';
}

interface Entry {
  face: FontFace | null;
  bytes: number;
  state: LoadState;
  pins: number;
  lastUsed: number;
  promise: Promise<LoadState>;
}

const entries = new Map<string, Entry>();

/** Counters for measuring loader behaviour under fast scrolling. */
export const loadStats = { started: 0, loaded: 0, skipped: 0 };
let inFlight = 0;
const queue: (() => void)[] = [];

function runQueued<T>(task: () => Promise<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    const start = () => {
      inFlight++;
      task()
        .then(resolve, reject)
        .finally(() => {
          inFlight--;
          queue.shift()?.();
        });
    };
    if (inFlight < MAX_CONCURRENT) start();
    else queue.push(start);
  });
}

const overBudget = () => entries.size > MAX_LOADED || heldBytes > MAX_BYTES;

function evict() {
  if (!overBudget()) return;
  // Only unpinned faces are candidates; on-screen faces are never evicted,
  // so the budget can be exceeded while they are all visible.
  const idle = [...entries.entries()]
    .filter(([, e]) => e.pins === 0 && e.state !== 'loading')
    .sort((a, b) => a[1].lastUsed - b[1].lastUsed);
  for (const [id, e] of idle) {
    if (!overBudget()) break;
    if (e.face) {
      document.fonts.delete(e.face);
      heldBytes -= e.bytes;
    }
    entries.delete(id);
  }
}

/** Load a face and pin it while on screen. Pair every call with `release`. */
export function acquire(faceId: string): Promise<LoadState> {
  let e = entries.get(faceId);
  if (!e) {
    const entry: Entry = {
      face: null,
      bytes: sizes.get(faceId) ?? DEFAULT_BYTES,
      state: 'loading',
      // Pinned before it is queued: a load that starts immediately checks
      // `pins` synchronously, and must not mistake itself for scrolled-away.
      pins: 1,
      lastUsed: performance.now(),
      promise: Promise.resolve<LoadState>('loading'),
    };
    entries.set(faceId, entry);
    entry.promise = runQueued(async () => {
      // Scrolled past before its turn in the queue: don't spend a load on it.
      // A later acquire creates a fresh entry.
      if (entry.pins === 0) {
        if (entries.get(faceId) === entry) entries.delete(faceId);
        loadStats.skipped++;
        return 'loading';
      }
      loadStats.started++;
      try {
        const face = new FontFace(cssFamily(faceId), `url("${fontUrl(faceId)}")`);
        await face.load();
        if (face.status !== 'loaded') throw new Error(face.status);
        document.fonts.add(face);
        entry.face = face;
        entry.state = 'loaded';
        heldBytes += entry.bytes;
        loadStats.loaded++;
        evict();
      } catch {
        entry.state = 'error';
      }
      return entry.state;
    });
    evict();
    return entry.promise;
  }
  e.pins++;
  e.lastUsed = performance.now();
  return e.promise;
}

export function release(faceId: string) {
  const e = entries.get(faceId);
  if (!e) return;
  e.pins = Math.max(0, e.pins - 1);
  e.lastUsed = performance.now();
  evict();
}

// ---------------------------------------------------------------------------
// `use:specimen={faceId}` — render an element in a face once it scrolls near
// the viewport. Until the face is loaded the element is marked
// data-state="loading" and CSS hides its text, so fallback glyphs are never
// shown as if they were the font.
// ---------------------------------------------------------------------------

type Watcher = (visible: boolean) => void;
const watchers = new WeakMap<Element, Watcher>();
let observer: IntersectionObserver | null = null;

function getObserver(): IntersectionObserver {
  observer ??= new IntersectionObserver(
    (records) => {
      for (const r of records) watchers.get(r.target)?.(r.isIntersecting);
    },
    { rootMargin: '400px 0px' },
  );
  return observer;
}

export function specimen(node: HTMLElement, faceId: string) {
  let current = faceId;
  let pinned: string | null = null;

  const setState = (s: LoadState, fresh = false) => {
    node.dataset.state = s;
    // Fades in (app.css) only when the face had to be fetched.
    if (fresh) node.dataset.fresh = '';
    else delete node.dataset.fresh;
  };

  let dwell: ReturnType<typeof setTimeout> | null = null;

  const load = (id: string) => {
    pinned = id;
    const cached = isLoaded(id);
    setState('loading');
    acquire(id).then((s) => {
      // The row may have been recycled for another face while loading.
      if (pinned !== id) return;
      if (s === 'loaded') node.style.fontFamily = fontStack(id);
      setState(s, s === 'loaded' && !cached);
    });
  };
  const show = (id: string) => {
    if (pinned === id) return;
    hide();
    if (isLoaded(id)) return load(id); // cached: no reason to wait
    setState('loading');
    dwell = setTimeout(() => {
      dwell = null;
      load(id);
    }, DWELL_MS);
  };
  const hide = () => {
    if (dwell) clearTimeout(dwell);
    dwell = null;
    if (pinned) release(pinned);
    pinned = null;
  };

  let visible = false;
  watchers.set(node, (v) => {
    visible = v;
    if (v) show(current);
    else hide();
  });
  setState('loading');
  getObserver().observe(node);

  return {
    update(next: string) {
      if (next === current) return;
      current = next;
      node.style.fontFamily = '';
      if (visible) show(next);
      else setState('loading');
    },
    destroy() {
      hide();
      getObserver().unobserve(node);
      watchers.delete(node);
    },
  };
}
