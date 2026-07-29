import { mkdir, readFile, writeFile } from 'node:fs/promises';

import { authorStarterArtifacts } from '../packages/starter-ruleset/dist/index.js';

const mode = process.argv[2];
if (mode !== '--write' && mode !== '--check') {
  throw new Error('usage: generate-artifacts.mjs --write|--check');
}

const root = new URL('../artifacts/starter/', import.meta.url);
const artifacts = authorStarterArtifacts();
const entries = [
  ['starter-core.json', artifacts.core],
  ['steel-guard.json', artifacts.steelGuard],
  ['ember-ward.json', artifacts.emberWard],
  ['wardens-gate.json', artifacts.wardensGate],
  ['catalog-probe.json', artifacts.catalogProbe],
  ['invalid-semantics.json', artifacts.invalidSemantics],
];
const runtimeArtifacts = [
  artifacts.core,
  artifacts.steelGuard,
  artifacts.emberWard,
  artifacts.wardensGate,
  artifacts.catalogProbe,
];
const manifest = `${JSON.stringify(
  {
    schemaVersion: 1,
    artifacts: entries.map(([path, artifact]) => ({
      path,
      domain: artifact.package.domain,
      package: artifact.package.package,
      version: artifact.package.version,
      fingerprint: artifact.fingerprint,
    })),
  },
  null,
  2,
)}\n`;

await mkdir(root, { recursive: true });
for (const [path, artifact] of entries) {
  await update(new URL(path, root), artifact.canonicalJson);
}
await update(new URL('manifest.json', root), manifest);
await update(
  new URL('catalog.json', root),
  `${JSON.stringify(
    {
      schemaVersion: 1,
      packages: runtimeArtifacts.map((artifact) => artifact.canonicalJson),
    },
    null,
    2,
  )}\n`,
);

async function update(url, expected) {
  if (mode === '--write') {
    await writeFile(url, expected, 'utf8');
    return;
  }
  let actual;
  try {
    actual = await readFile(url, 'utf8');
  } catch {
    throw new Error(`generated artifact is missing: ${url.pathname}`);
  }
  if (actual !== expected) {
    throw new Error(
      `generated artifact is stale: ${url.pathname}; run pnpm --dir rules run generate`,
    );
  }
}
