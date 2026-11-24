// Searching, filtering and sorting, shared by the library and Google Fonts.
//
// Search text is parsed into words that must all appear, "quoted phrases",
// and -exclusions. Filters are facets: a set of values to pick from (with
// counts of what each pick would show) or a numeric range. Counts are
// computed with every *other* facet applied, so a facet never hides its own
// alternatives. Sorting is one key and a direction, with the name as the
// tiebreak so equal items never shuffle.

import { fuzzy } from './palette';
import type { IconName } from './ui/Icon.svelte';

// -- Search -------------------------------------------------------------------

export interface ParsedQuery {
  /** Words and phrases that must each appear in one of an item's fields. */
  include: string[];
  /** Words and phrases that must appear in none of them. */
  exclude: string[];
}

/** Parse `serif "small caps" -display` into terms. Case is ignored. */
export function parseQuery(text: string): ParsedQuery {
  const include: string[] = [];
  const exclude: string[] = [];
  const re = /(-?)(?:"([^"]*)"?|(\S+))/g;
  for (const m of text.toLowerCase().matchAll(re)) {
    const term = (m[2] ?? m[3] ?? '').trim();
    // A lone "-" is a word being typed, not an exclusion.
    if (!term || term === '-') continue;
    (m[1] ? exclude : include).push(term);
  }
  return { include, exclude };
}

export const isEmptyQuery = (q: ParsedQuery) => !q.include.length && !q.exclude.length;

/** Every included term in some field, and no excluded term in any. */
export function matchQuery(q: ParsedQuery, fields: readonly string[]): boolean {
  const lower = fields.map((f) => f.toLowerCase());
  return (
    q.include.every((t) => lower.some((f) => f.includes(t))) &&
    !q.exclude.some((t) => lower.some((f) => f.includes(t)))
  );
}

/**
 * How well a name answers the query, for "Best match" order: the whole name,
 * then its start, then a word start, then anywhere. Terms found only in other
 * fields (tags, category) still count, but less.
 */
export function relevance(q: ParsedQuery, name: string): number {
  if (!q.include.length) return 0;
  const n = name.toLowerCase();
  const whole = q.include.join(' ');
  if (n === whole) return 1000;
  let score = n.startsWith(whole) ? 500 : 0;
  for (const t of q.include) {
    const at = n.indexOf(t);
    if (at === 0) score += 60;
    else if (at > 0 && /[\s\-_]/.test(n[at - 1]!)) score += 40;
    else if (at > 0) score += 20;
  }
  // Among names that match, shorter ones are closer to what was typed. A
  // match only in other fields scores 0, leaving the order to the tiebreak.
  return score ? score - n.length * 0.1 : 0;
}

/**
 * Names close to the query when nothing matches it exactly, typos included.
 * Exclusions still apply. Returns at most `limit`, best first.
 */
export function closeMatches<T>(
  items: readonly T[],
  q: ParsedQuery,
  name: (item: T) => string,
  limit = 20,
): T[] {
  const text = q.include.join(' ');
  if (!text) return [];
  const scored: { item: T; score: number }[] = [];
  for (const item of items) {
    const n = name(item);
    if (q.exclude.some((t) => n.toLowerCase().includes(t))) continue;
    const m = fuzzy(text, n);
    // Require most of the query to land on word starts or runs, or a
    // scattered "e" through "s" matches everything.
    if (m && m.score > text.replace(/\s/g, '').length * 2.5) scored.push({ item, score: m.score });
  }
  return scored
    .sort((a, b) => b.score - a.score)
    .slice(0, limit)
    .map((s) => s.item);
}

// -- Facets -------------------------------------------------------------------

export interface FacetOption {
  value: string;
  label: string;
  /** Heading the option is listed under, such as a tag's group. */
  group?: string;
}

export interface SetFacet<T> {
  kind: 'set';
  id: string;
  label: string;
  /** Shown on the filter chip, so filters are told apart at a glance. */
  icon: IconName;
  /** The values an item has. An item matches when it has any picked value,
   *  or every picked value when the mode is "all". */
  values: (item: T) => readonly string[];
  /** Fixed options, in display order. Without it, options come from the
   *  items, sorted by label. */
  options?: FacetOption[];
  /** Label for a value found in the items. */
  format?: (value: string) => string;
  group?: (value: string) => string | undefined;
  /** Long lists get a search field in their popover. */
  searchable?: boolean;
  /** Offer "any" versus "all" matching (tags, scripts). */
  modes?: boolean;
}

export interface RangeFacet<T> {
  kind: 'range';
  id: string;
  label: string;
  icon: IconName;
  min: number;
  max: number;
  step: number;
  unit?: string;
  /** Preset spans, shown as quick picks. */
  presets?: { label: string; min: number; max: number }[];
  /** The span an item covers. It matches when the spans overlap. */
  span: (item: T) => readonly [number, number];
  format?: (n: number) => string;
}

export type Facet<T> = SetFacet<T> | RangeFacet<T>;

export type FacetValue =
  | { kind: 'set'; values: string[]; mode: 'any' | 'all' }
  | { kind: 'range'; min: number; max: number };

export type FilterState = Record<string, FacetValue | undefined>;

export function isFacetActive<T>(facet: Facet<T>, v: FacetValue | undefined): boolean {
  if (!v) return false;
  if (v.kind === 'set') return v.values.length > 0;
  return facet.kind === 'range' && (v.min > facet.min || v.max < facet.max);
}

export function activeCount<T>(facets: readonly Facet<T>[], state: FilterState): number {
  return facets.filter((f) => isFacetActive(f, state[f.id])).length;
}

function matchesFacet<T>(item: T, facet: Facet<T>, v: FacetValue | undefined): boolean {
  if (!isFacetActive(facet, v)) return true;
  if (facet.kind === 'set' && v!.kind === 'set') {
    const have = facet.values(item);
    return v!.mode === 'all'
      ? v!.values.every((x) => have.includes(x))
      : v!.values.some((x) => have.includes(x));
  }
  if (facet.kind === 'range' && v!.kind === 'range') {
    const [lo, hi] = facet.span(item);
    return hi >= v!.min && lo <= v!.max;
  }
  return true;
}

/** Items passing every facet, or every facet but `except`. */
export function applyFacets<T>(
  items: readonly T[],
  facets: readonly Facet<T>[],
  state: FilterState,
  except?: string,
): T[] {
  const live = facets.filter((f) => f.id !== except && isFacetActive(f, state[f.id]));
  if (!live.length) return items.slice();
  return items.filter((item) => live.every((f) => matchesFacet(item, f, state[f.id])));
}

/**
 * A set facet's options with how many items each would show, given every
 * other facet. Options with no items are kept when picked, so a pick can
 * always be undone.
 */
export function facetOptions<T>(
  items: readonly T[],
  facets: readonly Facet<T>[],
  state: FilterState,
  facet: SetFacet<T>,
): (FacetOption & { count: number })[] {
  const pool = applyFacets(items, facets, state, facet.id);
  const counts = new Map<string, number>();
  for (const item of pool) for (const v of new Set(facet.values(item))) counts.set(v, (counts.get(v) ?? 0) + 1);
  const picked = new Set(state[facet.id]?.kind === 'set' ? (state[facet.id] as { values: string[] }).values : []);

  let options: FacetOption[];
  if (facet.options) options = facet.options;
  else {
    const seen = new Set<string>();
    for (const item of items) for (const v of facet.values(item)) seen.add(v);
    options = [...seen].map((value) => ({
      value,
      label: facet.format?.(value) ?? value,
      group: facet.group?.(value),
    }));
    options.sort(
      (a, b) => (a.group ?? '').localeCompare(b.group ?? '') || a.label.localeCompare(b.label),
    );
  }
  return options
    .map((o) => ({ ...o, count: counts.get(o.value) ?? 0 }))
    .filter((o) => o.count > 0 || picked.has(o.value) || facet.options);
}

/** Short summary for a filter chip: "Serif, Display", "300–700". */
export function describeFacet<T>(facet: Facet<T>, v: FacetValue | undefined): string {
  if (!v || !isFacetActive(facet, v)) return '';
  if (facet.kind === 'set' && v.kind === 'set') {
    const label = (x: string) =>
      facet.options?.find((o) => o.value === x)?.label ?? facet.format?.(x) ?? x;
    const names = v.values.map(label);
    const joined = names.length > 2 ? `${names.slice(0, 2).join(', ')} +${names.length - 2}` : names.join(', ');
    return v.mode === 'all' && names.length > 1 ? `all of ${joined}` : joined;
  }
  if (facet.kind === 'range' && v.kind === 'range') {
    const f = facet.format ?? String;
    const unit = facet.unit ? ` ${facet.unit}` : '';
    if (v.min <= facet.min) return `up to ${f(v.max)}${unit}`;
    if (v.max >= facet.max) return `${f(v.min)}${unit} or more`;
    return v.min === v.max ? `${f(v.min)}${unit}` : `${f(v.min)}–${f(v.max)}${unit}`;
  }
  return '';
}

/** Toggle one value of a set facet. */
export function toggleValue(state: FilterState, id: string, value: string): FilterState {
  const cur = state[id];
  const values = cur?.kind === 'set' ? cur.values : [];
  const mode = cur?.kind === 'set' ? cur.mode : 'any';
  const next = values.includes(value) ? values.filter((x) => x !== value) : [...values, value];
  return { ...state, [id]: next.length ? { kind: 'set', values: next, mode } : undefined };
}

// -- Sorting ------------------------------------------------------------------

export type SortDir = 'asc' | 'desc';

export interface SortKey<T> {
  id: string;
  label: string;
  /** Ascending comparison. */
  compare: (a: T, b: T) => number;
  /** The natural first direction: Z–A is rarely wanted first, newest is. */
  dir: SortDir;
  /** Words for each direction: ["A to Z", "Z to A"]. */
  dirs: [asc: string, desc: string];
  /** Only offered while searching. */
  needsQuery?: boolean;
}

export interface SortState {
  key: string;
  dir: SortDir;
}

export function sortItems<T>(
  items: T[],
  keys: readonly SortKey<T>[],
  state: SortState,
  name: (item: T) => string,
): T[] {
  const key = keys.find((k) => k.id === state.key) ?? keys[0];
  if (!key) return items;
  const sign = state.dir === 'asc' ? 1 : -1;
  const byName = (a: T, b: T) => name(a).localeCompare(name(b), undefined, { sensitivity: 'base' });
  return items.sort((a, b) => sign * key.compare(a, b) || byName(a, b));
}
