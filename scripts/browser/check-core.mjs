import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { initSync, browser_request } from '../../apps/eightbit-studio-web/ui/public/generated/dragon_core.js';
initSync({ module: readFileSync(new URL('../../apps/eightbit-studio-web/ui/public/generated/dragon_core_bg.wasm', import.meta.url)) });
const request = (metadata, bytes = new Uint8Array()) => JSON.parse(browser_request(JSON.stringify({ schemaVersion: 1, ...metadata }), bytes));
const source = '10 PRINT "BROWSER CONTRACT"\n20 END\n';
const built = request({ operation: 'test', source });
assert.equal(built.ok, true); assert.equal(built.result.roundTripVerified, true);
const cassette = Uint8Array.from(built.result.cassette);
assert.equal(request({ operation: 'decompile', input: { format: 'cassette' } }, cassette).result.source, source);
assert.equal(request({ operation: 'build', source: '10 PRINT "broken' }).ok, false);
assert.equal(request({ operation: 'decompile', input: { format: 'cassette' } }, new Uint8Array([1, 2])).ok, false);
assert.equal(request({ operation: 'firmware' }, new Uint8Array(16384)).result.matchesVerifiedDragon32, false);
assert.equal(request({ operation: 'decompile', input: { format: 'raw', loadAddress: 1024, executionAddress: 1024 } }, new Uint8Array([0x86, 0x42, 0x39])).result.kind, 'assembly');
const sample = readFileSync(new URL('../../packages/browser-workbench/src/guess-the-number.bas', import.meta.url), 'utf8');
assert.equal(request({ operation: 'test', source: sample }).result.roundTripVerified, true);
const presentation = request({ operation: 'presentation', source: sample });
assert.ok(presentation.result.highlights.length > 0);
assert.equal(presentation.result.cassette, undefined);
assert.deepEqual(request({ operation: 'presentation', source: '10 PRINT "£"' }).result.highlights, []);
console.log('Browser WASM contract: original beginner game, presentation, build, test, decompile, failure and generated-ROM rejection passed.');

const created = request({ operation: 'create-project', name: 'Portable game', kind: 'basic', source });
assert.equal(created.ok, true); assert.equal(created.result.entryPath, 'src/basic/main.bas');
assert.ok(created.result.project.supportFiles['AGENTS.md']); assert.ok(created.result.project.supportFiles['CLAUDE.md']);
assert.ok(created.result.project.supportFiles['tests/README.md']);
assert.equal(request({operation:'test-project', project:created.result.project}).result.roundTripVerified, true);
const exported = request({ operation: 'export-project', project: created.result.project });
assert.equal(exported.ok, true);
const reopened = request({ operation: 'import-project' }, Uint8Array.from(exported.result.archive));
assert.deepEqual(reopened.result.project, created.result.project);
assert.equal(request({operation:'test-project', project:{...created.result.project, source:'20 END\n'}}).ok, false);
console.log('Browser WASM projects: shared desktop scaffold, project build, ZIP round trip and entry rejection passed.');

for (const machine of request({ operation: 'machines' }).result.machines) {
  const made = request({ operation: 'create-project', name: 'Portable game', kind: 'basic', source: machine.sample, target: machine.id });
  assert.equal(made.ok, true, machine.id);
  const checked = request({ operation: 'test-project', project: made.result.project });
  assert.equal(checked.ok, true, machine.id);
  assert.equal(checked.result.roundTripVerified, true);
  const recovered = request({ operation: 'decompile', target: machine.id, input: { format: 'cassette' } }, Uint8Array.from(checked.result.cassette));
  assert.equal(recovered.ok, true, machine.id);
  assert.equal(recovered.result.kind, 'basic');
  const zipped = request({ operation: 'export-project', project: made.result.project });
  assert.deepEqual(request({ operation: 'import-project' }, Uint8Array.from(zipped.result.archive)).result.project, made.result.project);
}
console.log('All eight machine profiles: actual WASM create, build/check, BASIC recovery and ZIP round trip passed.');
