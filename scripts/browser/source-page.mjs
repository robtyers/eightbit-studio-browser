import { sourceRelease } from './source-release.mjs';

export function renderSourcePage(config) {
  const release = sourceRelease(config);
  return `<!doctype html>
<html lang="en"><head>
<meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>Source code on GitHub · Eightbit Studio</title>
<style>
:root { color-scheme: dark light; font-family: -apple-system, BlinkMacSystemFont, "Helvetica Neue", sans-serif; background: #1e1e1e; color: #f2f2f2; }
body { max-width: 660px; margin: 0 auto; padding: clamp(24px, 6vw, 64px); line-height: 1.6; }
h1 { font-size: clamp(28px, 5vw, 38px); line-height: 1.2; }
a { color: #64d2ff; display: inline-block; min-height: 44px; align-content: center; overflow-wrap: anywhere; }
a:focus-visible { outline: 3px solid currentColor; outline-offset: 4px; }
@media (prefers-color-scheme: light) { :root { background: #f5f5f5; color: #1d1d1f; } a { color: #0066cc; } }
</style></head><body>
<main><a href="../index.html">← Back to Studio</a>
<p>Eightbit Studio · Browser edition</p>
<h1>Source code on GitHub</h1>
<p>Explore the browser edition’s code, build guide and software licences on GitHub.</p>
<p><a href="${release.url}">Open the matching source release</a></p>
<p><a href="${release.repositoryUrl}">Browse the source repository</a></p>
<p>You can use Studio without downloading the source. No system ROMs are included.</p>
</main></body></html>
`;
}
