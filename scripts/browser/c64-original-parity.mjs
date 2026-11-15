// Opt-in acceptance of the actual original C64 native/WASM Studio session API.
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {readFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {pathToFileURL} from 'node:url';
import {spawnOwnedPeer} from './original-runtime-process.mjs';
assert.equal(process.argv.length,2,'This acceptance takes no admission overrides');
const root=resolve(import.meta.dirname,'../..');
const generated=resolve(root,'apps/eightbit-studio-web/ui/public/generated');
const module=await import(pathToFileURL(resolve(generated,'dragon_core.js')).href);
await module.default({module_or_path:readFileSync(resolve(generated,'dragon_core_bg.wasm'))});
assert.equal(typeof module.C64OriginalRuntime?.fromSource,'function');
const built=execFileSync('cargo',['build','--offline','--locked','--release','--message-format=json','-p','eightbit-browser','--example','original-runtime-peer'],{cwd:root,encoding:'utf8',stdio:['ignore','pipe','inherit'],timeout:180_000,maxBuffer:16*1024*1024});
const executable=built.split('\n').filter(Boolean).map(line=>JSON.parse(line)).find(value=>value.reason==='compiler-artifact'&&value.target?.name==='original-runtime-peer'&&typeof value.executable==='string')?.executable;
assert.equal(typeof executable,'string');
const owner=spawnOwnedPeer(executable,[],{cwd:root});
let pending,buffer='',sequence=0,runtime,compared=0;
const trace=createHash('sha256');
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
function fail(message){if(pending){const current=pending;pending=undefined;clearTimeout(current.timer);current.reject(new Error(message));}owner.child.kill('SIGTERM');}
owner.child.on('error',()=>fail('Native C64 peer failed'));
owner.child.on('exit',()=>{if(pending)fail('Native C64 peer exited during a request');});
owner.child.stdout.setEncoding('utf8');
owner.child.stdout.on('data',chunk=>{
  buffer+=chunk;if(buffer.length>1024*1024)return fail('Native C64 response exceeds its bound');
  while(buffer.includes('\n')){const end=buffer.indexOf('\n'),line=buffer.slice(0,end);buffer=buffer.slice(end+1);
    try{const value=JSON.parse(line);if(!pending||value.id!==pending.id)return fail('Stale native C64 response');const current=pending;pending=undefined;clearTimeout(current.timer);current.resolve(value);}
    catch{return fail('Malformed native C64 response');}
  }
});
function native(command){assert.equal(pending,undefined);return new Promise((resolve,reject)=>{const id=++sequence;pending={id,resolve,reject,timer:setTimeout(()=>fail('Native C64 request timed out'),30_000)};owner.child.stdin.write(`${JSON.stringify({id,command})}\n`);});}
function compact(frame){
  if(frame.pixels){frame.pixelsSha256=hash(Buffer.from(frame.pixels));delete frame.pixels;}
  if(frame.audio){const audio=Buffer.alloc(frame.audio.length*2);frame.audio.forEach((sample,index)=>audio.writeInt16LE(sample,index*2));frame.audioSamples=frame.audio.length;frame.audioSha256=hash(audio);delete frame.audio;}
  return frame;
}
function wasm(command){switch(command.operation){
  case 'browser':return JSON.parse(module.browser_request(JSON.stringify(command.request),Uint8Array.from(command.bytes)));
  case 'c64-create':runtime=module.C64OriginalRuntime.fromSource(command.session_id,JSON.stringify(command.source_request));return null;
  case 'c64-monitor':return compact(JSON.parse(runtime.command(JSON.stringify(command.request))));
  case 'c64-close':runtime?.free();runtime=undefined;return null;
  default:throw new Error('Unknown C64 acceptance operation');
}}
function canonical(value){return JSON.stringify(value&&typeof value==='object'&&!Array.isArray(value)?Object.fromEntries(Object.keys(value).sort().map(key=>[key,JSON.parse(canonical(value[key]))])):value);}
async function compare(command,expectedFailure=false){
  const result=await native(command);let actual,error;
  try{actual=wasm(command);}catch(value){error=String(value);}
  assert.equal(result.ok,!expectedFailure);assert.equal(Boolean(error),expectedFailure);
  if(expectedFailure){assert.match(result.error,/^C64RUN100[01]:/);assert.match(error,/^C64RUN100[01]:/);}
  else assert.deepEqual(actual,result.result,'C64 native/WASM session observations differ');
  trace.update(canonical(expectedFailure?{error:result.error.split(':')[0]}:actual));compared++;return actual;
}
const monitor=(sessionId,command)=>({operation:'c64-monitor',request:{schemaVersion:1,sessionId,command}});
async function launch(sessionId,source){
  const created=await compare({operation:'browser',request:{schemaVersion:1,operation:'create-project',target:'c64-pal',name:'C64 original acceptance',kind:'basic',source},bytes:[]});assert.equal(created.ok,true);
  const built=await compare({operation:'browser',request:{schemaVersion:1,operation:'test-project',project:created.result.project},bytes:[]});assert.equal(built.ok,true);
  const request={schemaVersion:1,target:'c64-pal',source,sourceSha256:built.result.summary.sourceSha256,prgSha256:built.result.summary.cassetteSha256};
  await compare({operation:'c64-create',session_id:sessionId,source_request:{...request,prgSha256:'0'.repeat(64)}},true);
  await compare({operation:'c64-create',session_id:sessionId,source_request:request});
}
async function until(sessionId,text,max=500){for(let index=0;index<max;index++){const frame=await compare(monitor(sessionId,{operation:'advance',cycles:78624}));assert.equal(frame.state,'running');if(frame.text.includes(text))return frame;}throw new Error(`Original guest did not reach ${text}`);}
try{
  await launch(7,'10 DIM A$(2):A$(1)="HELLO"\n20 DEF FNA(X)=X*X\n30 FOR I=1TO3:S=S+FNA(I):NEXT\n40 PRINT"SESSION";S;LEN(A$(1)):STOP\n50 PRINT"CONT OK":END\n');
  const basic=await until(7,'BREAK IN 40');assert.ok(basic.text.includes('SESSION 14  5'));
  await compare(monitor(8,{operation:'advance',cycles:1}),true);
  await compare(monitor(7,{operation:'advance',cycles:78625}),true);
  await compare(monitor(7,{operation:'type',text:'CONT\r'}));await until(7,'CONT OK');
  await compare(monitor(7,{operation:'key',code:'ArrowLeft',pressed:true}));await compare(monitor(7,{operation:'release'}));
  await compare(monitor(7,{operation:'reset'}));const reset=await until(7,'READY.');assert.ok(!reset.text.includes('SESSION'));
  await compare(monitor(7,{operation:'stop'}));await compare(monitor(7,{operation:'reset'}),true);await compare({operation:'c64-close'});
  await launch(9,readFileSync(resolve(root,'families/commodore/crates/eightbit-commodore-core/tests/fixtures/acceptance.bas'),'utf8'));
  await until(9,'SPRITE + SID ACTIVE');const active=await compare(monitor(9,{operation:'advance',cycles:78624}));assert.ok(active.audioSamples>0);
  await compare(monitor(9,{operation:'type',text:'Q'}));await until(9,'INPUT OK');
  await compare(monitor(9,{operation:'stop'}));await compare({operation:'c64-close'});
  const exited=await owner.stop();assert.equal(exited.code,0);
  console.log(JSON.stringify({schemaVersion:1,stage:'c64-original-studio-session',comparedRequests:compared,nativeWasmEqual:true,sourceBuildBinding:true,arraysFunctionsLoops:true,stopContinue:true,staleBudgetRejection:true,coldReset:true,terminalStop:true,spriteSidPhysicalInput:true,peerExited:true,traceSha256:trace.digest('hex'),wasmSha256:hash(readFileSync(resolve(generated,'dragon_core_bg.wasm')))}));
}finally{runtime?.free();await owner.stop();}
