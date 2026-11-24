// What the library and Google Fonts can be filtered and sorted by.

import type { FamilySummary, GoogleFamilySummary } from './api';
import type { Facet, SortKey } from './query';

const weightPresets = [
  { label: 'Thin to Light', min: 100, max: 300 },
  { label: 'Regular to Medium', min: 400, max: 500 },
  { label: 'Bold to Black', min: 700, max: 900 },
];

const weightName: Record<number, string> = {
  100: 'Thin', 200: 'ExtraLight', 300: 'Light', 400: 'Regular', 500: 'Medium',
  600: 'SemiBold', 700: 'Bold', 800: 'ExtraBold', 900: 'Black',
};
export const formatWeight = (n: number) => (weightName[n] ? `${n} ${weightName[n]}` : String(n));

/** "/Sans/Humanist" → group "Sans", label "Humanist". */
export function splitTag(path: string): { group: string; label: string } {
  const parts = path.split('/').filter(Boolean);
  if (parts.length < 2) return { group: 'Other', label: parts[0] ?? path };
  return { group: parts[0]!, label: parts.slice(1).join(' / ') };
}

const AXIS_NAMES: Record<string, string> = {
  wght: 'Weight', wdth: 'Width', opsz: 'Optical size', slnt: 'Slant', ital: 'Italic',
  GRAD: 'Grade', XOPQ: 'Thick stroke', YOPQ: 'Thin stroke', XTRA: 'Counter width',
  YTUC: 'Uppercase height', YTLC: 'Lowercase height', YTAS: 'Ascender', YTDE: 'Descender',
  YTFI: 'Figure height', CASL: 'Casual', CRSV: 'Cursive', MONO: 'Monospace', SOFT: 'Softness',
  WONK: 'Wonky', FILL: 'Fill', ROND: 'Roundness', BLED: 'Bleed', SCAN: 'Scanlines',
};
export const axisLabel = (tag: string) => (AXIS_NAMES[tag] ? `${AXIS_NAMES[tag]} (${tag})` : tag);

const SCRIPT_NAMES: Record<string, string> = {
  latin: 'Latin', 'latin-ext': 'Latin Extended', cyrillic: 'Cyrillic', 'cyrillic-ext': 'Cyrillic Extended',
  greek: 'Greek', 'greek-ext': 'Greek Extended', vietnamese: 'Vietnamese', arabic: 'Arabic',
  hebrew: 'Hebrew', devanagari: 'Devanagari', bengali: 'Bengali', thai: 'Thai', khmer: 'Khmer',
  'chinese-simplified': 'Chinese (Simplified)', 'chinese-traditional': 'Chinese (Traditional)',
  'chinese-hongkong': 'Chinese (Hong Kong)', japanese: 'Japanese', korean: 'Korean', math: 'Math',
  symbols: 'Symbols', music: 'Music', emoji: 'Emoji', tamil: 'Tamil', telugu: 'Telugu',
  gujarati: 'Gujarati', gurmukhi: 'Gurmukhi', kannada: 'Kannada', malayalam: 'Malayalam',
  oriya: 'Odia', sinhala: 'Sinhala', georgian: 'Georgian', armenian: 'Armenian', ethiopic: 'Ethiopic',
  lao: 'Lao', myanmar: 'Myanmar', tibetan: 'Tibetan', mongolian: 'Mongolian',
};
export const scriptLabel = (s: string) =>
  SCRIPT_NAMES[s] ?? s.replace(/-/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase());

// -- Library ------------------------------------------------------------------

/** Lightest and heaviest weight a family covers, counting variable axes. */
export function familyWeights(f: FamilySummary): [number, number] {
  let lo = Infinity;
  let hi = -Infinity;
  for (const x of f.faces) {
    const [a, b] = x.weightRange ?? [x.weight, x.weight];
    lo = Math.min(lo, a);
    hi = Math.max(hi, b);
  }
  return Number.isFinite(lo) ? [lo, hi] : [400, 400];
}

function widthClass(w: number): string {
  return w <= 4 ? 'condensed' : w >= 6 ? 'expanded' : 'normal';
}

export function libraryFacets(tagsOf: (family: string) => string[]): Facet<FamilySummary>[] {
  return [
    {
      kind: 'set',
      id: 'status',
      icon: 'active',
      label: 'Status',
      options: [
        { value: 'all', label: 'Active' },
        { value: 'partial', label: 'Partly active' },
        { value: 'none', label: 'Inactive' },
        { value: 'installed', label: 'Installed by the system' },
      ],
      values: (f) => [f.installed ? 'installed' : f.activation],
    },
    {
      kind: 'set',
      id: 'traits',
      icon: 'traits',
      label: 'Traits',
      options: [
        { value: 'variable', label: 'Variable' },
        { value: 'static', label: 'Static' },
        { value: 'italic', label: 'Has italics' },
        { value: 'mono', label: 'Monospaced' },
      ],
      values: (f) => {
        const v: string[] = [f.variable ? 'variable' : 'static'];
        if (f.faces.some((x) => x.italic)) v.push('italic');
        if (f.monospaced) v.push('mono');
        return v;
      },
      modes: true,
    },
    {
      kind: 'range',
      id: 'weight',
      icon: 'weight',
      label: 'Weight',
      min: 100,
      max: 900,
      step: 100,
      presets: weightPresets,
      span: familyWeights,
      format: formatWeight,
    },
    {
      kind: 'set',
      id: 'width',
      icon: 'width',
      label: 'Width',
      options: [
        { value: 'condensed', label: 'Condensed' },
        { value: 'normal', label: 'Normal' },
        { value: 'expanded', label: 'Expanded' },
      ],
      values: (f) => [...new Set(f.faces.map((x) => widthClass(x.width)))],
    },
    {
      kind: 'range',
      id: 'styles',
      icon: 'styles',
      label: 'Styles',
      min: 1,
      max: 24,
      step: 1,
      presets: [
        { label: 'Just one', min: 1, max: 1 },
        { label: '2 to 4', min: 2, max: 4 },
        { label: '5 or more', min: 5, max: 24 },
      ],
      span: (f) => [Math.min(f.faces.length, 24), Math.min(f.faces.length, 24)],
      format: (n) => (n >= 24 ? '24+' : String(n)),
    },
    {
      kind: 'set',
      id: 'tags',
      icon: 'tags',
      label: 'Tags',
      values: (f) => tagsOf(f.name),
      searchable: true,
      modes: true,
    },
    {
      kind: 'set',
      id: 'format',
      icon: 'format',
      label: 'Format',
      options: [
        { value: 'truetype', label: 'TrueType outlines' },
        { value: 'opentype', label: 'PostScript (CFF) outlines' },
        { value: 'collection', label: 'Font collection' },
      ],
      values: (f) => [...new Set(f.faces.map((x) => x.format))],
    },
  ];
}

const newest = (f: FamilySummary) => Math.max(0, ...f.faces.map((x) => x.modified ?? 0));
const activeShare = (f: FamilySummary) =>
  f.installed ? 2 : f.activation === 'all' ? 1 : f.activation === 'partial' ? 0.5 : 0;

export function librarySorts(score: (f: FamilySummary) => number): SortKey<FamilySummary>[] {
  return [
    { id: 'relevance', label: 'Best match', compare: (a, b) => score(a) - score(b), dir: 'desc', dirs: ['Weakest first', 'Strongest first'], needsQuery: true },
    { id: 'name', label: 'Name', compare: (a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: 'base' }), dir: 'asc', dirs: ['A to Z', 'Z to A'] },
    { id: 'added', label: 'Date modified', compare: (a, b) => newest(a) - newest(b), dir: 'desc', dirs: ['Oldest first', 'Newest first'] },
    { id: 'styles', label: 'Number of styles', compare: (a, b) => a.faces.length - b.faces.length, dir: 'desc', dirs: ['Fewest first', 'Most first'] },
    { id: 'weight', label: 'Weight range', compare: (a, b) => { const [al, ah] = familyWeights(a); const [bl, bh] = familyWeights(b); return ah - al - (bh - bl); }, dir: 'desc', dirs: ['Narrowest first', 'Widest first'] },
    { id: 'active', label: 'Activation', compare: (a, b) => activeShare(a) - activeShare(b), dir: 'desc', dirs: ['Inactive first', 'Active first'] },
  ];
}

// -- Google Fonts -------------------------------------------------------------

export const CATEGORIES = [
  { value: 'sans-serif', label: 'Sans serif' },
  { value: 'serif', label: 'Serif' },
  { value: 'display', label: 'Display' },
  { value: 'handwriting', label: 'Handwriting' },
  { value: 'monospace', label: 'Monospace' },
];
export const categoryLabel = (c: string) => CATEGORIES.find((x) => x.value === c)?.label ?? c;

/** Whether a downloaded family is active, given the library. */
export type LibraryLookup = (g: GoogleFamilySummary) => FamilySummary | undefined;

export function googleFacets(lookup: LibraryLookup): Facet<GoogleFamilySummary>[] {
  return [
    {
      kind: 'set',
      id: 'category',
      icon: 'category',
      label: 'Category',
      options: CATEGORIES,
      values: (f) => [f.category],
    },
    {
      kind: 'set',
      id: 'status',
      icon: 'active',
      label: 'Status',
      options: [
        { value: 'available', label: 'Not downloaded' },
        { value: 'downloaded', label: 'In your library' },
        { value: 'active', label: 'Active' },
        { value: 'update', label: 'Update available' },
      ],
      values: (f) => {
        if (f.status === 'available') return ['available'];
        const v = ['downloaded'];
        if (f.status === 'update') v.push('update');
        const lib = lookup(f);
        if (lib && lib.activation !== 'none') v.push('active');
        return v;
      },
    },
    {
      kind: 'set',
      id: 'traits',
      icon: 'traits',
      label: 'Traits',
      options: [
        { value: 'variable', label: 'Variable' },
        { value: 'static', label: 'Static' },
        { value: 'italic', label: 'Has italics' },
      ],
      values: (f) => (f.italic ? [f.variable ? 'variable' : 'static', 'italic'] : [f.variable ? 'variable' : 'static']),
      modes: true,
    },
    {
      kind: 'range',
      id: 'weight',
      icon: 'weight',
      label: 'Weight',
      min: 100,
      max: 900,
      step: 100,
      presets: weightPresets,
      span: (f) => f.weightRange,
      format: formatWeight,
    },
    {
      kind: 'range',
      id: 'styles',
      icon: 'styles',
      label: 'Styles',
      min: 1,
      max: 18,
      step: 1,
      presets: [
        { label: 'Just one', min: 1, max: 1 },
        { label: '2 to 4', min: 2, max: 4 },
        { label: '5 or more', min: 5, max: 18 },
      ],
      span: (f) => [Math.min(f.styles, 18), Math.min(f.styles, 18)],
      format: (n) => (n >= 18 ? '18+' : String(n)),
    },
    {
      kind: 'set',
      id: 'scripts',
      icon: 'scripts',
      label: 'Scripts',
      values: (f) => f.subsets,
      format: scriptLabel,
      searchable: true,
      modes: true,
    },
    {
      kind: 'set',
      id: 'axes',
      icon: 'variable',
      label: 'Axes',
      values: (f) => f.axes,
      format: axisLabel,
      searchable: true,
      modes: true,
    },
    {
      kind: 'set',
      id: 'tags',
      icon: 'tags',
      label: 'Tags',
      values: (f) => f.tags,
      format: (t) => splitTag(t).label,
      group: (t) => splitTag(t).group,
      searchable: true,
      modes: true,
    },
  ];
}

export function googleSorts(score: (f: GoogleFamilySummary) => number): SortKey<GoogleFamilySummary>[] {
  return [
    // Equal matches (say, every family with Cyrillic) fall back to popularity.
    { id: 'relevance', label: 'Best match', compare: (a, b) => score(a) - score(b) || b.rank - a.rank, dir: 'desc', dirs: ['Weakest first', 'Strongest first'], needsQuery: true },
    { id: 'popular', label: 'Popularity', compare: (a, b) => b.rank - a.rank, dir: 'desc', dirs: ['Least popular first', 'Most popular first'] },
    { id: 'name', label: 'Name', compare: (a, b) => a.family.localeCompare(b.family, undefined, { sensitivity: 'base' }), dir: 'asc', dirs: ['A to Z', 'Z to A'] },
    { id: 'updated', label: 'Last updated', compare: (a, b) => a.lastModified.localeCompare(b.lastModified), dir: 'desc', dirs: ['Oldest first', 'Newest first'] },
    { id: 'styles', label: 'Number of styles', compare: (a, b) => a.styles - b.styles, dir: 'desc', dirs: ['Fewest first', 'Most first'] },
    { id: 'scripts', label: 'Number of scripts', compare: (a, b) => a.subsets.length - b.subsets.length, dir: 'desc', dirs: ['Fewest first', 'Most first'] },
  ];
}

