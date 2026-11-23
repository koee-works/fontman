// Zed themes, mapped onto the app's design tokens.
//
// A Zed theme is JSON: a family with themes, each light or dark, each a set
// of named colours ("editor.background", "text.muted", "text.accent" ...).
// The schema is public (zed.dev/schema/themes/v0.2.0.json). This maps the
// UI colours onto app.css's tokens, derives the few the app needs that Zed
// doesn't name (tinted selection, soft backgrounds, text on filled buttons),
// and then makes sure every text colour stays readable: anything under WCAG
// AA contrast is nudged until it passes, and reported.

export interface ZedTheme {
  name: string;
  appearance: 'light' | 'dark';
  style: Record<string, unknown>;
  players?: { cursor?: string }[];
}

export interface ZedFamily {
  name: string;
  author?: string;
  themes: ZedTheme[];
}

type RGB = [number, number, number];
type RGBA = [number, number, number, number];

/** #rgb, #rgba, #rrggbb or #rrggbbaa. */
export function parseColor(v: unknown): RGBA | null {
  if (typeof v !== 'string') return null;
  const m = v.trim().match(/^#([0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/i);
  if (!m) return null;
  let h = m[1]!;
  if (h.length <= 4) h = [...h].map((c) => c + c).join('');
  const n = (i: number) => parseInt(h.slice(i, i + 2), 16);
  return [n(0), n(2), n(4), h.length === 8 ? n(6) / 255 : 1];
}

const hex = ([r, g, b]: RGB) =>
  '#' + [r, g, b].map((x) => Math.round(Math.min(255, Math.max(0, x))).toString(16).padStart(2, '0')).join('');

/** A translucent colour as it looks on a solid background. */
const over = ([r, g, b, a]: RGBA, [br, bg, bb]: RGB): RGB => [
  r * a + br * (1 - a),
  g * a + bg * (1 - a),
  b * a + bb * (1 - a),
];

/** `t` of the way from `b` to `a`. */
const mix = (a: RGB, b: RGB, t: number): RGB => [
  a[0] * t + b[0] * (1 - t),
  a[1] * t + b[1] * (1 - t),
  a[2] * t + b[2] * (1 - t),
];

function luminance([r, g, b]: RGB): number {
  const c = [r, g, b].map((x) => {
    const s = x / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * c[0]! + 0.7152 * c[1]! + 0.0722 * c[2]!;
}

export function contrast(a: RGB, b: RGB): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi! + 0.05) / (lo! + 0.05);
}

const WHITE: RGB = [255, 255, 255];
const BLACK: RGB = [0, 0, 0];

/** The app's own values for what Zed themes don't define. */
const FALLBACK = {
  light: { magenta: '#b8005f', red: '#c22a1d', amber: '#8a5a00', accent: '#006a93', ink: '#15181c' },
  dark: { magenta: '#ff6fb3', red: '#ff7b6e', amber: '#f0b64c', accent: '#4cc3ec', ink: '#e9ecef' },
};

const TOKEN_NAMES: Record<string, string> = {
  ink: 'text',
  muted: 'muted text',
  cyan: 'accent',
  'cyan-solid': 'accent buttons',
  magenta: 'name-conflict colour',
  red: 'error colour',
  'red-solid': 'danger buttons',
  amber: 'warning colour',
  focus: 'focus ring',
};

export interface ThemeTokens {
  /** CSS custom properties, `--sheet` and so on. */
  vars: Record<string, string>;
  /** Plain names of colours that were adjusted for contrast. */
  adjusted: string[];
}

export function themeTokens(t: ZedTheme): ThemeTokens {
  const dark = t.appearance === 'dark';
  const fb = dark ? FALLBACK.dark : FALLBACK.light;
  const get = (...keys: string[]): RGBA | null => {
    for (const k of keys) {
      const c = parseColor(t.style[k]);
      if (c) return c;
    }
    return null;
  };
  const solid = (c: RGBA | null, on: RGB, fallback: RGB): RGB => (c ? over(c, on) : fallback);
  const fbc = (v: string) => over(parseColor(v)!, WHITE);

  // Surfaces. The sheet, where specimens sit, is Zed's editor.
  const sheet = solid(get('editor.background', 'surface.background', 'background'), dark ? BLACK : WHITE, dark ? [23, 25, 28] : WHITE);
  const lightbox = solid(get('panel.background', 'surface.background', 'background'), sheet, sheet);
  const raised = solid(get('elevated_surface.background', 'surface.background'), sheet, sheet);
  const hover = solid(get('ghost_element.hover', 'element.hover'), sheet, mix(dark ? WHITE : BLACK, sheet, 0.06));
  const pressed = solid(get('ghost_element.active', 'element.active'), sheet, mix(dark ? WHITE : BLACK, sheet, 0.1));
  const line = solid(get('border'), sheet, mix(dark ? WHITE : BLACK, sheet, 0.2));
  const faint = solid(get('border.variant', 'border'), sheet, mix(dark ? WHITE : BLACK, sheet, 0.1));

  // Ink and roles.
  let ink = solid(get('text'), sheet, fbc(fb.ink));
  let muted = solid(get('text.muted'), sheet, mix(ink, sheet, 0.65));
  const accentRaw = get('text.accent', 'icon.accent', 'info') ?? parseColor(t.players?.[0]?.cursor) ?? parseColor(fb.accent)!;
  let cyan = over(accentRaw, sheet);
  let cyanSolid = cyan;
  let red = solid(get('error'), sheet, fbc(fb.red));
  let redSolid = red;
  let amber = solid(get('warning'), sheet, fbc(fb.amber));
  let magenta = fbc(fb.magenta);
  let focus = solid(get('border.focused'), sheet, cyan);

  // Tinted backgrounds Zed doesn't name, derived from the role colours.
  const tint = dark ? 0.18 : 0.12;
  const selected = mix(cyan, sheet, tint);
  const cyanSoft = selected;
  const redSoft = solid(get('error.background'), sheet, mix(red, sheet, tint));
  const amberSoft = solid(get('warning.background'), sheet, mix(amber, sheet, tint));
  const magentaSoft = mix(magenta, sheet, tint);

  // Readability: nudge each text colour toward the extreme (white on dark,
  // black on light) until it clears `min` against all its backgrounds.
  const adjusted = new Set<string>();
  const extreme = dark ? WHITE : BLACK;
  const fix = (name: string, fg: RGB, bgs: RGB[], min = 4.5): RGB => {
    let c = fg;
    for (let i = 0; i < 40 && bgs.some((b) => contrast(c, b) < min); i++) {
      c = mix(extreme, c, 0.06);
      adjusted.add(name);
    }
    return c;
  };
  ink = fix('ink', ink, [sheet, lightbox, raised]);
  muted = fix('muted', muted, [sheet, lightbox, hover]);
  cyan = fix('cyan', cyan, [sheet, lightbox, selected]);
  magenta = fix('magenta', magenta, [sheet, magentaSoft]);
  red = fix('red', red, [sheet, redSoft]);
  amber = fix('amber', amber, [sheet, amberSoft]);
  focus = fix('focus', focus, [sheet], 3);

  // Text on filled buttons: white or black, whichever reads better; if
  // neither does, move the fill away from the text until one does.
  const onFill = (name: string, fill: RGB): [RGB, RGB] => {
    const on = contrast(WHITE, fill) >= contrast(BLACK, fill) ? WHITE : BLACK;
    let f = fill;
    const away = on === WHITE ? BLACK : WHITE;
    for (let i = 0; i < 40 && contrast(on, f) < 4.5; i++) {
      f = mix(away, f, 0.06);
      adjusted.add(name);
    }
    return [f, on];
  };
  let cyanOn: RGB;
  let redOn: RGB;
  [cyanSolid, cyanOn] = onFill('cyan-solid', cyanSolid);
  [redSolid, redOn] = onFill('red-solid', redSolid);

  const vars: Record<string, string> = {
    '--lightbox': hex(lightbox),
    '--sheet': hex(sheet),
    '--raised': hex(raised),
    '--hover': hex(hover),
    '--pressed': hex(pressed),
    '--selected': hex(selected),
    '--ink': hex(ink),
    '--muted': hex(muted),
    '--faint': hex(faint),
    '--line': hex(line),
    '--cyan': hex(cyan),
    '--cyan-solid': hex(cyanSolid),
    '--cyan-on': hex(cyanOn),
    '--cyan-soft': hex(cyanSoft),
    '--magenta': hex(magenta),
    '--magenta-soft': hex(magentaSoft),
    '--red': hex(red),
    '--red-solid': hex(redSolid),
    '--red-on': hex(redOn),
    '--red-soft': hex(redSoft),
    '--amber': hex(amber),
    '--amber-soft': hex(amberSoft),
    '--focus': hex(focus),
  };
  return { vars, adjusted: [...adjusted].map((k) => TOKEN_NAMES[k] ?? k) };
}

/** The text/background pairs the app relies on, for checking a theme. */
export const CHECKED_PAIRS: [string, string, number][] = [
  ['--ink', '--sheet', 4.5],
  ['--ink', '--raised', 4.5],
  ['--muted', '--sheet', 4.5],
  ['--muted', '--lightbox', 4.5],
  ['--muted', '--hover', 4.5],
  ['--cyan', '--sheet', 4.5],
  ['--cyan', '--lightbox', 4.5],
  ['--cyan', '--selected', 4.5],
  ['--cyan-on', '--cyan-solid', 4.5],
  ['--magenta', '--sheet', 4.5],
  ['--magenta', '--magenta-soft', 4.5],
  ['--red', '--sheet', 4.5],
  ['--red', '--red-soft', 4.5],
  ['--red-on', '--red-solid', 4.5],
  ['--amber', '--sheet', 4.5],
  ['--amber', '--amber-soft', 4.5],
];

/** Is this a usable Zed theme family? Checked before importing. */
export function isZedFamily(v: unknown): v is ZedFamily {
  if (!v || typeof v !== 'object') return false;
  const f = v as ZedFamily;
  return (
    typeof f.name === 'string' &&
    Array.isArray(f.themes) &&
    f.themes.length > 0 &&
    f.themes.every(
      (t) =>
        t &&
        typeof t.name === 'string' &&
        (t.appearance === 'light' || t.appearance === 'dark') &&
        t.style !== null &&
        typeof t.style === 'object',
    )
  );
}
