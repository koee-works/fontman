// Checks the OpenType feature catalogue and toggling. Run: npm run test:features
import assert from 'node:assert/strict';
import { describe, featureCss, grouped, isOn, toggle, type FeatureView } from '../src/lib/features';

const f = (tag: string, name: string | null = null, chars: string[] = []) => ({ tag, name, chars });
const list = [
  f('liga'), f('kern'), f('lnum'), f('onum'), f('tnum'), f('pnum'), f('smcp'), f('sups'), f('subs'),
  f('ss01', 'Open digits'), f('ss02'), f('cv05', 'Lower-case L with tail', ['l']), f('ccmp'), f('locl'), f('zzzz'),
];

// Shaping features are hidden; unknown tags land in Other; names come through.
const groups = grouped(list);
const all = new Map(groups.flatMap((g) => g.features).map((v) => [v.tag, v]));
assert.equal(all.has('ccmp'), false);
assert.equal(all.has('locl'), false);
assert.equal(all.get('zzzz')!.group, 'Other');
assert.equal(all.get('ss01')!.label, 'Open digits');
assert.equal(all.get('ss02')!.label, 'Set 2');
assert.equal(all.get('cv05')!.sample, 'l');
assert.equal(all.size, 13);
assert.deepEqual(groups.find((g) => g.group === 'Position')!.features.map((x) => x.tag), ['sups', 'subs']);
assert.deepEqual(groups.map((g) => g.group), ['Ligatures', 'Figures', 'Position', 'Capitals', 'Stylistic sets', 'Character variants', 'Spacing', 'Other']);

const v = (tag: string) => all.get(tag) as FeatureView;

// Defaults: ligatures and kerning are on until switched off.
assert.equal(isOn({}, v('liga')), true);
assert.equal(isOn({}, v('smcp')), false);
let on = toggle({}, v('liga'), false, all);
assert.deepEqual(on, { liga: false });
assert.equal(featureCss(on), '"liga" 0');
// Back to its default: the override goes, rather than "liga" 1.
on = toggle(on, v('liga'), true, all);
assert.deepEqual(on, {});
assert.equal(featureCss(on), 'normal');

// Exclusive pairs: old-style turns lining off, tabular turns proportional off.
on = toggle({}, v('lnum'), true, all);
on = toggle(on, v('onum'), true, all);
assert.deepEqual(on, { onum: true });
on = toggle(on, v('pnum'), true, all);
on = toggle(on, v('tnum'), true, all);
assert.deepEqual(on, { onum: true, tnum: true });
// One position at a time.
on = toggle(toggle({}, v('sups'), true, all), v('subs'), true, all);
assert.deepEqual(on, { subs: true });
// Unrelated features stack.
on = toggle(toggle({}, v('smcp'), true, all), v('ss01'), true, all);
assert.equal(featureCss(on), '"smcp" 1, "ss01" 1');

assert.equal(describe(f('aalt')), null);
console.log('ok');
