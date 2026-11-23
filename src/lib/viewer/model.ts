// The style viewer: paragraph styles laid out on a modular type scale.
//
// Every size is base × ratio^step. Changing the base or the ratio
// re-proportions the whole page at once, which is how type systems are
// usually built, and lets a pairing be judged across sizes in seconds.

export type Role = 'heading' | 'body';

export type StyleKey =
  | 'h1'
  | 'h2'
  | 'h3'
  | 'h4'
  | 'lead'
  | 'p'
  | 'list'
  | 'quote'
  | 'callout'
  | 'caption';

export interface StyleDef {
  key: StyleKey;
  label: string;
  /** Exponent on the scale: size = base × ratio^step. */
  step: number;
  /** Which font it uses unless changed. */
  role: Role;
  /** Headings take the heading line height; text takes the body line height. */
  leading: Role;
}

export const STYLES: StyleDef[] = [
  { key: 'h1', label: 'Heading 1', step: 4, role: 'heading', leading: 'heading' },
  { key: 'h2', label: 'Heading 2', step: 3, role: 'heading', leading: 'heading' },
  { key: 'h3', label: 'Heading 3', step: 2, role: 'heading', leading: 'heading' },
  { key: 'h4', label: 'Heading 4', step: 1, role: 'heading', leading: 'heading' },
  { key: 'lead', label: 'Lead', step: 0.5, role: 'body', leading: 'body' },
  { key: 'p', label: 'Paragraph', step: 0, role: 'body', leading: 'body' },
  { key: 'list', label: 'List', step: 0, role: 'body', leading: 'body' },
  { key: 'quote', label: 'Quote', step: 1, role: 'heading', leading: 'heading' },
  { key: 'callout', label: 'Callout', step: 0, role: 'body', leading: 'body' },
  { key: 'caption', label: 'Caption', step: -1, role: 'body', leading: 'body' },
];

export const RATIOS = [
  { value: '1.067', label: 'Minor second, 1.067' },
  { value: '1.125', label: 'Major second, 1.125' },
  { value: '1.2', label: 'Minor third, 1.2' },
  { value: '1.25', label: 'Major third, 1.25' },
  { value: '1.333', label: 'Perfect fourth, 1.333' },
  { value: '1.414', label: 'Augmented fourth, 1.414' },
  { value: '1.5', label: 'Perfect fifth, 1.5' },
  { value: '1.618', label: 'Golden ratio, 1.618' },
];

export function sizeFor(step: number, base: number, ratio: number): number {
  return base * Math.pow(ratio, step);
}

/** The sample page: blocks in reading order. Ids keep edits per block. */
export interface Block {
  id: string;
  style: StyleKey;
  /** List blocks hold one item per line. */
  text: string;
}

export const DEFAULT_BLOCKS: Block[] = [
  { id: 'title', style: 'h1', text: 'The shape of reading' },
  {
    id: 'intro',
    style: 'lead',
    text: 'A typeface does most of its work unnoticed. Set well, it lets an idea arrive without friction; set badly, every line costs a little attention.',
  },
  {
    id: 'p1',
    style: 'p',
    text: 'Start with the text people will spend the longest reading. Choose its size and spacing first, at the length a line will really run, and only then build headings above it. A scale that begins from comfortable body text tends to stay comfortable as it grows.',
  },
  { id: 'scale', style: 'h2', text: 'Choosing a scale' },
  {
    id: 'p2',
    style: 'p',
    text: 'A modular scale multiplies one size by the same ratio again and again. Small ratios suit dense interfaces with many levels; large ratios suit editorial pages that need a clear, dramatic hierarchy.',
  },
  {
    id: 'list',
    style: 'list',
    text: 'Set the body size first\nPick a ratio that fits the number of levels you need\nCheck every level at its real line length',
  },
  { id: 'rhythm', style: 'h3', text: 'Rhythm and measure' },
  {
    id: 'p3',
    style: 'p',
    text: 'Between forty-five and seventy-five characters per line is a reliable range for running text. Longer lines need more leading, so the eye can find its way back to the start of the next one.',
  },
  {
    id: 'callout',
    style: 'callout',
    text: 'Headings usually want tighter leading than body text. At large sizes, the default spacing of most fonts looks loose.',
  },
  { id: 'notes', style: 'h4', text: 'Small text matters too' },
  {
    id: 'p4',
    style: 'p',
    text: 'Captions and footnotes are where a typeface is tested hardest. Look at the figures, the punctuation and the italic here as closely as at the headline.',
  },
  { id: 'caption', style: 'caption', text: 'Figure 1. Body text, caption and heading set from a single scale.' },
  {
    id: 'quote',
    style: 'quote',
    text: '“The best type is the one you stop noticing by the second paragraph.”',
  },
];

export interface StyleSetting {
  on: boolean;
  role: Role;
}

export interface FontChoice {
  family: string;
  faceId: string;
}

export interface ViewerSettings {
  heading: FontChoice | null;
  body: FontChoice | null;
  base: number;
  ratio: string;
  headingLeading: number;
  bodyLeading: number;
  /** Line length of running text, in characters. */
  measure: number;
  styles: Record<StyleKey, StyleSetting>;
  texts: Record<string, string>;
}

export function defaultSettings(): ViewerSettings {
  return {
    heading: null,
    body: null,
    base: 16,
    ratio: '1.25',
    headingLeading: 1.15,
    bodyLeading: 1.55,
    measure: 66,
    styles: Object.fromEntries(STYLES.map((s) => [s.key, { on: true, role: s.role }])) as Record<
      StyleKey,
      StyleSetting
    >,
    texts: Object.fromEntries(DEFAULT_BLOCKS.map((b) => [b.id, b.text])),
  };
}

/** What a saved pairing holds: everything but the sample text. */
export type PairingSettings = Omit<ViewerSettings, 'texts'>;

/** The pairing part of settings, with a fixed key order so two can be compared as JSON. */
export function pairingOf(s: PairingSettings): PairingSettings {
  return {
    heading: s.heading ? { family: s.heading.family, faceId: s.heading.faceId } : null,
    body: s.body ? { family: s.body.family, faceId: s.body.faceId } : null,
    base: s.base,
    ratio: s.ratio,
    headingLeading: s.headingLeading,
    bodyLeading: s.bodyLeading,
    measure: s.measure,
    styles: Object.fromEntries(
      STYLES.map((d) => [d.key, { on: s.styles[d.key]?.on ?? true, role: s.styles[d.key]?.role ?? d.role }]),
    ) as Record<StyleKey, StyleSetting>,
  };
}

/**
 * Read a saved pairing. Settings added in later versions take their
 * defaults, and anything unreadable gives the defaults.
 */
export function parsePairing(json: string): PairingSettings {
  const d = defaultSettings();
  try {
    const saved = JSON.parse(json) as Partial<PairingSettings>;
    return pairingOf({ ...d, ...saved, styles: { ...d.styles, ...(saved.styles ?? {}) } });
  } catch {
    return pairingOf(d);
  }
}

export const samePairing = (a: PairingSettings, b: PairingSettings) =>
  JSON.stringify(pairingOf(a)) === JSON.stringify(pairingOf(b));
