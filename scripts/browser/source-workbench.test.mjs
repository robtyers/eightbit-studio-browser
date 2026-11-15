import { test } from 'node:test';
import assert from 'node:assert/strict';
import { cpSync, existsSync, mkdtempSync, readFileSync, readdirSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, relative, resolve } from 'node:path';
import { browserSharedFiles, browserSharedExport, excludedBrowserSource } from './source-workbench.mjs';
const root = resolve(import.meta.dirname, '../..');
const imports = source => Array.from(source.matchAll(/(?:from\s*|import\s*)["']([^"']+)["']/g), match => match[1]);
test('retained browser shared source is closed over local imports and explicit exports', () => {
  const retained = new Set(browserSharedFiles.map(path => resolve(root, path)));
  const manifest = JSON.parse(readFileSync(join(root, 'packages/workbench/package.json'), 'utf8'));
  for (const path of retained) {
    assert.ok(existsSync(path), `Missing retained source ${relative(root, path)}`);
    if (!/\.tsx?$/.test(path)) continue;
    for (const name of imports(readFileSync(path, 'utf8'))) {
      if (name.startsWith('@eightbitstudio/workbench/')) {
        const key = name.replace('@eightbitstudio/workbench/', './');
        assert.ok(browserSharedExport(key), `Missing export ${key}`);
        assert.ok(retained.has(resolve(root, 'packages/workbench', manifest.exports[key])));
      } else if (name.startsWith('.')) {
        const base = resolve(dirname(path), name);
        assert.ok(['', '.ts', '.tsx', '.json'].some(ext => retained.has(base + ext)), `Missing local input ${name} from ${relative(root, path)}`);
      }
    }
  }
  for (const directory of ['apps/eightbit-studio-web/ui/src', 'packages/browser-workbench/src']) {
    for (const filename of readdirSync(join(root, directory)).filter(name => /\.tsx?$/.test(name) && !/\.test\./.test(name))) {
      for (const name of imports(readFileSync(join(root, directory, filename), 'utf8'))) {
        if (name.startsWith('@eightbitstudio/workbench/')) assert.ok(browserSharedExport(name.replace('@eightbitstudio/workbench/', './')), `Missing app export ${name}`);
      }
    }
  }
  for (const key of ['./services/desktop', './services/ai', './components/TerminalPanel']) assert.equal(browserSharedExport(key), false);
});

test('browser source excludes overflow qualification and retains every portable crate Rust input', () => {
  const temporary = mkdtempSync(join(tmpdir(), 'eightbit-browser-retained-'));
  const crate = 'families/dragon/crates/eightbit-dragon-emulator';
  try {
    const copy = path => cpSync(join(root, path), join(temporary, path), { recursive: true, filter: from => !excludedBrowserSource(relative(root, from).replaceAll('\\', '/')) });
    const crates = ['families/vectrex/crates/eightbit-vectrex-emulator', 'crates/eightbit-targets', 'crates/eightbit-source-core', 'cpu/m6809/eightbit-m6809-core', 'cpu/mos6502/eightbit-mos6502-core', 'families/commodore/crates/eightbit-commodore-machine', 'cpu/z80/eightbit-sinclair-basic', 'cpu/z80/eightbit-z80-core', 'families/spectrum/crates/eightbit-spectrum-emulator', 'families/zx81/crates/eightbit-zx81-emulator',
      ...['targets', 'basic-core', 'media', 'mobile', 'project', 'emulator'].map(name => `families/dragon/crates/eightbit-dragon-${name}`),
      ...['commodore', 'acorn', 'spectrum', 'zx81'].map(name => `families/${name}/crates/eightbit-${name}-core`), 'apps/eightbit-studio-web/core'];
    for (const path of crates) copy(path);
    for (const family of ['vectrex', 'dragon', 'commodore', 'acorn', 'spectrum', 'zx81']) copy(`families/${family}/templates`);
    copy('families/dragon/dialects/dragon32-basic'); copy('templates/project-CLAUDE.md');
    copy('packages/browser-workbench/src/guess-the-number.bas');
    const guides = ['families/commodore/help/GUIDE.html', 'families/acorn/help/GUIDE.html', 'families/spectrum/help/GUIDE.html', 'cpu/z80/help/GUIDE.html'];
    const packaging = readFileSync(join(root, 'scripts/browser/package-source.mjs'), 'utf8');
    for (const path of guides) {
      assert.ok(packaging.includes(`'${path}'`), `Missing explicit source allow-list input ${path}`);
      copy(path);
    }
    copy('families/dragon/firmware/basic'); copy('families/dragon/firmware/monitor');
    copy('families/dragon/firmware/basic-extended'); copy('families/dragon/firmware/basic-complete');
    assert.ok(packaging.includes("'families/dragon/firmware/basic64'"));
    copy('families/dragon/firmware/basic64');
    copy('families/spectrum/firmware/original');
    for (const path of ['families/vectrex/templates', 'families/vectrex/firmware/original', 'families/vectrex/crates/eightbit-vectrex-emulator']) {
      assert.ok(packaging.includes(`'${path}'`), `Missing original Vectrex source input ${path}`);
      copy(path);
    }
    assert.ok(packaging.includes("'families/commodore/firmware/original'"));
    copy('families/commodore/firmware/original');
    const c64 = JSON.parse(readFileSync(join(temporary,'families/commodore/firmware/original/runtime-manifest.json'),'utf8'));
    assert.equal(readFileSync(join(temporary,'families/commodore/firmware/original/runtime.c64rom')).length,c64.imageBytes);
    for (const path of ['cpu/z80/firmware/original', 'families/zx81/firmware/original']) {
      assert.ok(packaging.includes(`'${path}'`), `Missing original Z80 source input ${path}`);
      copy(path);
    }
    assert.ok(packaging.includes("'families/zx81/crates/eightbit-zx81-emulator'"));
    for (const path of ['cpu/z80/eightbit-z80-core', 'families/spectrum/crates/eightbit-spectrum-emulator', 'families/spectrum/firmware/original']) assert.ok(packaging.includes(`'${path}'`), `Missing Spectrum source input ${path}`);
    assert.equal(existsSync(join(temporary, crate, 'tests/basic_complete.rs')), false);
    assert.equal(existsSync(join(temporary, crate, 'tests/fixtures/complete-basic')), false);
    assert.equal(existsSync(join(temporary, 'families/dragon/firmware/basic-complete/source/console.inc')), true);
    assert.equal(existsSync(join(temporary, 'families/dragon/firmware/basic-complete/fixed-layout.asm')), true);
    assert.equal(existsSync(join(temporary, 'families/dragon/firmware/basic-complete/layout.asm')), true);
    assert.equal(existsSync(join(temporary, 'families/dragon/firmware/basic-complete/qualification')), false);
    assert.equal(excludedBrowserSource('families/dragon/firmware/basic-complete/source/unknown.bin'), true);
    assert.equal(excludedBrowserSource('families/dragon/firmware/basic-complete/qualification/candidate.bin'), true);
    assert.equal(excludedBrowserSource(`${crate}/tests/fixtures/complete-basic/keyboard.bin`), true);
    assert.equal(excludedBrowserSource(`${crate}/tests/basic_rom.rs`), false);
    const walk = directory => readdirSync(directory, { withFileTypes: true }).flatMap(entry => entry.isDirectory() ? walk(join(directory, entry.name)) : [join(directory, entry.name)]);
    const retained = walk(temporary);
    assert.equal(retained.some(path => /\.bin$/.test(path)), false);
    for (const path of retained.filter(path => path.endsWith('.rs'))) {
      const source = readFileSync(path, 'utf8'); assert.equal(source.includes('basic-complete'), false);
      for (const match of source.matchAll(/include_(?:str|bytes)!\(\s*"([^"]+)"\s*\)/g)) assert.ok(existsSync(resolve(dirname(path), match[1])), `Missing retained Rust input ${match[1]}`);
    }
    const roms = retained.filter(path => path.endsWith('.rom'));
    assert.equal(roms.length, 5);
    const d64 = JSON.parse(readFileSync(join(temporary, 'families/dragon/firmware/basic64/manifest.json'), 'utf8'));
    for (const path of roms) {
      const bytes = path.endsWith('/basic64/native.rom') ? d64.profiles.native.bytes
        : path.endsWith('/basic64/compatibility.rom') ? d64.profiles.compatibility.bytes
          : path.endsWith('/basic-extended/basic.rom') ? 32768 : 16384;
      assert.equal(readFileSync(path).length, bytes);
    }
  } finally { rmSync(temporary, { recursive: true, force: true }); }
});
