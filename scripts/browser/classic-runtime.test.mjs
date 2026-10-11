import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import { gzipSync } from 'node:zlib';
import { test } from 'node:test';
function fixture(decoded = false) {
  const events = new Map(), calls = [], sent = [], files = new Map();
  function node() { return { disabled: true, textContent: '', value: '', dataset: {}, handlers: new Map(), addEventListener(name, handler) { this.handlers.set(name, handler); }, focus() {}, setPointerCapture() {}, dispatchEvent: (event) => calls.push(['key', event.type, event.keyCode]) }; }
  const nodes = Object.fromEntries(['canvas','status','keys','audio','type'].map((name) => [name, node()]));
  const up = node(); up.dataset.key = 'up';
  const document = { hidden:false, getElementById:(id) => nodes[id], querySelectorAll:(selector) => selector === 'button[data-key]' ? [up] : [...Object.values(nodes),up], createElement:node, head:{ append:(script) => calls.push(['script',script.src]) }, addEventListener:(name,handler) => events.set(`document:${name}`,handler) };
  const parent = { postMessage:(message,origin) => sent.push({ message,origin }) };
  const location = { origin:'http://127.0.0.1:1425',href:'http://127.0.0.1:1425/studio/classic.html' };
  function XHR() {} XHR.prototype.open = (...args) => calls.push(['xhr',...args]);
  const window = { fetch:async (url) => { calls.push(['fetch',String(url)]); return new Response(decoded ? 'original wasm fixture' : gzipSync(Buffer.from('original wasm fixture')), { headers: decoded ? { 'Content-Encoding':'gzip' } : {} }); }, addEventListener:(name,handler) => events.set(name,handler), JSMAME:{ machine_exit:() => calls.push(['exit']) }, __eightbit_mameAudio:{ studioStop:() => calls.push(['audio-stop']) } };
  runInNewContext(readFileSync(new URL('../../apps/eightbit-studio-web/ui/public/classic.js',import.meta.url),'utf8'), { window,document,parent,location,URL,Request,Response,DecompressionStream,TransformStream,XMLHttpRequest:XHR,Uint8Array,AbortController,KeyboardEvent:class { constructor(type,props) { Object.assign(this,{type},props); } }, setInterval:(callback) => { events.set('poll',callback); return 1; },clearInterval:() => calls.push(['clear-poll']),cancelAnimationFrame:() => {} });
  const start = (target='spectrum128',extra={},origin=location.origin,sender=parent) => events.get('message')({ source:sender,origin,data:{ schemaVersion:1,operation:'start',session:1,target,firmware:new Uint8Array(target.startsWith('zx81') ? 8192 : 32768).fill(0xa5),media:new Uint8Array([1,2]),...extra } });
  const ready = () => { const module=window.Module; module.callMain=(args) => calls.push(["main",args[0]]); module.FS={ mkdir:(path) => calls.push(['mkdir',path]),writeFile:(path,bytes) => files.set(path,bytes.slice()) }; module._eightbit_ready=() => 1; module._eightbit_key=(code) => { calls.push(['type',code]); return 1; }; module._ZN15running_machine15emscripten_exitEv=() => calls.push(['exit']); module.preRun[0](); module.onRuntimeInitialized(); events.get('poll')?.(); };
  return { window,start,ready,events,calls,sent,files,nodes,up,location,parent,XHR };
}
test('Sinclair runtime validates ownership, target and bounded buffers before loading a fixed script', () => {
  const value=fixture(); value.start('spectrum128',{},'https://untrusted.example'); value.start('unknown'); value.start('spectrum128',{firmware:new Uint8Array(16384)});
  assert.equal(value.window.Module,undefined); value.start();
  assert.deepEqual(value.calls,[['script','generated/mame-z80.js']]);
  assert.equal(value.window.Module.arguments.includes('-nohttp'),true); assert.equal(value.window.Module.arguments.includes('0'),false);
  assert.equal(value.window.Module.arguments[0],'spec128'); assert.equal(value.window.Module.arguments.includes('128K'),true);
  const firmware=new Uint8Array(32768).fill(0xa5),media=new Uint8Array([1,2]);
  const other=fixture(); other.start('spectrum128',{firmware,media}); other.ready();
  assert.equal(other.files.get('/roms/spec128/zx128_0.rom').length,16384); assert.equal(other.files.get('/roms/spec128/zx128_1.rom').length,16384);
  assert.equal(firmware.some(Boolean),false); assert.equal(media.some(Boolean),false);
  assert.equal(other.sent[0].message.state,'ready'); other.start(); assert.equal(other.calls.filter(([name]) => name==='script').length,1);
  const zx=fixture(); zx.start('zx81-1k'); zx.ready(); assert.equal(zx.files.get('/roms/zx81/zx81a.rom').length,8192); assert.equal(zx.window.Module.arguments.includes('1K'),true);
});
test('classic runtime restricts network and decompresses only the fixed bounded WASM response', async () => {
  const value=fixture();
  const response=await value.window.fetch('generated/mame-z80.wasm.gz'); assert.equal(await response.text(),'original wasm fixture');
  const decoded=fixture(true); assert.equal(await (await decoded.window.fetch('generated/mame-z80.wasm.gz')).text(),'original wasm fixture');
  for (const path of ['spectrum.rom','game.tap','https://untrusted.example/mame-z80.wasm.gz']) await assert.rejects(value.window.fetch(path),/not allowed/);
  assert.throws(() => new value.XHR().open('GET','generated/mame-z80.wasm.gz'),/streaming fetch/);
  assert.throws(() => new value.XHR().open('POST','generated/c64.wasm'),/not allowed/);
});
test('classic runtime releases held controls, cancels polling and stops owned audio on suspension', () => {
  const value=fixture(); value.start(); value.ready();
  value.up.handlers.get('pointerdown')({pointerId:1,preventDefault(){}});
  value.window.__eightbit_stop(); value.window.__eightbit_stop();
  assert.deepEqual(value.calls.filter(([name]) => name==='key'),[['key','keydown',38],['key','keyup',38]]);
  assert.equal(value.calls.filter(([name]) => name==='exit').length,1); assert.equal(value.calls.filter(([name]) => name==='audio-stop').length,1);
});

test('startup promise failure reports failure and safely closes an uninitialised runtime', () => {
  const value=fixture(); value.start(); value.events.get('unhandledrejection')();
  assert.equal(value.nodes.status.textContent,'Emulator failed to start.');
  assert.equal(value.sent.at(-1).message.state,'failed');
});

test('cancelling during module load erases pending firmware and never starts late native main', () => {
  const value=fixture(); const firmware=new Uint8Array(32768).fill(0xa5),media=new Uint8Array([1,2]);
  value.start('spectrum128',{firmware,media}); value.window.__eightbit_stop(); value.ready();
  assert.equal(firmware.some(Boolean),false); assert.equal(media.some(Boolean),false);
  assert.equal(value.files.size,0); assert.equal(value.calls.some(([name]) => name==='main'),false);
  assert.equal(value.sent.some(({message}) => message.state==='ready'),false);
});

test('BBC runtime stages five ROMs and the full SSD with the exact fixed Model B options', async () => {
  const value=fixture();
  const firmware=new Uint8Array(66496).fill(0xa5),media=new Uint8Array(204800).fill(0x42);
  value.start('bbc-model-b',{firmware,media:new Uint8Array(65536)});
  assert.equal(value.window.Module,undefined);
  value.start('bbc-model-b',{firmware,media}); value.ready();
  assert.equal(value.calls[0][1],'generated/mame-bbc.js');
  const args=value.window.Module.arguments;
  assert.equal(args[0],'bbcb');
  for (const [option,expected] of [['-fdc','acorn8271,bios=dnfs120'],['-bios','120'],['-ramsize','32K'],['-flop1','/game.ssd'],['-autoboot_command','*EXEC !BOOT\\r'],['-vsp','']]) assert.equal(args[args.indexOf(option)+1],expected);
  assert.equal(args.includes('-cassette'),false);
  for (const [path,size] of [['bbcb/os12.rom',16384],['bbcb/basic2.rom',16384],['bbc_acorn8271/dnfs120.rom',16384],['bbcb/cm62024.bin',16384],['saa5050/saa5050',960]]) assert.equal(value.files.get('/roms/'+path).length,size);
  assert.equal(value.files.get('/game.ssd').length,204800);
  assert.equal(firmware.some(Boolean),false); assert.equal(media.some(Boolean),false);
  assert.equal(await (await value.window.fetch('generated/mame-bbc.wasm.gz')).text(),'original wasm fixture');
  value.nodes.keys.value='PRINT 42'; value.nodes.type.handlers.get('click')(); value.events.get('poll')();
  assert.deepEqual(value.calls.at(-1),['type',80]);
  value.window.__eightbit_stop(); assert.equal(value.calls.filter(([name])=>name==='exit').length,1);
});

test('cancelled BBC loading cannot create firmware files or launch a late emulator', () => {
  const value=fixture(); const firmware=new Uint8Array(66496).fill(0xa5),media=new Uint8Array(204800).fill(0x42);
  value.start('bbc-model-b',{firmware,media}); value.window.__eightbit_stop(); value.ready();
  assert.equal(firmware.some(Boolean),false); assert.equal(media.some(Boolean),false);
  assert.equal(value.files.size,0); assert.equal(value.calls.some(([name])=>name==='main'),false);
});

// SDL2's Emscripten video backend addresses this element by its fixed selector.
test('classic HTML supplies the SDL2 canvas identifier', () => {
  const html=readFileSync(new URL('../../apps/eightbit-studio-web/ui/public/classic.html',import.meta.url),'utf8');
  assert.match(html, /<canvas id="canvas"/);
});

test('BBC sound requires an explicit click and failed resume remains recoverable', async () => {
  const value=fixture(); value.start('bbc-model-b',{firmware:new Uint8Array(66496),media:new Uint8Array(204800)}); value.ready();
  let resumes=0; value.window.__eightbit_mameAudio.studioResume=async () => { resumes++; };
  assert.equal(resumes,0);
  await value.nodes.audio.handlers.get('click')();
  assert.equal(resumes,1); assert.equal(value.nodes.status.textContent,'Sound enabled');
  value.window.__eightbit_mameAudio.studioResume=async () => { throw new Error('Original blocked audio fixture'); };
  await value.nodes.audio.handlers.get('click')();
  assert.equal(value.nodes.status.textContent,'Sound is unavailable');
  assert.equal(value.sent.some(({message}) => message.state==='failed'),false);
  value.window.__eightbit_stop();
  await value.nodes.audio.handlers.get('click')();
  assert.equal(value.nodes.status.textContent,'Sound is unavailable');
});
