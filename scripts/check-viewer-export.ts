// Checks the style viewer's pairing model and exporter. Run: npm run test:viewer
import { toCss, toJson, fileStem } from '../src/lib/viewer/export';
import { defaultSettings, pairingOf, parsePairing, samePairing } from '../src/lib/viewer/model';
import assert from 'node:assert/strict';
const s = pairingOf(defaultSettings());
s.styles.quote.on = false;
const faces = {
  heading: { family: 'Big "Shoulders"', subfamily: 'Bold', weight: 700, italic: false, postscriptName: 'BS-Bold' },
  body: { family: 'Crimson Pro', subfamily: 'Italic', weight: 400, italic: true, postscriptName: null },
};
const css = toCss('Editorial */ x', s, faces);

assert.ok(css.includes('--font-heading: "Big \\"Shoulders\\"";'));
assert.ok(!css.includes('blockquote'), 'hidden styles left out');
assert.ok(css.includes('font-size: 2.441rem; /* 39.06px */'), 'h1 at 16 × 1.25^4');
assert.ok(!css.slice(2).includes('*/ x'), 'comment cannot be closed by the name');
assert.ok(css.includes('font-style: italic'));
const json = JSON.parse(toJson('Editorial', s, faces));
assert.equal(json.styles.find((x) => x.key === 'quote').enabled, false);
assert.equal(json.styles.find((x) => x.key === 'caption').sizePx, 12.8);
assert.equal(fileStem('Café Pairing #2!'), 'cafe-pairing-2');
assert.equal(fileStem('***'), 'type-pairing');
// Round trip and comparison
const back = parsePairing(JSON.stringify(s));
assert.ok(samePairing(back, s));
assert.ok(samePairing(parsePairing('{"base":16}'), pairingOf(defaultSettings())), 'missing keys take defaults');
assert.ok(samePairing(parsePairing('not json'), pairingOf(defaultSettings())));
const t = { ...s, base: 18 };
assert.ok(!samePairing(t, s));
console.log('ok');
