// Export a pairing as CSS or JSON, for use in a project.
//
// Sizes are given in rem (assuming the browser's default 16px root) with the
// pixel value alongside. Fonts are named by family, with weight and style
// taken from the chosen face, which is how a site would ask for them.

import { STYLES, sizeFor, type PairingSettings, type Role, type StyleKey } from './model';

/** What the exporter needs to know about each chosen face. */
export interface FaceInfo {
  family: string;
  subfamily: string;
  weight: number;
  italic: boolean;
  postscriptName: string | null;
}

const SELECTORS: Record<StyleKey, string> = {
  h1: 'h1',
  h2: 'h2',
  h3: 'h3',
  h4: 'h4',
  lead: '.lead',
  p: 'p',
  list: 'ul,\nol',
  quote: 'blockquote',
  callout: '.callout',
  caption: 'figcaption',
};

const round = (n: number, places: number) => {
  const f = 10 ** places;
  return Math.round(n * f) / f;
};

/** A CSS string literal, escaped. */
const cssString = (s: string) => `"${s.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}"`;

/** A file name from a pairing name: lowercase, hyphens, nothing risky. */
export function fileStem(name: string): string {
  const stem = name
    .normalize('NFKD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 60);
  return stem || 'type-pairing';
}

function faceLabel(f: FaceInfo | undefined): string {
  return f ? `${f.family} ${f.subfamily}` : 'not chosen';
}

export function toCss(name: string, s: PairingSettings, faces: Record<Role, FaceInfo | undefined>): string {
  const ratio = Number(s.ratio);
  const lines: string[] = [];
  // A comment can't contain "*/"; nothing else in a name needs escaping here.
  const safeName = name.replace(/\*\//g, '* /');
  lines.push(
    `/* ${safeName}: type pairing exported from Font Manager.`,
    `   Headings: ${faceLabel(faces.heading)}. Body: ${faceLabel(faces.body)}.`,
    `   Sizes follow a modular scale: ${s.base}px × ${ratio}^step, in rem of a 16px root. */`,
    '',
    ':root {',
  );
  for (const role of ['heading', 'body'] as Role[]) {
    const f = faces[role];
    if (f) lines.push(`  --font-${role}: ${cssString(f.family)};`);
  }
  lines.push(
    `  --leading-heading: ${s.headingLeading};`,
    `  --leading-body: ${s.bodyLeading};`,
    `  --measure: ${s.measure}ch;`,
    '}',
  );

  const block = (selector: string, role: Role, px: number, leading: Role) => {
    const f = faces[role];
    lines.push('', `${selector} {`);
    if (f) lines.push(`  font-family: var(--font-${role});`, `  font-weight: ${f.weight};`);
    if (f?.italic) lines.push('  font-style: italic;');
    lines.push(
      `  font-size: ${round(px / 16, 3)}rem; /* ${round(px, 2)}px */`,
      `  line-height: var(--leading-${leading});`,
      '}',
    );
  };

  // The body sets the default; each style that's switched on follows.
  block('body', 'body', s.base, 'body');
  lines.push('', 'p,\nul,\nol {\n  max-width: var(--measure);\n}');
  for (const d of STYLES) {
    const st = s.styles[d.key];
    if (!st?.on) continue;
    block(SELECTORS[d.key], st.role, sizeFor(d.step, s.base, ratio), d.leading);
  }
  return lines.join('\n') + '\n';
}

export function toJson(name: string, s: PairingSettings, faces: Record<Role, FaceInfo | undefined>): string {
  const ratio = Number(s.ratio);
  const font = (f: FaceInfo | undefined) =>
    f
      ? { family: f.family, style: f.subfamily, weight: f.weight, italic: f.italic, postscriptName: f.postscriptName }
      : null;
  const doc = {
    name,
    fonts: { heading: font(faces.heading), body: font(faces.body) },
    scale: {
      basePx: s.base,
      ratio,
      headingLineHeight: s.headingLeading,
      bodyLineHeight: s.bodyLeading,
      measureCh: s.measure,
    },
    styles: STYLES.map((d) => {
      const px = sizeFor(d.step, s.base, ratio);
      return {
        key: d.key,
        label: d.label,
        enabled: s.styles[d.key]?.on ?? true,
        font: s.styles[d.key]?.role ?? d.role,
        step: d.step,
        sizePx: round(px, 2),
        sizeRem: round(px / 16, 3),
        lineHeight: d.leading === 'heading' ? s.headingLeading : s.bodyLeading,
      };
    }),
  };
  return JSON.stringify(doc, null, 2) + '\n';
}
