// Checks search parsing, facets with counts, and sorting. Run: npm run test:query
import assert from 'node:assert/strict';
import {
  applyFacets, closeMatches, describeFacet, facetOptions, isFacetActive, matchQuery, parseQuery,
  relevance, sortItems, toggleValue, type FilterState,
} from '../src/lib/query';
import { familyWeights, googleFacets, googleSorts, libraryFacets, librarySorts, splitTag } from '../src/lib/facets';
import type { FaceSummary, FamilySummary, GoogleFamilySummary } from '../src/lib/api';

// -- Search syntax
assert.deepEqual(parseQuery('Serif "small caps" -display'), { include: ['serif', 'small caps'], exclude: ['display'] });
assert.deepEqual(parseQuery('  '), { include: [], exclude: [] });
assert.deepEqual(parseQuery('mono -'), { include: ['mono'], exclude: [] });
assert.deepEqual(parseQuery('-"open sans"'), { include: [], exclude: ['open sans'] });
assert.deepEqual(parseQuery('"unclosed phrase'), { include: ['unclosed phrase'], exclude: [] });

const q = parseQuery('sans -mono');
assert.equal(matchQuery(q, ['Open Sans']), true);
assert.equal(matchQuery(q, ['Noto Sans Mono']), false);
assert.equal(matchQuery(q, ['Inter', 'Sans / Humanist']), true, 'terms may match other fields, like tags');
assert.equal(matchQuery(parseQuery('inter tight'), ['Inter Tight']), true);
assert.equal(matchQuery(parseQuery('"inter tight"'), ['Inter Display Tight']), false);

// Relevance: exact name, then prefix, then word start, then anywhere.
const r = (name: string) => relevance(parseQuery('inter'), name);
assert.ok(r('Inter') > r('Inter Tight'));
assert.ok(r('Inter Tight') > r('Noto Inter'));
assert.ok(r('Noto Inter') > r('Winterland'));

// Typos fall back to close matches.
const names = ['Roboto', 'Roboto Mono', 'Inter', 'Lora', 'Montserrat'];
assert.deepEqual(closeMatches(names, parseQuery('robto'), (n) => n).slice(0, 2), ['Roboto', 'Roboto Mono']);
assert.deepEqual(closeMatches(names, parseQuery('montsrat'), (n) => n), ['Montserrat']);
assert.deepEqual(closeMatches(names, parseQuery('zzz'), (n) => n), []);

// -- Library facets
let n = 0;
const face = (o: Partial<FaceSummary> = {}): FaceSummary => ({
  id: `f${n++}`, subfamily: 'Regular', fullName: null, postscriptName: null, weight: 400, width: 5,
  italic: false, variable: false, format: 'truetype', active: false, installed: false, bytes: 1,
  variation: null, modified: 0, axes: [], weightRange: null, ...o,
});
const fam = (name: string, faces: FaceSummary[], o: Partial<FamilySummary> = {}): FamilySummary => ({
  name, activation: 'none', representative: faces[0]!.id, variable: faces.some((f) => f.variable),
  monospaced: false, installed: false, faces, ...o,
});
const lib = [
  fam('Inter', [face({ variable: true, weightRange: [100, 900], axes: ['wght', 'opsz'], modified: 30 }), face({ italic: true, variable: true, weightRange: [100, 900], modified: 30 })], { activation: 'all' }),
  fam('Lora', [face({ weight: 400, modified: 10 }), face({ weight: 700, modified: 10 }), face({ weight: 700, italic: true, modified: 10 })], { activation: 'partial' }),
  fam('Fira Mono', [face({ weight: 400, format: 'opentype', modified: 20 })], { monospaced: true }),
  fam('Bebas Condensed', [face({ weight: 700, width: 3, modified: 5 })]),
  fam('Arial', [face({ installed: true, weight: 400 })], { installed: true, activation: 'all' }),
];
const tagMap: Record<string, string[]> = { Inter: ['UI', 'Grotesque'], Lora: ['Book'], 'Fira Mono': ['Code', 'UI'] };
const lf = libraryFacets((name) => tagMap[name] ?? []);
const facet = (id: string) => lf.find((f) => f.id === id)!;
const names2 = (xs: FamilySummary[]) => xs.map((x) => x.name);

assert.deepEqual(familyWeights(lib[0]!), [100, 900]);
assert.deepEqual(familyWeights(lib[1]!), [400, 700]);

let st: FilterState = toggleValue({}, 'status', 'all');
assert.deepEqual(names2(applyFacets(lib, lf, st)), ['Inter'], 'installed families are not "Active" library fonts');
st = toggleValue(st, 'status', 'installed');
assert.deepEqual(names2(applyFacets(lib, lf, st)), ['Inter', 'Arial']);
st = toggleValue(st, 'status', 'all');
st = toggleValue(st, 'status', 'installed');
assert.equal(st.status, undefined, 'unpicking the last value clears the facet');

// Weight is an overlap: a 100–900 variable family matches Bold.
st = { weight: { kind: 'range', min: 700, max: 900 } };
assert.deepEqual(names2(applyFacets(lib, lf, st)), ['Inter', 'Lora', 'Bebas Condensed']);
assert.equal(describeFacet(facet('weight'), st.weight), '700 Bold or more');
assert.equal(isFacetActive(facet('weight'), { kind: 'range', min: 100, max: 900 }), false, 'a full range is no filter');

// Traits: any vs all.
st = { traits: { kind: 'set', values: ['variable', 'italic'], mode: 'any' } };
assert.deepEqual(names2(applyFacets(lib, lf, st)), ['Inter', 'Lora']);
st = { traits: { kind: 'set', values: ['variable', 'italic'], mode: 'all' } };
assert.deepEqual(names2(applyFacets(lib, lf, st)), ['Inter']);
assert.equal(describeFacet(facet('traits'), st.traits), 'all of Variable, Has italics');

// Counts are computed with the other facets applied, not their own.
st = { width: { kind: 'set', values: ['condensed'], mode: 'any' } };
const widthOpts = facetOptions(lib, lf, st, facet('width') as never);
assert.equal(widthOpts.find((o) => o.value === 'normal')!.count, 4, 'own facet ignored for counts');
const tagOpts = facetOptions(lib, lf, { traits: { kind: 'set', values: ['mono'], mode: 'any' } }, facet('tags') as never);
assert.deepEqual(tagOpts.map((o) => `${o.label}:${o.count}`), ['Code:1', 'UI:1'], 'other facets narrow counts; empty tags drop out');

// Sorting, with the name as tiebreak.
const sorts = librarySorts(() => 0);
const sorted = (key: string, dir: 'asc' | 'desc') => names2(sortItems(lib.slice(), sorts, { key, dir }, (f) => f.name));
assert.deepEqual(sorted('name', 'asc'), ['Arial', 'Bebas Condensed', 'Fira Mono', 'Inter', 'Lora']);
assert.deepEqual(sorted('added', 'desc'), ['Inter', 'Fira Mono', 'Lora', 'Bebas Condensed', 'Arial']);
assert.deepEqual(sorted('styles', 'desc'), ['Lora', 'Inter', 'Arial', 'Bebas Condensed', 'Fira Mono']);
assert.deepEqual(sorted('weight', 'desc')[0], 'Inter');

// -- Google facets
const g = (family: string, o: Partial<GoogleFamilySummary> = {}): GoogleFamilySummary => ({
  family, category: 'sans-serif', styles: 1, variable: false, axes: [], scripts: 1, subsets: ['latin'],
  italic: false, weightRange: [400, 400], tags: [], rank: 0, version: 'v1', lastModified: '2024-01-01',
  status: 'available', library: null, ...o,
});
const cat = [
  g('Roboto', { rank: 0, styles: 18, variable: true, axes: ['wght', 'wdth'], weightRange: [100, 900], italic: true, subsets: ['latin', 'cyrillic', 'greek'], tags: ['/Sans/Neo Grotesque'], status: 'downloaded', library: 'Roboto', lastModified: '2025-06-01' }),
  g('Lora', { rank: 3, category: 'serif', styles: 8, weightRange: [400, 700], italic: true, subsets: ['latin', 'cyrillic'], tags: ['/Serif/Transitional'], status: 'update', library: 'Lora' }),
  g('Pacifico', { rank: 9, category: 'handwriting', subsets: ['latin', 'vietnamese'], tags: ['/Script/Handwritten'] }),
];
const libByName = new Map(lib.map((f) => [f.name, f]));
const gf = googleFacets((x) => (x.library ? libByName.get(x.library) : undefined));
const gfacet = (id: string) => gf.find((f) => f.id === id)!;
const gnames = (xs: GoogleFamilySummary[]) => xs.map((x) => x.family);

assert.deepEqual(gnames(applyFacets(cat, gf, toggleValue({}, 'status', 'downloaded'))), ['Roboto', 'Lora']);
assert.deepEqual(gnames(applyFacets(cat, gf, toggleValue({}, 'status', 'active'))), ['Lora'], 'Lora is partly active in the library; Roboto is not in it');
assert.deepEqual(gnames(applyFacets(cat, gf, toggleValue({}, 'status', 'update'))), ['Lora']);
assert.deepEqual(gnames(applyFacets(cat, gf, { scripts: { kind: 'set', values: ['latin', 'cyrillic'], mode: 'all' } })), ['Roboto', 'Lora']);
assert.deepEqual(gnames(applyFacets(cat, gf, { weight: { kind: 'range', min: 100, max: 300 } })), ['Roboto']);
const tagOpts2 = facetOptions(cat, gf, {}, gfacet('tags') as never);
assert.deepEqual(tagOpts2.map((o) => `${o.group}/${o.label}`), ['Sans/Neo Grotesque', 'Script/Handwritten', 'Serif/Transitional']);
assert.deepEqual(splitTag('/Sans/Humanist'), { group: 'Sans', label: 'Humanist' });
assert.equal(describeFacet(gfacet('scripts'), { kind: 'set', values: ['latin', 'cyrillic-ext', 'greek'], mode: 'any' }), 'Latin, Cyrillic Extended +1');

const gs = googleSorts(() => 0);
assert.deepEqual(gnames(sortItems(cat.slice(), gs, { key: 'popular', dir: 'desc' }, (x) => x.family)), ['Roboto', 'Lora', 'Pacifico']);
assert.deepEqual(gnames(sortItems(cat.slice(), gs, { key: 'updated', dir: 'desc' }, (x) => x.family)), ['Roboto', 'Lora', 'Pacifico']);
assert.deepEqual(gnames(sortItems(cat.slice(), gs, { key: 'scripts', dir: 'desc' }, (x) => x.family)), ['Roboto', 'Lora', 'Pacifico']);

console.log('query: ok');
