const test = require('node:test');
const assert = require('node:assert');
const { createTrigger } = require('../lib/trigger.js');

function clock() {
  let t = 0;
  return { now: () => t, advance: (ms) => (t += ms) };
}
const key = (k, extra = {}) => ({ key: k, repeat: false, isComposing: false, altGraph: false, ...extra });

test('long press arms and latches after the hold time', () => {
  const c = clock();
  const tr = createTrigger({ trigger: 'alt', holdMs: 450, now: c.now });
  assert.equal(tr.keydown(key('Alt')), 'hold-start');
  c.advance(300);
  assert.equal(tr.tick(), null);
  c.advance(200);
  assert.equal(tr.tick(), 'armed');
  assert.equal(tr.keyup(key('Alt')), null, 'releasing does not disarm');
  assert.equal(tr.state, 'armed');
});

test('releasing early cancels', () => {
  const c = clock();
  const tr = createTrigger({ trigger: 'control', holdMs: 450, now: c.now });
  tr.keydown(key('Control'));
  c.advance(100);
  assert.equal(tr.keyup(key('Control')), 'cancel');
  c.advance(1000);
  assert.equal(tr.tick(), null);
});

test('ordinary shortcuts cancel the hold and are not blocked', () => {
  const c = clock();
  const tr = createTrigger({ trigger: 'control', holdMs: 450, now: c.now });
  tr.keydown(key('Control'));
  assert.equal(tr.keydown(key('c', { ctrlKey: true })), 'cancel');
  c.advance(1000);
  assert.equal(tr.tick(), null);
  tr.keydown(key('Control'));
  assert.equal(tr.pointerdown(), 'cancel', 'Ctrl+click cancels');
});

test('editable fields, repeats, IME, AltGr, and chords are ignored', () => {
  const tr = createTrigger({ trigger: 'alt' });
  assert.equal(tr.keydown(key('Alt'), { editable: true }), null);
  assert.equal(tr.keydown(key('Alt', { repeat: true })), null);
  assert.equal(tr.keydown(key('Alt', { isComposing: true })), null);
  assert.equal(tr.keydown(key('Alt', { altGraph: true })), null);
  assert.equal(tr.keydown(key('Alt', { shiftKey: true })), null);
  assert.equal(tr.state, 'idle');
});

test('shortcut-only mode never arms from the keyboard but can be armed directly', () => {
  const tr = createTrigger({ trigger: 'shortcut' });
  assert.equal(tr.keydown(key('Alt')), null);
  assert.equal(tr.arm(), 'armed');
});

test('hold duration is clamped to 300–1000 ms', () => {
  const tr = createTrigger();
  tr.configure({ holdMs: 50 });
  assert.equal(tr.holdMs, 300);
  tr.configure({ holdMs: 5000 });
  assert.equal(tr.holdMs, 1000);
});
