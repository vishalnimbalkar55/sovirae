const test = require('node:test');
const assert = require('node:assert');
const T = require('../lib/text.js');

test('tidy keeps words and paragraphs but drops layout whitespace', () => {
  assert.equal(T.tidy('  Hello  world \r\n\r\n\r\n  Next\tpara  '), 'Hello world\n\nNext para');
});

test('counts words across scripts and punctuation', () => {
  assert.equal(T.wordCount("It's 3.5 km — naïve café"), 6);
  assert.equal(T.wordCount('   '), 0);
});

test('estimates at 180 words per minute divided by rate', () => {
  assert.equal(T.estimate(360, 1), 'About 2 min');
  assert.equal(T.estimate(360, 2), 'About 1 min');
  assert.equal(T.estimate(170, 1), 'Under 1 min');
  assert.equal(T.estimate(0), '');
});

test('page language accepts BCP-47 tags only', () => {
  assert.equal(T.pageLanguage({ documentElement: { lang: 'fr-CA' } }), 'fr-CA');
  assert.equal(T.pageLanguage({ documentElement: { lang: 'x y' } }), null);
});

test('truncate stays under the limit and prefers sentence ends', () => {
  const text = 'One sentence here. ' + 'x'.repeat(100);
  assert.equal(T.truncate(text, 30), 'One sentence here.');
  assert.equal(T.truncate(text, 40).length, 40, 'hard cut when the sentence ends too early');
  assert.equal(T.truncate('😀😀', 3), '😀');
  assert.equal(T.truncate('short', 40), 'short');
});
