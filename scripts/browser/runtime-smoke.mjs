import { readFileSync, writeFileSync } from 'node:fs';
import { initSync, browser_request } from '../../apps/eightbit-studio-web/ui/public/generated/dragon_core.js';
initSync({ module: readFileSync(new URL('../../apps/eightbit-studio-web/ui/public/generated/dragon_core_bg.wasm', import.meta.url)) });
const result = JSON.parse(browser_request(JSON.stringify({ schemaVersion: 1, operation: 'build', source: '10 END\n' }), new Uint8Array()));
if (!result.ok) throw new Error('Cannot generate runtime smoke cassette');
// Original ROM-shaped fixture: an endless 6809 loop, not Dragon firmware or BASIC.
// Only the isolated runtime harness accepts it; the application must reject its hash.
const script = `const frame = document.querySelector('iframe');
const rom = new Uint8Array(16384).fill(0x12); rom.set([0x20, 0xfe], 0); rom[16382]=0x80; rom[16383]=0;
frame.addEventListener('load', () => frame.contentWindow.postMessage({schemaVersion:1,operation:'start',session:1,firmware:rom,media:new Uint8Array(${JSON.stringify(result.result.cassette)})},location.origin));
addEventListener('message', (event) => {if(event.source===frame.contentWindow && event.origin===location.origin) document.getElementById('state').textContent = event.data.state;});`;
const dist = new URL('../../apps/eightbit-studio-web/ui/dist/', import.meta.url);
writeFileSync(new URL('runtime-smoke.js', dist), script);
writeFileSync(new URL('runtime-smoke.html', dist), '<!doctype html><html lang="en-GB"><head><title>Generated-ROM runtime smoke</title></head><body><p id="state" role="status">Starting generated-ROM smoke</p><iframe title="Generated-ROM runtime" src="runtime.html" style="width:700px;height:420px"></iframe><script src="runtime-smoke.js"></script></body></html>');
console.log('Generated-ROM runtime harness created in disposable dist/. Rebuild the UI before staging; never publish this harness.');
