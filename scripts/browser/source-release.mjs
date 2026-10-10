import { readFileSync } from 'node:fs';

export function sourceRelease(config) {
  if (config?.schemaVersion !== 1 || config.repository !== 'robtyers/eightbit-studio-browser'
    || typeof config.tag !== 'string' || !/^browser-\d+\.\d+\.\d+-\d{8}\.\d+$/.test(config.tag)) {
    throw new Error('Invalid browser source release');
  }
  const repositoryUrl = `https://github.com/${config.repository}`;
  return { schemaVersion: 1, repository: config.repository, tag: config.tag, repositoryUrl, url: `${repositoryUrl}/releases/tag/${config.tag}`, downloadBase: `${repositoryUrl}/releases/download/${config.tag}/` };
}

export function configuredSourceRelease() {
  return sourceRelease(JSON.parse(readFileSync(new URL('../../packages/browser-workbench/src/source-release.json', import.meta.url), 'utf8')));
}
