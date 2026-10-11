import { createHash } from 'node:crypto';
import { cpSync, existsSync, lstatSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, realpathSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { execFileSync } from 'node:child_process';
import { renderSourcePage } from './source-page.mjs';
import { configuredSourceRelease } from './source-release.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const output = join(root, 'apps/eightbit-studio-web/ui/public/generated');
const cache = join(root, '.cache/xroar-web');
const release = configuredSourceRelease();
const releaseDirectory = join(cache, 'github-release', release.tag);
const hash = (bytes) => createHash('sha256').update(bytes).digest('hex');
const archive = join(cache, 'xroar-1.12.1.tar.gz');
if (hash(readFileSync(archive)) !== 'ba525225fbd732c4dbf2cbb571dc9d2810fd9dcedf0482133cf637e2eef61f88') throw new Error('XRoar source digest mismatch');
const temporary = mkdtempSync(join(cache, 'source-'));
const source = join(temporary, 'browser-source'); mkdirSync(source);
const excluded = new Set(['node_modules', 'dist', 'target', 'generated', '.DS_Store', '.wrangler', '.dev.vars']);
function copyTree(from, to) {
  const stat = lstatSync(from);
  if (stat.isSymbolicLink()) throw new Error('Source bundle refuses symlinks');
  if (stat.isDirectory()) {
    mkdirSync(to, { recursive: true });
    for (const name of readdirSync(from).sort()) if (!excluded.has(name) && !name.endsWith('.tsbuildinfo')) {
      if ((name.startsWith('.env') || name.startsWith('.dev.vars')) || /\.(rom|cas|wav|bin|key|p12|pfx)$/i.test(name)) throw new Error('Unexpected private/binary input in first-party source');
      copyTree(join(from, name), join(to, name));
    }
  } else if (stat.isFile()) { mkdirSync(dirname(to), { recursive: true }); cpSync(from, to); }
}
const rustMembers = ['crates/eightbit-targets', 'crates/eightbit-source-core', 'cpu/z80/eightbit-sinclair-basic', ...['targets', 'basic-core', 'media', 'mobile', 'project'].map((name) => `families/dragon/crates/eightbit-dragon-${name}`), ...['commodore', 'acorn', 'spectrum', 'zx81'].map((name) => `families/${name}/crates/eightbit-${name}-core`), 'apps/eightbit-studio-web/core'];
const components = ['WorkbenchShell', 'WorkbenchResizeHandle', 'WorkbenchPanelTabs', 'EditorToolbar', 'EditorTabs', 'Icons', 'WelcomeView', 'ProcessorLogo'];
const sharedFiles = [
  ...components.map((name) => `packages/workbench/src/components/${name}.tsx`),
  'packages/workbench/src/services/recentProjects.ts', 'packages/workbench/src/services/editorCommands.ts', 'packages/workbench/src/services/commands.json',
  'packages/workbench/src/services/workbenchLayout.ts', 'packages/workbench/src/styles.css', 'packages/workbench/src/macos.css',
  'packages/workbench/branding/fonts/Silkscreen-Regular.ttf', 'packages/workbench/branding/fonts/OFL.txt',
  'packages/workbench/branding/cpu.svg', 'packages/workbench/src/assets.d.ts',
];
// This allow-list is the public source boundary; no checkout-wide copy or Git archive.
const paths = [...rustMembers, 'families/dragon/dialects/dragon32-basic', 'families/dragon/templates', ...['commodore', 'acorn', 'spectrum', 'zx81'].map((name) => `families/${name}/templates`), 'templates/project-CLAUDE.md', 'packages/browser-workbench', 'apps/eightbit-studio-web/ui', 'apps/eightbit-studio-web/github-worker/index.ts', 'apps/eightbit-studio-web/github-worker/wrangler.jsonc', 'scripts/browser', 'docs/browser-edition.md', 'docs/adr/0052-static-browser-edition.md', 'docs/adr/0053-browser-6502-z80-basic.md', 'docs/adr/0054-bbc-model-b-source-support.md', 'docs/adr/0055-bbc-browser-platform-completion.md', 'LICENSE', 'Cargo.lock', 'pnpm-lock.yaml', 'rust-toolchain.toml', ...sharedFiles];
function inventory(directory, base = directory, files = {}) {
  for (const name of readdirSync(directory).sort()) {
    const path = join(directory, name); const stat = lstatSync(path);
    if (stat.isDirectory()) inventory(path, base, files);
    else if (stat.isFile()) files[relative(base, path).replaceAll('\\', '/')] = hash(readFileSync(path));
    else throw new Error('Unexpected source bundle entry');
  }
  return files;
}
try {
  for (const path of paths) copyTree(join(root, path), join(source, path));
  const inputs = inventory(source);
  // The published workspace contains the portable graph. Native adapters and their
  // optional dependencies are excluded, while the original manifest hashes remain
  // in the receipt so this explicit reduction cannot hide later source changes.
  for (const member of rustMembers) {
    const path = join(source, member, 'Cargo.toml');
    let manifest = readFileSync(path, 'utf8');
    manifest = manifest.replace(/^.*optional = true.*\n/gm, '').replace(/^eightbit-test-support = .*\n/gm, '').replace(/^default = \["native"\]$/m, 'default = []').replace(/^native(?:-acceptance)? = .*$/gm, (line) => `${line.split('=')[0]}= []`);
    writeFileSync(path, manifest);
  }
  rmSync(join(source, 'families/commodore/crates/eightbit-commodore-core/src/vice.rs'));
  rmSync(join(source, 'families/acorn/crates/eightbit-acorn-core/src/runtime.rs'));

  inputs['Cargo.toml'] = hash(readFileSync(join(root, 'Cargo.toml')));
  inputs['packages/workbench/package.json'] = hash(readFileSync(join(root, 'packages/workbench/package.json')));
  let cargo = readFileSync(join(root, 'Cargo.toml'), 'utf8');
  cargo = cargo.replace(/members = \[[\s\S]*?\]/, `members = [\n${rustMembers.map((path) => `    "${path}",`).join('\n')}\n]`);
  cargo = cargo.split('[patch.crates-io]')[0];
  writeFileSync(join(source, 'Cargo.toml'), cargo);
  const shared = JSON.parse(readFileSync(join(root, 'packages/workbench/package.json'), 'utf8'));
  shared.dependencies = { react: shared.dependencies.react, 'react-dom': shared.dependencies['react-dom'] };
  shared.devDependencies = Object.fromEntries(Object.entries(shared.devDependencies).filter(([name]) => ['@types/react', '@types/react-dom', 'typescript'].includes(name)));
  shared.scripts = {};
  shared.exports = Object.fromEntries(Object.entries(shared.exports).filter(([name]) => name === './styles.css' || name === './macos.css' || name === './services/editorCommands' || components.some((component) => name === `./components/${component}`)));
  writeFileSync(join(source, 'packages/workbench/package.json'), `${JSON.stringify(shared, null, 2)}\n`);
  writeFileSync(join(source, 'pnpm-workspace.yaml'), 'packages:\n  - apps/eightbit-studio-web/ui\n  - packages/browser-workbench\n  - packages/workbench\n');
  writeFileSync(join(source, 'package.json'), `${JSON.stringify({ name: 'eightbit-studio-browser-source', private: true, license: 'GPL-3.0-or-later', packageManager: 'pnpm@11.5.0' }, null, 2)}\n`);
  // Regenerate only the reduced importer graph, preserving exact checked-in pins.
  execFileSync('pnpm', ['install', '--lockfile-only', '--offline', '--ignore-scripts', '--no-frozen-lockfile'], { cwd: source, stdio: 'pipe', env: { ...process.env, CI: 'true' } });
  // Archive all Cargo dependency sources, licences and integrity metadata for this workspace.
  const vendorConfig = execFileSync('cargo', ['vendor', '--offline', '--manifest-path', join(source, 'Cargo.toml'), join(source, 'vendor')], { cwd: source, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  mkdirSync(join(source, '.cargo')); writeFileSync(join(source, '.cargo/config.toml'), vendorConfig.replaceAll(join(source, 'vendor'), 'vendor'));
  mkdirSync(join(source, '.cache/xroar-web'), { recursive: true }); cpSync(archive, join(source, '.cache/xroar-web/xroar-1.12.1.tar.gz'));
  // SDL is linked into XRoar; retain the exact SDK port source, not an unrelated latest release.
  const sdk = process.env.EMSDK || join(root, '.cache/browser-tools/emsdk-4.0.10');
  const sdl = join(sdk, 'upstream/emscripten/cache/ports/sdl2.32.0.zip');
  mkdirSync(join(source, 'third-party'), { recursive: true }); cpSync(sdl, join(source, 'third-party/sdl2.32.0.zip'));
  const mameArchive = join(root, '.cache/mame-web/mame0289.tar.gz');
  const mameHash = '0929cc749afabcef892900e10dd90bd8b05f94a7dde8f367ac6a5d2082589f84';
  if (hash(readFileSync(mameArchive)) !== mameHash) throw new Error('MAME source digest mismatch');
  const chips = join(root, '.cache/mame-web/chips-9e88298.tar.gz');
  if (hash(readFileSync(chips)) !== 'fcae39995f61bfe461175047b11f16d147be0e7353c8a247ac503b5fd3a09b8d') throw new Error('C64 source digest mismatch');
  mkdirSync(join(source, '.cache/mame-web'), { recursive: true });
  cpSync(chips, join(source, '.cache/mame-web/chips-9e88298.tar.gz'));
  const mameSdk = process.env.MAME_EMSDK || join(root, '.cache/browser-tools/emsdk-6.0.2');
  for (const name of ['sdl2.32.10.zip', 'sdl2_ttf.20.2.zip', 'freetype.zip', 'harfbuzz.2.0.tar.xz', 'zlib.3.2.tar.gz']) cpSync(join(mameSdk, 'upstream/emscripten/cache/ports', name), join(source, 'third-party', name));
  for (const path of ['LICENSE', 'AUTHORS', 'system/lib/libc/musl/COPYRIGHT', 'system/lib/compiler-rt/LICENSE.TXT']) if (existsSync(join(mameSdk, 'upstream/emscripten', path))) copyTree(join(mameSdk, 'upstream/emscripten', path), join(source, 'third-party/emscripten-6.0.2', path));
  let notice = 'Eightbit Studio browser edition, Copyright 2026 Rob Tyers. GPL-3.0-or-later.\nReused Eightbit Studio libraries and workbench components retain Apache-2.0 (LICENSE).\nXRoar 1.12.1, Copyright 2003-2026 Ciaran Anscomb and contributors, GPL-3.0-or-later.\nXRoar includes separately noticed components; all upstream notices are in its source archive.\nLocal modification: missing ROM/media files fail without network downloads.\nSDL 2.32.0, Copyright Sam Lantinga and contributors, zlib licence; source archive included.\nEmscripten 4.0.10, MIT and University of Illinois/NCSA; SDK runtime licence texts included.\nRust 1.92.0 standard library, MIT OR Apache-2.0; toolchain source is available through rust-src.\n\n';
  for (const path of ['LICENSE', 'AUTHORS', 'system/lib/libc/musl/COPYRIGHT', 'system/lib/compiler-rt/LICENSE.TXT']) {
    const full = join(sdk, 'upstream/emscripten', path);
    if (existsSync(full)) { copyTree(full, join(source, 'third-party/emscripten', path)); notice += `\nEmscripten ${path}\n${readFileSync(full, 'utf8')}\n`; }
  }
  notice += '\nMAME 0.289, GPL-2.0-or-later with BSD-licensed components. All original notices and source are supplied in mame0289.tar.gz on the matching GitHub source release. Local changes are scripts/browser/mame-browser.patch.\nC64 chips core, commit 9e88298ce56319953ac7a43213a1120359f7a3a6, zlib/libpng licence. Original source and notices are included in .cache/mame-web/chips-9e88298.tar.gz.\nMAME toolchain: Emscripten 6.0.2, MIT/NCSA, licence texts included. Linked SDL2, SDL2_ttf, FreeType, HarfBuzz and zlib port source archives with their original notices are in third-party/.\n';
  notice += `\nSDL 2.32.0 licence\n${readFileSync(join(sdk, 'upstream/emscripten/cache/ports/sdl2/SDL-release-2.32.0/LICENSE.txt'), 'utf8')}\n`;
  notice += `\nSilkscreen font (SIL Open Font Licence 1.1)\n${readFileSync(join(root, 'packages/workbench/branding/fonts/OFL.txt'), 'utf8')}\n`;
  // Retain published JS package sources/notices for every library in the browser bundle.
  const modules = join(root, 'apps/eightbit-studio-web/ui/node_modules');
  const reactDom = realpathSync(join(modules, 'react-dom'));
  for (const [name, from] of [['react', join(modules, 'react')], ['react-dom', reactDom], ['scheduler', join(reactDom, '../scheduler')]]) {
    const real = realpathSync(from);
    copyTree(real, join(source, 'third-party/javascript', name));
    const manifest = JSON.parse(readFileSync(join(real, 'package.json'), 'utf8'));
    notice += `\n${name} ${manifest.version} (${manifest.license})\n${readFileSync(join(real, 'LICENSE'), 'utf8')}\n`;
  }
  const gpl = execFileSync('tar', ['-xOf', archive, 'xroar-1.12.1/COPYING.GPL']);
  writeFileSync(join(source, 'COPYING'), gpl);
  writeFileSync(join(source, 'NOTICE.txt'), notice);
  writeFileSync(join(source, 'README.md'), `# Eightbit Studio · Browser edition\n\nBuild, edit and play BASIC programmes for Dragon 32, Commodore 64 PAL, BBC Micro Model B, ZX Spectrum and ZX81 in your browser.\n\n[Open Studio](https://eightbitstudio.com/studio/) · [Matching source release](${release.url}) · [Build guide](docs/browser-edition.md)\n\n## Source and downloads\n\nThis repository contains the browser edition and its required shared source. The matching release supplies complete browser-source.tar.gz and mame0289.tar.gz archives, build-receipt.json and SHA256SUMS. Download the MAME archive into .cache/mame-web/ before building Sinclair or BBC emulation. The browser archive also contains pinned XRoar/chips/SDL sources, JavaScript package sources and vendored Rust dependencies. No system ROMs are supplied; add your own supported firmware to play.\n\nSee docs/browser-edition.md for exact tool versions, build commands, verification and local preview instructions. SOURCE-MANIFEST.json identifies this source snapshot. Each deployed version links to its corresponding tagged release.\n\n## Licences\n\nThe browser application is **GPL-3.0-or-later**; see [COPYING](COPYING). Reused Eightbit Studio libraries/components retain [Apache-2.0](LICENSE). Third-party attribution and licence texts remain in [NOTICE.txt](NOTICE.txt), vendor/ and third-party/. MAME retains its original GPL/BSD notices in its separate archive.\n\nThis is a curated browser workspace. Its Git history starts with the public source snapshot. Desktop hosts, firmware, credentials and user projects are excluded.\n`);
  writeFileSync(join(source, 'AGENTS.md'), '# Eightbit Studio browser source\n\nFollow docs/browser-edition.md and ADRs 0052/0053/0055. Preserve the reduced browser workspace and existing shared workbench. Retain GPL, Apache and third-party notices. Never add firmware, private user projects, credentials or desktop hosts. Keep dependencies pinned and test changes at their owning layer. Publish a new tagged source release before deploying matching runtime changes; publishing requires explicit authorisation.\n');
  writeFileSync(join(source, '.gitignore'), 'node_modules/\ntarget/\ndist/\n*.tsbuildinfo\n.DS_Store\n/apps/eightbit-studio-web/ui/public/generated/\n/.cache/browser-tools/\n/.cache/xroar-web/github-release/\n/.cache/xroar-web/source-*/\n/.cache/xroar-web/browser-source.tar.gz\n/apps/eightbit-studio-web/github-worker/.dev.vars*\n/apps/eightbit-studio-web/github-worker/.wrangler/\n');
  const manifest = inventory(source);
  const sourceManifest = `${JSON.stringify({ schemaVersion: 1, files: manifest }, null, 2)}\n`;
  writeFileSync(join(source, 'SOURCE-MANIFEST.json'), sourceManifest);
  mkdirSync(output, { recursive: true });
  const archiveTemporary = join(temporary, 'browser-source.tar.gz');
  execFileSync('tar', ['-czf', archiveTemporary, '-C', temporary, 'browser-source'], { stdio: 'pipe', env: { ...process.env, COPYFILE_DISABLE: '1' } });
  mkdirSync(releaseDirectory, { recursive: true });
  const sourceArchive = join(releaseDirectory, 'browser-source.tar.gz');
  renameSync(archiveTemporary, sourceArchive);
  cpSync(mameArchive, join(releaseDirectory, 'mame0289.tar.gz'));
  rmSync(join(output, 'browser-source.tar.gz'), { force: true });
  for (const name of readdirSync(output)) if (/^(?:browser|mame)-source\.part[0-9]{2}$/.test(name)) rmSync(join(output, name));
  writeFileSync(join(output, 'sources.html'), renderSourcePage(release));
  writeFileSync(join(output, 'COPYING.txt'), gpl); writeFileSync(join(output, 'NOTICE.txt'), notice);
  const binaries = Object.fromEntries(['dragon_core.js', 'dragon_core_bg.wasm', 'xroar.js', 'xroar.wasm', 'c64.js', 'c64.wasm', 'mame-z80.js', 'mame-z80.wasm.gz', 'mame-bbc.js', 'mame-bbc.wasm.gz'].map((name) => [name, hash(readFileSync(join(output, name)))]));
  const receipt = { schemaVersion: 2, sourceRelease: { repository: release.repository, tag: release.tag, url: release.url }, sourceManifestSha256: hash(sourceManifest), sourceArchiveSha256: hash(readFileSync(sourceArchive)), toolchain: { rust: '1.92.0', wasmBindgen: '0.2.127', emscripten: '4.0.10', xroar: '1.12.1', mame: '0.289', mameEmscripten: '6.0.2', chips: '9e88298ce56319953ac7a43213a1120359f7a3a6', pnpm: '11.5.0' }, binaries, externalSources: { mameArchiveSha256: mameHash }, inputs };
  writeFileSync(join(output, 'build-receipt.json'), `${JSON.stringify(receipt, null, 2)}\n`);
  for (const name of ['COPYING.txt', 'NOTICE.txt', 'build-receipt.json']) cpSync(join(output, name), join(releaseDirectory, name));
  const downloads = ['browser-source.tar.gz', 'mame0289.tar.gz', 'COPYING.txt', 'NOTICE.txt', 'build-receipt.json'];
  writeFileSync(join(releaseDirectory, 'SHA256SUMS'), downloads.map((name) => `${hash(readFileSync(join(releaseDirectory, name)))}  ${name}\n`).join(''));
  console.log(`Browser source archive prepared; manifest SHA-256 ${receipt.sourceManifestSha256}`);
} finally { rmSync(temporary, { recursive: true, force: true }); }
