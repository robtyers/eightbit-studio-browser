import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const generated = resolve(root, 'apps/eightbit-studio-web/ui/public/generated');
const module = await import(pathToFileURL(resolve(generated, 'dragon_core.js')).href);
await module.default({ module_or_path: readFileSync(resolve(generated, 'dragon_core_bg.wasm')) });
const runtime = new module.MonitorRuntime(1);
const request = command => JSON.parse(runtime.command(JSON.stringify({ schemaVersion: 1, sessionId: 1, command })));
const audio = createHash('sha256');
const states = createHash('sha256');
let audioSamples = 0;
let nonSilentAudio = false;
try {
  const commands = ['M 0600 41', 'F 0610 10 42', 'C 0610 10', 'L 0620 8641B7040039', 'G 0620', 'M 0500 41', 'R', 'L 0620 86FEB7FF208634B7FF21B7FF01B7FF03863CB7FF238602B7FF208E0100301F26FC88FC20F2', 'G 0620', null];
  for (const text of commands) {
    request(text === null ? { operation: 'reset' } : { operation: 'type', text });
    for (let n = 0; n < (text?.length > 60 ? 50 : 30); n++) {
      const step = request({ operation: 'advance', cycles: 32768 });
      assert.equal(step.state, 'running');
      assert.equal(step.audioSampleRate, 48000);
      assert.ok(step.audio.length <= 4096);
      states.update(JSON.stringify(step));
      const samples = Buffer.alloc(step.audio.length * 2);
      step.audio.forEach((sample, index) => samples.writeInt16LE(sample, index * 2));
      audio.update(samples); audioSamples += step.audio.length;
      nonSilentAudio ||= step.audio.some(sample => sample !== 0);
    }
  }
  assert.ok(nonSilentAudio, 'original guest DAC programme must generate audio');
  const frame = request({ operation: 'state' });
  assert.match(frame.text, /EIGHTBIT DRAGON MONITOR/);
  const hash = bytes => createHash('sha256').update(bytes).digest('hex');
  const actual = { schemaVersion: 1, firmware: frame.firmwareId, cycles: frame.cycles, pc: frame.pc, pixelsSha256: hash(Buffer.from(frame.pixels)), textSha256: hash(frame.text), stateTraceSha256: states.digest('hex'), audioSha256: audio.digest('hex'), audioSamples, nonSilentAudio };
  const native = JSON.parse(execFileSync('cargo', ['run', '--quiet', '-p', 'eightbit-dragon-emulator', '--example', 'acceptance'], { cwd: root, encoding: 'utf8', timeout: 120000, maxBuffer: 1024 * 1024 }));
  assert.deepEqual(actual, native);
  console.log(JSON.stringify({ ...actual, nativeWasmParity: true }));
} finally { runtime.free(); }
