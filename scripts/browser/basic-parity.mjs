import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const corpus = JSON.parse(readFileSync(resolve(root, 'families/dragon/crates/eightbit-dragon-emulator/examples/basic-acceptance.json'), 'utf8'));
assert.equal(corpus.schemaVersion, 1);
assert.ok(Array.isArray(corpus.commands) && corpus.commands.length <= 32);
const generated = resolve(root, 'apps/eightbit-studio-web/ui/public/generated');
const module = await import(pathToFileURL(resolve(generated, 'dragon_core.js')).href);
await module.default({ module_or_path: readFileSync(resolve(generated, 'dragon_core_bg.wasm')) });
const runtime = module.MonitorRuntime.basicDevelopment(1);
const request = command => JSON.parse(runtime.command(JSON.stringify({ schemaVersion: 1, sessionId: 1, command })));
const audio = createHash('sha256');
const states = createHash('sha256');
let audioSamples = 0;
let nonSilentAudio = false;
try {
  for (const text of corpus.commands) {
    assert.ok(text === null || typeof text === 'string' && text.length <= 251);
    request(text === null ? { operation: 'reset' } : { operation: 'type', text });
    const settleFrames = text?.startsWith('CSAVE') ? 128 : 48;
    for (let n = 0; n < Math.ceil((text?.length ?? 0) * 24_000 / 32_768) + settleFrames; n++) {
      const step = request({ operation: 'advance', cycles: 32768 });
      assert.equal(step.state, 'running');
      assert.ok(!step.text.includes(' ERROR'), 'original guest corpus must execute');
      assert.equal(step.audioSampleRate, 48000);
      assert.ok(step.audio.length <= 4096);
      states.update(JSON.stringify(step));
      const samples = Buffer.alloc(step.audio.length * 2);
      step.audio.forEach((sample, index) => samples.writeInt16LE(sample, index * 2));
      audio.update(samples); audioSamples += step.audio.length;
      nonSilentAudio ||= step.audio.some(sample => sample !== 0);
    }
    if (text?.includes('CURVE READY')) assert.match(request({ operation: 'state' }).text, /CURVE READY/);
    if (text?.includes('PAINT READY')) assert.match(request({ operation: 'state' }).text, /PAINT READY/);
    if (text?.includes('MID READY')) assert.match(request({ operation: 'state' }).text, /MID READY/);
    if (text?.includes('TIMER READY')) assert.match(request({ operation: 'state' }).text, /TIMER READY/);
    for (const marker of ['RADIX READY', 'NUMBER READY', 'SAVE READY', 'ASCII READY', 'RND READY']) {
      if (text?.includes(marker)) {
        const screen = request({ operation: 'state' }).text;
        assert.ok(screen.includes(marker), `${marker} did not complete: ${screen}`);
      }
    }
    if (text?.includes('BUFFER READY')) {
      const screen = request({ operation: 'state' }).text;
      assert.match(screen, /BUFFER READY/);
      assert.ok(screen.split('\n').some(line => line.trim() === '85'), 'cassette buffer sentinel must survive direct tokens');
    }
  }
  assert.ok(nonSilentAudio);
  const frame = request({ operation: 'state' });
  assert.match(frame.text, /DEV>/);
  const hash = bytes => createHash('sha256').update(bytes).digest('hex');
  const actual = { schemaVersion: 1, firmware: frame.firmwareId, cycles: frame.cycles, pc: frame.pc, pixelsSha256: hash(Buffer.from(frame.pixels)), textSha256: hash(frame.text), stateTraceSha256: states.digest('hex'), audioSha256: audio.digest('hex'), audioSamples, nonSilentAudio };
  const native = JSON.parse(execFileSync('cargo', ['run', '--release', '--locked', '--quiet', '-p', 'eightbit-dragon-emulator', '--example', 'basic-acceptance'], { cwd: root, encoding: 'utf8', timeout: 120000, maxBuffer: 1024 * 1024 }));
  assert.deepEqual(actual, native);
  console.log(JSON.stringify({ ...actual, nativeWasmParity: true, scope: 'development BASIC corpus; complete compatibility remains unqualified' }));
} finally { runtime.free(); }
