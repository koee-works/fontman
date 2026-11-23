// Checks the command palette's fuzzy matching. Run: npm run test:palette
import assert from 'node:assert/strict';
import { fuzzy, highlight, matchCommand } from '../src/lib/palette';

assert.equal(fuzzy('xyz', 'Check for changes'), null);
assert.ok(fuzzy('cfc', 'Check for changes'), 'word starts');
assert.deepEqual(fuzzy('chk', 'Check for changes')!.indices, [0, 1, 4]);
assert.ok(fuzzy('CHECK', 'Check for changes'), 'case-insensitive');
assert.ok(fuzzy('check changes', 'Check for changes'), 'spaces ignored');
// Word starts beat buried letters.
assert.ok(fuzzy('sv', 'Open style viewer')!.score > fuzzy('sv', 'Show advanced')!.score);
// A tighter, shorter match wins.
assert.ok(fuzzy('grid', 'Switch to grid layout')!.score > fuzzy('grid', 'Go right in the details')!.score);
assert.deepEqual(highlight('Abc', [0, 2]), [
  { text: 'A', hit: true },
  { text: 'b', hit: false },
  { text: 'c', hit: true },
]);
const cmd = { id: 'x', group: 'Library', title: 'Check for changes', keywords: 'rescan refresh', run() {} };
assert.ok(matchCommand('refresh', cmd), 'keywords match');
assert.ok(matchCommand('library check', cmd), 'group matches');
assert.ok(matchCommand('check', cmd)!.score > matchCommand('refresh', cmd)!.score, 'title beats keywords');
console.log('ok');
