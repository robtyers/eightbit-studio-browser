import { readFileSync, writeFileSync } from 'node:fs';
import { initSync, browser_request } from '../../apps/eightbit-studio-web/ui/public/generated/dragon_core.js';
initSync({ module: readFileSync(new URL('../../apps/eightbit-studio-web/ui/public/generated/dragon_core_bg.wasm', import.meta.url)) });
const call = (request) => JSON.parse(browser_request(JSON.stringify({schemaVersion:1,...request}),new Uint8Array()));
const programmes = {};
for (const target of ['c64-pal','spectrum48','zx81-16k']) {
  const made=call({operation:'create-project',name:'Generated runtime fixture',kind:'basic',source: target==='c64-pal' ? '10 END\n' : '10 STOP\n',target});
  const built=call({operation:'test-project',project:made.result.project});
  if (!built.ok) throw new Error('Cannot build original runtime fixture');
  programmes[target]=built.result.cassette;
}
const script=`const programmes=${JSON.stringify(programmes)};
let frame,session=0;
for (const button of document.querySelectorAll('[data-target]')) button.addEventListener('click',()=>{
  if(frame) { try { frame.contentWindow.__eightbit_stop(); } catch {} frame.remove(); }
  const target=button.dataset.target;
  const rom=new Uint8Array(target==='c64-pal'?20480:target==='spectrum48'?16384:8192);
  if(target==='c64-pal') { rom.set([0x4c,0,0xe0],8192); for(const offset of [16378,16380,16382]) rom.set([0,0xe0],offset); }
  else rom.set([0xc3,0,0],0);
  document.getElementById('state').textContent='Starting generated '+target;
  frame=document.createElement('iframe'); frame.title='Generated firmware runtime'; frame.style='width:850px;height:500px';
  frame.addEventListener('load',()=>frame.contentWindow.postMessage({schemaVersion:1,operation:'start',session:++session,target,firmware:rom,media:new Uint8Array(programmes[target])},location.origin));
  frame.src='classic.html'; document.body.append(frame);
});
addEventListener('message',(event)=>{ if(frame && event.source===frame.contentWindow && event.origin===location.origin) document.getElementById('state').textContent=event.data.state+' · generated fixture'; });
document.getElementById('stop').addEventListener('click',()=>{ if(frame) { frame.contentWindow.__eightbit_stop(); frame.remove(); frame=undefined; } document.getElementById('state').textContent='Stopped · generated fixture'; });`;
const dist=new URL('../../apps/eightbit-studio-web/ui/dist/',import.meta.url);
writeFileSync(new URL('classic-smoke.js',dist),script);
writeFileSync(new URL('classic-smoke.html',dist),'<!doctype html><html lang="en-GB"><title>Generated classic runtime smoke</title><body><p id="state" role="status">Choose a generated firmware fixture; this does not test real BASIC firmware.</p><button data-target="c64-pal">Start C64 fixture</button><button data-target="spectrum48">Start Spectrum fixture</button><button data-target="zx81-16k">Start ZX81 fixture</button><button id="stop">Stop fixture</button><script src="classic-smoke.js"></script></body></html>');
console.log('Disposable generated-ROM classic harness written to dist/. Rebuild the UI before staging.');
