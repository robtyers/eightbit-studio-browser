// Actual wasm32 versus native guest instructions, board, executive, vectors and PCM.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {initSync,VectrexOriginalRuntime,browser_request} from '../../apps/eightbit-studio-web/ui/public/generated/dragon_core.js';
const hash=b=>createHash('sha256').update(b).digest('hex');
initSync({module:readFileSync(new URL('../../apps/eightbit-studio-web/ui/public/generated/dragon_core_bg.wasm',import.meta.url))});
function trace(template, fault='') {
let source=readFileSync(new URL(`../../families/vectrex/templates/${template}`,import.meta.url),'utf8');
if(fault==='unsupported') source=source.split('start:')[0]+'start: jsr $F687\n bra start\n';
if(fault==='text-list') source=source.split('start:')[0]+'start: ldu #list\n jsr $F385\n bra start\nlist: fcb -8,40,1,0,$1F,$80,0\n';
if(fault==='pattern') source=source.split('start:')[0]+'start: ldx #list\n jsr $F46E\n bra start\nlist: fcb 3,0,0,1\n';
const request=metadata=>JSON.parse(browser_request(JSON.stringify({schemaVersion:1,...metadata}),new Uint8Array()));
const created=request({operation:'create-project',target:'vectrex',kind:'assembly',name:'Parity vectors',source});assert.equal(created.ok,true);
const built=request({operation:'test-project',project:created.result.project});assert.equal(built.ok,true);
const launch={schemaVersion:1,target:'vectrex',source,sourceSha256:hash(source),cartridgeSha256:hash(Buffer.from(built.result.cassette))};
const runtime=VectrexOriginalRuntime.fromSource(7,JSON.stringify(launch));
const commands=[{operation:'advance',cycles:30000},{operation:'key',code:'ArrowRight',pressed:true},{operation:'advance',cycles:90000},{operation:'key',code:'KeyZ',pressed:true},{operation:'advance',cycles:60000},{operation:'release'},{operation:'advance',cycles:60000},{operation:'reset'},{operation:'advance',cycles:90000},{operation:'stop'}];
const actual=commands.map(command=>{const frame=JSON.parse(runtime.command(JSON.stringify({schemaVersion:1,sessionId:7,command})));frame.pixelsSha256=hash(Buffer.from(frame.pixels));const pcm=Buffer.alloc(frame.audio.length*2);frame.audio.forEach((v,i)=>pcm.writeInt16LE(v,i*2));frame.audioSha256=hash(pcm);frame.pixels=null;frame.audio=null;return frame;});
runtime.free();
return actual;
}
const actual={starter:trace('vector-demo.asm'),executive:trace('executive-demo.asm'),packet:trace('packet-demo.asm'),textLists:trace('text-list-demo.asm'),patterns:trace('pattern-demo.asm')};
for(const [key,fault] of [['unsupported','unsupported'],['textListFault','text-list'],['patternFault','pattern']]) {
try { trace('vector-demo.asm',fault); assert.fail('Invalid guest unexpectedly returned'); } catch(error) { actual[key]=String(error); assert.match(actual[key],/^VXO1001:/); }
}
const native=JSON.parse(execFileSync('cargo',['run','--quiet','--offline','--locked','-p','eightbit-browser','--example','vectrex-parity'],{cwd:new URL('../..',import.meta.url),encoding:'utf8',maxBuffer:1024*1024,timeout:120000}));
assert.deepEqual(actual,native);
console.log(`Original Vectrex native/WASM: ${actual.starter.length + actual.executive.length + actual.packet.length + actual.textLists.length + actual.patterns.length} frames identical across starter, text/counted-list, packet-list, text-list and patterned-vector cartridges (pixels, PCM, input, reset and stop); unsupported-service and malformed-text-list and malformed-pattern faults identical`);
