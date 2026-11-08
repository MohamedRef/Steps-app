// Runs the production Study foreground script without a browser
// (adapted from batna-app main: crates/app/tests/study-visibility.test.cjs).
const assert = require('node:assert/strict');
const { test } = require('node:test');
const fs = require('node:fs');
const path = require('node:path');

const source = fs.readFileSync(path.join(__dirname, '../src/study/foreground.js'), 'utf8');
const AsyncFunction = Object.getPrototypeOf(async function () {}).constructor;

function harness() {
  const handlers = { document: new Map(), window: new Map() };
  let state = 'visible';
  const sent = [];
  const window = { addEventListener: (kind, cb) => handlers.window.set(kind, cb) };
  const document = {
    addEventListener: (kind, cb) => handlers.document.set(kind, cb),
    get visibilityState() { return state; },
  };
  const run = new AsyncFunction('window', 'document', 'dioxus', source);
  const start = (received = sent) => {
    run(window, document, { send: message => received.push(message) });
  };
  const setState = (next) => { state = next; handlers.document.get('visibilitychange')?.(); };
  const pageShown = () => handlers.window.get('pageshow')?.();
  return { start, setState, pageShown, sent, handlers };
}

test('coming back signals only while the page is visible', () => {
  const h = harness();
  h.start();
  h.setState('hidden');
  assert.deepEqual(h.sent, []);
  h.setState('visible');
  assert.deepEqual(h.sent, ['visible']);
  h.setState('hidden');
  h.pageShown();
  assert.deepEqual(h.sent, ['visible'], 'pageshow while hidden does not restart the answer clock');
  h.setState('visible');
  assert.deepEqual(h.sent, ['visible', 'visible']);
});

test('a second start replaces the channel without adding listeners', () => {
  const h = harness();
  h.start();
  const latest = [];
  h.start(latest);
  assert.equal(h.handlers.document.size, 1);
  assert.equal(h.handlers.window.size, 1);
  h.setState('hidden');
  h.setState('visible');
  assert.deepEqual(h.sent, []);
  assert.deepEqual(latest, ['visible']);
});
