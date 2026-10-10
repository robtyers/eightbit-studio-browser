import assert from 'node:assert/strict';
import { test } from 'node:test';
import { renderSourcePage } from './source-page.mjs';
import { sourceRelease } from './source-release.mjs';
const config = { schemaVersion: 1, repository: 'robtyers/eightbit-studio-browser', tag: 'browser-0.5.2-20261010.1' };

test('compatibility source page links to the exact GitHub source release without local archives', () => {
  const html = renderSourcePage(config);
  assert.ok(html.includes(`href="${sourceRelease(config).url}"`));
  assert.ok(html.includes('href="https://github.com/robtyers/eightbit-studio-browser"'));
  assert.ok(html.includes('href="../index.html"'));
  assert.ok(html.includes('name="viewport"'));
  assert.ok(!html.includes('.part'));
  assert.ok(!/<a\b[^>]*\sdownload[\s=>]/.test(html));
});
test('source release rejects other hosts/repositories and unsafe or unversioned tags', () => {
  for (const invalid of [null, { ...config, schemaVersion: 2 }, { ...config, repository: 'attacker/other' }, { ...config, tag: 'latest' }, { ...config, tag: '../private' }, { ...config, tag: 'browser-0.5.2-20261010.1" onclick="alert(1)' }]) {
    assert.throws(() => renderSourcePage(invalid), /Invalid browser source release/);
  }
});
