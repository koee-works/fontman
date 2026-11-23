// Checks the Zed theme mapping: every bundled theme, and a deliberately bad
// one, must come out with readable text. Run: npm run test:themes
import assert from 'node:assert/strict';
import builtin from '../src/lib/themes/builtin.json';
import { CHECKED_PAIRS, contrast, isZedFamily, parseColor, themeTokens, type ZedFamily } from '../src/lib/themes/zed';

const rgb = (v: string) => parseColor(v)!.slice(0, 3) as [number, number, number];

assert.deepEqual(parseColor('#abc'), [170, 187, 204, 1]);
assert.deepEqual(parseColor('#11223380'), [17, 34, 51, 128 / 255]);
assert.equal(parseColor('red'), null);

const families = builtin as ZedFamily[];
for (const fam of families) {
  assert.ok(isZedFamily(fam), fam.name);
  for (const t of fam.themes) {
    const { vars, adjusted } = themeTokens(t);
    for (const [fg, bg, min] of CHECKED_PAIRS) {
      const r = contrast(rgb(vars[fg]!), rgb(vars[bg]!));
      assert.ok(r >= min - 0.01, `${t.name}: ${fg} on ${bg} is ${r.toFixed(2)}`);
    }
    console.log(`${t.name.padEnd(20)} ok${adjusted.length ? `, adjusted: ${adjusted.join(', ')}` : ''}`);
  }
}

// A theme with unreadable colours gets fixed, and says so.
const bad = themeTokens({
  name: 'Bad',
  appearance: 'light',
  style: { 'editor.background': '#ffffff', text: '#eeeeee', 'text.muted': '#f0f0f0', 'text.accent': '#ffff00' },
});
for (const [fg, bg, min] of CHECKED_PAIRS) {
  assert.ok(contrast(rgb(bad.vars[fg]!), rgb(bad.vars[bg]!)) >= min - 0.01, `bad: ${fg} on ${bg}`);
}
assert.ok(bad.adjusted.includes('text') && bad.adjusted.includes('accent'), bad.adjusted.join());

// A theme with almost nothing still produces a full set.
const sparse = themeTokens({ name: 'Sparse', appearance: 'dark', style: {} });
assert.equal(Object.keys(sparse.vars).length, 23);
assert.ok(!isZedFamily({ name: 'x', themes: [{ name: 'y', appearance: 'sepia', style: {} }] }));
console.log('ok');
