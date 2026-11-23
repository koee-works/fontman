// OpenType features in plain words: what each tag is called, where it
// belongs, text that shows it off, and which features can't be on together.
//
// Tags the font lists but nobody would switch by hand (shaping features that
// are always on for the scripts that need them) are left out. Tags this
// catalogue doesn't know are listed under Other by their tag.

import type { FontFeature } from './api';

export type FeatureGroup =
  | 'Ligatures'
  | 'Figures'
  | 'Position'
  | 'Capitals'
  | 'Alternates'
  | 'Stylistic sets'
  | 'Character variants'
  | 'Spacing'
  | 'Other';

export const GROUP_ORDER: FeatureGroup[] = [
  'Ligatures',
  'Figures',
  'Position',
  'Capitals',
  'Alternates',
  'Stylistic sets',
  'Character variants',
  'Spacing',
  'Other',
];

interface Known {
  label: string;
  group: FeatureGroup;
  /** Text that shows the feature; without it, the inspector's sample. */
  sample?: string;
  /** Browsers turn it on unless told otherwise. */
  byDefault?: boolean;
  /** Features that can't sensibly be on at the same time. */
  excludes?: string[];
}

const POSITIONS = ['sups', 'subs', 'sinf', 'ordn', 'numr', 'dnom'];
const others = (tag: string, set: string[]) => set.filter((t) => t !== tag);

const KNOWN: Record<string, Known> = {
  liga: { label: 'Standard ligatures', group: 'Ligatures', sample: 'fi fl ff ffi ffl fj', byDefault: true },
  clig: { label: 'Contextual ligatures', group: 'Ligatures', sample: 'fi fl ffi Th', byDefault: true },
  dlig: { label: 'Discretionary ligatures', group: 'Ligatures', sample: 'ct st sp Th fb fh fk' },
  hlig: { label: 'Historical ligatures', group: 'Ligatures', sample: 'ct st sp ſt' },
  calt: { label: 'Contextual alternates', group: 'Ligatures', sample: '-> => <- 1x2 :-)', byDefault: true },

  lnum: { label: 'Lining figures', group: 'Figures', sample: 'Since 1987: 2,340 sold', excludes: ['onum'] },
  onum: { label: 'Old-style figures', group: 'Figures', sample: 'Since 1987: 2,340 sold', excludes: ['lnum'] },
  pnum: { label: 'Proportional figures', group: 'Figures', sample: '1,111.11  9,876.50', excludes: ['tnum'] },
  tnum: { label: 'Tabular figures', group: 'Figures', sample: '1,111.11  9,876.50', excludes: ['pnum'] },
  zero: { label: 'Slashed zero', group: 'Figures', sample: '0O 2010 0x00FF' },
  frac: { label: 'Fractions', group: 'Figures', sample: '1/2 3/4 7/8 15/16', excludes: ['afrc'] },
  afrc: { label: 'Stacked fractions', group: 'Figures', sample: '1/2 3/4 7/8', excludes: ['frac'] },

  sups: { label: 'Superscript', group: 'Position', sample: 'E=mc2 x2+y3', excludes: others('sups', POSITIONS) },
  subs: { label: 'Subscript', group: 'Position', sample: 'H2O CO2', excludes: others('subs', POSITIONS) },
  sinf: { label: 'Scientific inferiors', group: 'Position', sample: 'H2O C6H12O6', excludes: others('sinf', POSITIONS) },
  ordn: { label: 'Ordinals', group: 'Position', sample: '1st 2nd 3rd 1a 2o No', excludes: others('ordn', POSITIONS) },
  numr: { label: 'Numerators', group: 'Position', sample: '0123456789', excludes: others('numr', POSITIONS) },
  dnom: { label: 'Denominators', group: 'Position', sample: '0123456789', excludes: others('dnom', POSITIONS) },

  smcp: { label: 'Small capitals', group: 'Capitals', sample: 'Small Capitals', excludes: ['pcap'] },
  c2sc: { label: 'Capitals to small capitals', group: 'Capitals', sample: 'NASA and UNESCO', excludes: ['c2pc'] },
  pcap: { label: 'Petite capitals', group: 'Capitals', sample: 'Petite Capitals', excludes: ['smcp'] },
  c2pc: { label: 'Capitals to petite capitals', group: 'Capitals', sample: 'NASA and UNESCO', excludes: ['c2sc'] },
  unic: { label: 'Unicase', group: 'Capitals', sample: 'Unicase Letters' },
  titl: { label: 'Titling', group: 'Capitals', sample: 'TITLING CAPITALS' },
  case: { label: 'Case-sensitive forms', group: 'Capitals', sample: '(HAMBURG) [A–Z] {X} H-O @HI ¿QUÉ?' },
  cpsp: { label: 'Capital spacing', group: 'Capitals', sample: 'CAPITAL SPACING' },

  swsh: { label: 'Swashes', group: 'Alternates', sample: 'Quick Amazing Royal Queen' },
  cswh: { label: 'Contextual swashes', group: 'Alternates', sample: 'Quick Amazing Royal Queen' },
  salt: { label: 'Stylistic alternates', group: 'Alternates' },
  hist: { label: 'Historical forms', group: 'Alternates', sample: 'sense of first sight' },
  nalt: { label: 'Annotation forms', group: 'Alternates', sample: '1 2 3 A B C' },
  ornm: { label: 'Ornaments', group: 'Alternates', sample: 'abcdefghij •' },
  rand: { label: 'Randomize', group: 'Alternates' },
  jalt: { label: 'Justification alternates', group: 'Alternates' },

  kern: { label: 'Kerning', group: 'Spacing', sample: 'AVATAR Toyota WAVE Ty', byDefault: true },
  halt: { label: 'Half-width alternates', group: 'Spacing' },
  palt: { label: 'Proportional alternates', group: 'Spacing' },
};

/** Always-on shaping and features with no meaning in a preview. */
const HIDDEN = new Set([
  'aalt', 'abvf', 'abvm', 'abvs', 'akhn', 'blwf', 'blwm', 'blws', 'ccmp', 'cfar', 'cjct', 'curs',
  'dist', 'fin2', 'fin3', 'fina', 'half', 'haln', 'init', 'isol', 'ljmo', 'locl', 'ltra', 'ltrm',
  'mark', 'med2', 'medi', 'mkmk', 'mset', 'nukt', 'pref', 'pres', 'pstf', 'psts', 'rclt', 'rkrf',
  'rlig', 'rphf', 'rtla', 'rtlm', 'rvrn', 'size', 'stch', 'tjmo', 'vatu', 'vert', 'vjmo', 'vkrn',
  'vpal', 'vrt2', 'vrtr', 'valt', 'vhal', 'dtls', 'flac', 'ssty', 'mgrk',
]);

/** For features with no sample of their own, when no preview text is typed:
 *  the letters alternates most often touch, and every figure. */
export const GENERAL_SAMPLE = 'Ragtime fjords ß & ? 0123456789';

const ORDER = Object.keys(KNOWN);

export interface FeatureView {
  tag: string;
  label: string;
  group: FeatureGroup;
  /** Text that shows it; null means the inspector's sample. */
  sample: string | null;
  byDefault: boolean;
  excludes: string[];
}

const numbered = (tag: string, prefix: string) =>
  tag.startsWith(prefix) && /^\d\d$/.test(tag.slice(2)) ? Number(tag.slice(2)) : null;

export function describe(f: FontFeature): FeatureView | null {
  if (HIDDEN.has(f.tag)) return null;
  const known = KNOWN[f.tag];
  if (known) {
    return {
      tag: f.tag,
      label: known.label,
      group: known.group,
      sample: known.sample ?? null,
      byDefault: !!known.byDefault,
      excludes: known.excludes ?? [],
    };
  }
  const ss = numbered(f.tag, 'ss');
  if (ss !== null) {
    return { tag: f.tag, label: f.name ?? `Set ${ss}`, group: 'Stylistic sets', sample: null, byDefault: false, excludes: [] };
  }
  const cv = numbered(f.tag, 'cv');
  if (cv !== null) {
    return {
      tag: f.tag,
      label: f.name ?? `Variant ${cv}`,
      group: 'Character variants',
      sample: f.chars.length ? f.chars.join(' ') : null,
      byDefault: false,
      excludes: [],
    };
  }
  return { tag: f.tag, label: f.tag, group: 'Other', sample: null, byDefault: false, excludes: [] };
}

/** Features grouped in display order, empty groups left out. */
export function grouped(list: FontFeature[]): { group: FeatureGroup; features: FeatureView[] }[] {
  const views = list.map(describe).filter((v): v is FeatureView => v !== null);
  // Catalogue order within a group (superscript before denominators), then
  // by tag, which numbers stylistic sets and variants in order.
  const rank = (v: FeatureView) => {
    const i = ORDER.indexOf(v.tag);
    return i < 0 ? ORDER.length : i;
  };
  views.sort((a, b) => rank(a) - rank(b) || a.tag.localeCompare(b.tag));
  return GROUP_ORDER.map((group) => ({ group, features: views.filter((v) => v.group === group) })).filter(
    (g) => g.features.length,
  );
}

/** Overrides as CSS: `"smcp" 1, "liga" 0`, or `normal` with none. */
export function featureCss(on: Record<string, boolean>): string {
  const parts = Object.entries(on).map(([tag, v]) => `"${tag}" ${v ? 1 : 0}`);
  return parts.length ? parts.join(', ') : 'normal';
}

/**
 * Switch a feature, returning new overrides. An override equal to the
 * browser's default is dropped, and turning one on turns off those it
 * excludes.
 */
export function toggle(
  on: Record<string, boolean>,
  f: FeatureView,
  value: boolean,
  views: Map<string, FeatureView>,
): Record<string, boolean> {
  const next = { ...on };
  const set = (tag: string, v: boolean) => {
    const def = views.get(tag)?.byDefault ?? false;
    if (v === def) delete next[tag];
    else next[tag] = v;
  };
  set(f.tag, value);
  if (value) for (const x of f.excludes) if (isOn(next, views.get(x))) set(x, false);
  return next;
}

export function isOn(on: Record<string, boolean>, f: FeatureView | undefined): boolean {
  if (!f) return false;
  return on[f.tag] ?? f.byDefault;
}
