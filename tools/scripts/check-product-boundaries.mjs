import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';

import {
  ENGINE_REPOSITORY,
  loadEngineSource,
} from '../../scripts/engine-revision-lib.mjs';

const root = process.cwd();
const productionRoots = [
  'apps/app/src',
  'libs/domain/src',
  'libs/feature-main-menu/src',
  'libs/platform/src',
  'libs/protocol/src',
  'libs/renderer/src',
  'libs/shell/src',
  'libs/store/src',
  'libs/transport/src',
];
const forbiddenProductionReferences = [
  '@rusty-d20/testing-fixtures',
  '@rusty-d20/demo-config',
  'PlaceholderActions',
  'createFakeRustyD20Transport',
  '@rusty-engine/',
];
const failures = [];

for (const relativeRoot of productionRoots) {
  for (const file of sourceFiles(join(root, relativeRoot))) {
    const source = readFileSync(file, 'utf8');
    for (const forbidden of forbiddenProductionReferences) {
      if (source.includes(forbidden)) {
        failures.push(
          `${file.slice(root.length + 1)} imports or names test-only wiring: ${forbidden}`,
        );
      }
    }
  }
}

const rootManifest = readFileSync(join(root, 'Cargo.toml'), 'utf8');
const manifest = readFileSync(join(root, 'rust/crates/rusty-d20/Cargo.toml'), 'utf8');
const applicationWorkspace = readFileSync(join(root, 'pnpm-workspace.yaml'), 'utf8');
const engineSource = loadEngineSource(root);
const expected = `rusty-engine = { git = "${ENGINE_REPOSITORY}", branch = "${engineSource.branch}" }`;
if (!rootManifest.split('\n').some((line) => line.trim() === expected)) {
  failures.push('Rusty Engine complete facade does not track canonical main');
}
if (!/^rusty-engine\.workspace = true$/mu.test(manifest)) {
  failures.push('Rusty D20 must import the complete Engine facade');
}
if (/^[a-z][a-z0-9-]*\s*=\s*\{[^}\n]*path\s*=/mu.test(manifest)) {
  failures.push('The downstream Rust manifest must not use path dependencies.');
}
if (applicationWorkspace.includes('@rusty-engine/')) {
  failures.push('The downstream application workspace must not carry Engine renderer packages.');
}

if (failures.length > 0) {
  console.error(failures.join('\n'));
  process.exit(1);
}
console.log('Product boundary audit passed.');

function sourceFiles(directory) {
  const files = [];
  for (const entry of readdirSync(directory)) {
    const path = join(directory, entry);
    if (statSync(path).isDirectory()) {
      files.push(...sourceFiles(path));
    } else if (path.endsWith('.ts')) {
      files.push(path);
    }
  }
  return files;
}
