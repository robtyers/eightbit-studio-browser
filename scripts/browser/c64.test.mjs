import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
const source = new URL('../../apps/eightbit-studio-web/ui/public/generated/c64.js', import.meta.url);
test('actual C64 WASM adapter bounds staged input, steps generated firmware and releases memory', async () => {
  globalThis.window = {};
  globalThis.document = { currentScript: null };
  const factory = (await import(source.href)).default;
  const module = await factory({ wasmBinary: readFileSync(new URL('./c64.wasm',source)) });
  const pointer = module._malloc(20480);
  try {
    module.HEAPU8.fill(0,pointer,pointer+20480);
    assert.equal(module._studio_init(pointer,20479),0);
    assert.equal(module._studio_init(pointer,20480),1);
    assert.equal(module._studio_load(pointer,1),0);
    module.HEAPU8.set([1,8,7,8,10,0,128,0,0,0],pointer);
    assert.equal(module._studio_load(pointer,10),1);
    module._studio_key(32,1); module._studio_step(20000); module._studio_key(32,0);
    assert.ok(module._studio_width()>256); assert.ok(module._studio_height()>192);
    assert.ok(module._studio_pixels()>0); assert.ok(module._studio_audio_count()<=8192);
    module._studio_stop(); assert.equal(module._studio_audio_count(),0);
    assert.equal(module._studio_load(pointer,10),0);
  } finally { module.HEAPU8.fill(0,pointer,pointer+20480); module._free(pointer); delete globalThis.window; delete globalThis.document; }
});
