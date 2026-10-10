import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import { test } from 'node:test';

function fixture() {
  const events = new Map();
  const calls = []; const sent = []; const fetched = []; const files = new Map();
  function node() { return { disabled: true, textContent: '', value: '', dataset: {}, handlers: new Map(), addEventListener(name, handler) { this.handlers.set(name, handler); }, focus() {}, setPointerCapture() {} }; }
  const screen = node(); const status = node(); const keys = node(); const audio = node(); const type = node(); const up = node(); up.dataset.scan = '43';
  const nodes = { screen, status, keys, audio, type };
  const document = { hidden: false, getElementById: (id) => nodes[id], querySelectorAll: (selector) => selector === 'button[data-scan]' ? [up] : [audio, type, up, keys], createElement: () => node(), head: { append: (script) => calls.push(['script', script.src]) }, addEventListener: (name, handler) => events.set(`document:${name}`, handler) };
  const parent = { postMessage: (message, origin) => sent.push({ message, origin }) };
  const location = { origin: 'http://127.0.0.1:1425', href: 'http://127.0.0.1:1425/studio/runtime.html' };
  function XHR() {} XHR.prototype.open = function (...args) { calls.push(['xhr', ...args]); };
  const window = { fetch: async (url) => { fetched.push(url); return {}; }, addEventListener: (name, handler) => events.set(name, handler) };
  runInNewContext(readFileSync(new URL('../../apps/eightbit-studio-web/ui/public/runtime.js', import.meta.url), 'utf8'), { window, document, parent, location, URL, Request, XMLHttpRequest: XHR, Uint8Array });
  const start = (extra = {}, origin = location.origin, sender = parent) => events.get('message')({ source: sender, origin, data: { schemaVersion: 1, operation: 'start', session: 1, firmware: new Uint8Array(16384).fill(0xa5), media: new Uint8Array([1, 2]), ...extra } });
  return { window, start, calls, sent, fetched, files, events, parent, location, up, status, keys, XHR };
}
test('runtime starts only for the owning origin and bounded byte buffers', () => {
  const value = fixture(); value.start({}, 'https://untrusted.example'); value.start({}, value.location.origin, {}); value.start({ firmware: new Uint8Array(1) });
  assert.equal(value.window.Module, undefined);
  value.start(); assert.equal(value.calls.length, 1); assert.equal(value.calls[0][1], 'generated/xroar.js');
  assert.equal(value.window.Module.arguments[0], '-no-c');
  assert.equal(value.window.Module.arguments.includes('-no-cart'), true);
  const module = value.window.Module;
  module.FS = { writeFile: (path, bytes) => value.files.set(path, bytes.slice()) };
  module.preRun[0](); assert.equal(value.files.get('/dragon32.rom').length, 16384); assert.deepEqual([...value.files.get('/game.cas')], [1, 2]);
  value.start(); assert.equal(value.calls.length, 1);
});
test('runtime allows only the fixed WASM fetch, never firmware, media or remote requests', async () => {
  const value = fixture();
  await value.window.fetch('generated/xroar.wasm'); assert.equal(value.fetched.length, 1);
  for (const url of ['dragon32.rom', 'game.cas', 'https://untrusted.example/xroar.wasm']) await assert.rejects(value.window.fetch(url), /not allowed/);
  const xhr = new value.XHR(); assert.throws(() => xhr.open('GET', 'dragon32.rom'), /not allowed/);
  assert.equal(value.fetched.length, 1);
});
test('runtime releases held controls and closes owned audio before teardown', async () => {
  const value = fixture(); value.start(); const calls = [];
  const audio = { state: 'running', close: async () => { audio.state = 'closed'; } };
  value.window.Module.ccall = (...args) => calls.push(args); value.window.Module.SDL2 = { audioContext: audio };
  value.window.ui_done_initialising(); assert.equal(value.sent[0].message.state, 'ready');
  value.up.handlers.get('pointerdown')({ pointerId: 1, preventDefault() {} });
  assert.equal(calls[0][0], 'wasm_scan_press'); value.window.__eightbit_stop();
  await Promise.resolve(); assert.equal(calls[1][0], 'wasm_scan_release'); assert.equal(audio.state, 'closed');
});
