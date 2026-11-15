// Opt-in, read-only acceptance of the user's authored game projects. No game
// source or reference executable is copied into the distributed test suite.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, statSync } from 'node:fs';
import { resolve } from 'node:path';

const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const definitions = [
  { name: 'DragonDoom', sourcePaths: ['src/asm/game.asm', 'src/asm/data.asm'], basic: 'src/basic/main.bas', loadAddress: 0x2000, referenceSha256: '03a7e6f6bbf9124f27dddb8f29e1436321ad67c0ce84346f21a923bc785f7267', title: 'TITLE_READY' },
  { name: 'battlezone', sourcePaths: ['asm/MAIN.asm'], basic: 'src/MAIN.BAS', loadAddress: 0x3800, referenceSha256: '6de323c475cdb26a1b23cc2e3895043c304b1e1dbe35faf36ba27d0af0e86681', title: 'menu_wait' },
];

export function readGameProjects(directories) {
  assert.equal(directories.length, 2, 'Provide DragonDoom and Battlezone project folders');
  return definitions.map((definition, index) => {
    const inputs = {};
    const contents = {};
    let total = 0;
    for (const path of ['dragon.toml', ...definition.sourcePaths, definition.basic]) {
      const fullPath = resolve(directories[index], path);
      const size = statSync(fullPath).size;
      total += size;
      assert.ok(size > 0 && total <= 1024 * 1024, 'Game acceptance input exceeds 1 MiB');
      const bytes = readFileSync(fullPath);
      assert.equal(bytes.length, size, 'Game input changed during inspection');
      inputs[path] = hash(bytes);
      contents[path] = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
    }
    const project = {
      schemaVersion: 2, target: 'dragon32', name: definition.name, kind: 'assembly',
      source: contents[definition.sourcePaths[0]], manifest: contents['dragon.toml'],
      supportFiles: Object.fromEntries([...definition.sourcePaths.slice(1), definition.basic].map(path => [path, contents[path]])),
    };
    return { ...definition, project, inputs };
  });
}

/** The caller supplies the real native/WASM parity owner, never a fake CPU or
 * memory writer. All game interaction goes through physical keyboard commands;
 * debugger memory is read-only and cassette bytes are loaded by guest firmware. */
export async function acceptGames(fixtures, host) {
  const { action, peer, browserRequest, createAssembly, monitor, debugState, machineCommand, getPixels, getAudio } = host;
  const results = [];
  for (const fixture of fixtures) {
    process.stderr.write(`Checking ${fixture.name}: unchanged source, build and guest execution\n`);
    let audioSamples = 0;
    let variedAudio = false;
    let audioTrace = createHash('sha256');
    const linkedReply = await peer({ operation: 'link-inspection', sources: fixture.sourcePaths.map((path, index) => ({ source_uri: path, source: index === 0 ? fixture.project.source : fixture.project.supportFiles[path] })), load_address: fixture.loadAddress, entry_symbol: 'START' });
    assert.equal(linkedReply.ok, true, linkedReply.error);
    const linked = linkedReply.result;
    assert.equal(linked.sha256, fixture.referenceSha256, `${fixture.name}: unchanged source must match its independently assembled LWTOOLS 4.25 reference`);
    const address = name => {
      const symbols = linked.symbols.filter(item => item.sourceUri === fixture.sourcePaths[0] && item.name.toUpperCase() === name.toUpperCase());
      assert.equal(symbols.length, 1, `${fixture.name}: unique symbol ${name}`);
      return symbols[0].address;
    };
    const lineFor = name => {
      const lines = fixture.project.source.split('\n');
      let index = lines.findIndex(line => new RegExp(`^${name}(?:\\s|$)`, 'i').test(line));
      assert.ok(index >= 0, `Missing source label ${name}`);
      if (lines[index].trim().toUpperCase() === name.toUpperCase()) {
        do { index++; } while (index < lines.length && (!lines[index].trim() || lines[index].trim().startsWith(';')));
      }
      return index + 1;
    };
    const imported = await browserRequest({ operation: 'import-project' }, [...Buffer.from(JSON.stringify(fixture.project))]);
    assert.equal(imported.ok, true, imported.message);
    assert.deepEqual(imported.result.sourcePaths, fixture.sourcePaths);
    assert.deepEqual(imported.result.project, fixture.project);
    const exported = await browserRequest({ operation: 'export-project', project: fixture.project });
    assert.equal(exported.ok, true, exported.message);
    const roundTrip = await browserRequest({ operation: 'import-project' }, exported.result.archive);
    assert.equal(roundTrip.ok, true, roundTrip.message);
    assert.deepEqual(roundTrip.result.project, fixture.project, 'ZIP must retain every declared source and original manifest');
    const built = await browserRequest({ operation: 'test-project', project: fixture.project });
    assert.equal(built.ok, true, built.message ?? JSON.stringify(built.diagnostics));
    assert.equal(built.result.summary.tokenBytes, linked.byteCount);
    assert.equal(built.result.assembly.loadAddress, fixture.loadAddress);
    assert.equal(built.result.assembly.entryAddress, fixture.loadAddress);
    const request = {
      schemaVersion: 1, kind: 'assembly-source', project: fixture.project,
      buildId: built.result.assembly.buildId, cassetteSha256: built.result.summary.cassetteSha256,
      debug: { kind: 'source', sourceUri: fixture.sourcePaths[0], sourceLine: lineFor(fixture.title) },
    };
    await createAssembly(request);
    async function advance() {
      const frame = await monitor({ operation: 'advance', cycles: 32768 });
      assert.notEqual(frame.state, 'failed', frame.message);
      const samples = getAudio();
      audioSamples += samples.length;
      variedAudio ||= samples.some(sample => sample !== samples[0]);
      audioTrace.update(frame.audioSha256);
      return frame;
    }
    async function stopped(limit = 8192) {
      for (let n = 0; n < limit; n++) {
        const frame = await advance();
        const state = await debugState();
        assert.notEqual(state.status.state, 'failed', state.status.message);
        if (state.status.state === 'stopped') return frame;
      }
      throw new Error(`${fixture.name}: game did not reach its breakpoint within the cycle budget`);
    }
    const memory = async (name, length = 1) => (await machineCommand({ operation: 'memory', address: address(name), length })).status.memory.bytes;
    const value = async name => (await memory(name))[0];
    const word = async name => { const bytes = await memory(name, 2); return bytes[0] * 256 + bytes[1]; };
    const key = (code, pressed) => monitor({ operation: 'key', code, pressed });
    async function resumeAt(name) {
      await machineCommand({ operation: 'breakpoint', address: address(name) });
      await machineCommand({ operation: 'continue' });
      const frame = await stopped(512);
      const state = await debugState();
      assert.equal(state.status.registers.pc, address(name));
      return frame;
    }
    function colours() {
      const pixels = getPixels();
      const colours = new Set();
      for (let i = 0; i < pixels.length; i += 4) colours.add(`${pixels[i]},${pixels[i + 1]},${pixels[i + 2]}`);
      return colours.size;
    }
    await stopped();
    process.stderr.write(`${fixture.name}: guest cassette load and title breakpoint reached\n`);
    audioSamples = 0;
    variedAudio = false;
    audioTrace = createHash('sha256');
    assert.equal((await debugState()).status.location.sourceUri, fixture.sourcePaths[0]);
    assert.equal((await debugState()).status.location.sourceLine, lineFor(fixture.title));
    // Both games switch display bases during vertical blanking. The first
    // breakpoint precedes scanout of the newly selected screen; let the real
    // VDG complete several fields before judging its visible image.
    await machineCommand({ operation: 'breakpoint', address: address('START') });
    await machineCommand({ operation: 'continue' });
    await advance(); await advance();
    await machineCommand({ operation: 'pause' });
    const title = await resumeAt(fixture.title);
    assert.ok(colours() >= (fixture.name === 'DragonDoom' ? 4 : 2), `${fixture.name}: title must render visible graphics (observed ${colours()} colours)`);
    let play;
    let interaction;
    if (fixture.name === 'DragonDoom') {
      assert.equal(await value('TITLE_ACTIVE'), 1);
      assert.equal(await word('FRAME_COUNT'), 0);
      assert.equal(await value('HEALTH'), 100);
      assert.equal(await value('AMMO'), 40);
      const writes = await word('DAC_WRITES');
      await resumeAt('TITLE_READY');
      assert.equal(await value('TITLE_GATE'), 1, 'Title must first observe released keys');
      assert.notEqual(await word('DAC_WRITES'), writes, 'Title must service the physical DAC');
      await key('Space', true);
      await resumeAt('TITLE_READY');
      assert.equal(await value('TITLE_GATE'), 2, 'Fresh title key must be observed');
      await key('Space', false);
      play = await resumeAt('FRAME_READY');
      assert.equal(await value('TITLE_ACTIVE'), 0);
      assert.equal(await word('FRAME_COUNT'), 1);
      const beforeMove = await word('PLAYER_X');
      await key('KeyW', true);
      await resumeAt('FRAME_READY');
      await resumeAt('FRAME_READY');
      await key('KeyW', false);
      const afterMove = await word('PLAYER_X');
      assert.notEqual(afterMove, beforeMove, 'Physical W must move the player');
      const beforeAmmo = await value('AMMO');
      await key('Space', true);
      await resumeAt('FRAME_READY');
      await key('Space', false);
      assert.ok(await value('AMMO') < beforeAmmo, 'Physical Space must fire');
      await key('KeyM', true);
      play = await resumeAt('FRAME_READY');
      await key('KeyM', false);
      assert.equal(await value('MUTED'), 1, 'Physical M must toggle sound');
      interaction = { freshTitlePressRelease: true, movement: true, firing: true, mute: true, frameCount: await word('FRAME_COUNT') };
    } else {
      assert.equal(await value('game_active'), 0);
      const titleClock = await value('clock');
      await key('Space', true);
      await resumeAt('main_loop');
      await key('Space', false);
      assert.equal(await value('game_active'), 1);
      assert.equal(await value('lives'), 3);
      await resumeAt('flip_screens');
      play = await resumeAt('flip_screens');
      assert.notEqual(await value('clock'), titleClock, 'PAL interrupts must advance the game clock');
      const beforeMove = await memory('player', 4);
      await key('KeyW', true);
      await resumeAt('flip_screens');
      await key('KeyW', false);
      assert.notDeepEqual(await memory('player', 4), beforeMove, 'Physical W must move the tank');
      await key('Space', true);
      await resumeAt('step_done');
      await key('Space', false);
      assert.ok((await memory('player_shell', 6))[5] > 0, 'Fresh physical fire must create a live shell');
      play = await resumeAt('flip_screens');
      interaction = { titleStart: true, fieldInterrupts: true, movement: true, firing: true, lives: await value('lives') };
    }
    assert.notEqual(play.pixelsSha256, title.pixelsSha256, 'Play must replace the title graphics');
    assert.equal(variedAudio, true, 'Real DAC output must produce changing audio samples');
    // Inspect/step the actual game, then confirm that stopped execution is inert.
    await machineCommand({ operation: 'step' });
    const state = await debugState();
    assert.equal(state.status.valuesValid, true);
    assert.equal(state.status.location.sourceUri, fixture.sourcePaths[0]);
    const paused = await monitor({ operation: 'state' });
    assert.equal((await advance()).cycles, paused.cycles);
    await monitor({ operation: 'stop' });
    await action({ operation: 'close' });
    results.push({ name: fixture.name, inputs: fixture.inputs, unchangedSources: true, bytes: linked.byteCount,
      rawSha256: linked.sha256, freshLwtools425ByteParity: true, buildId: request.buildId,
      cassetteSha256: request.cassetteSha256, zipRoundTrip: true, sourceBreakpointAndStep: true,
      titlePixelsSha256: title.pixelsSha256, playPixelsSha256: play.pixelsSha256,
      audioSamples, variedAudio, audioTraceSha256: audioTrace.digest('hex'), ...interaction });
    process.stderr.write(`${fixture.name}: graphics, input, audio and source debugging passed\n`);
  }
  return results;
}
