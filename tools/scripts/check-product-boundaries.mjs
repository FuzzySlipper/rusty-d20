import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';

const root = process.cwd();
const productionRoots = [
  'apps/app/src',
  'libs/domain/src',
  'libs/feature-main-menu/src',
  'libs/platform/src',
  'libs/protocol/src',
  'libs/shell/src',
  'libs/store/src',
  'libs/transport/src',
];
const forbiddenProductionReferences = [
  '@rusty-d20/testing-fixtures',
  '@rusty-d20/demo-config',
  'PlaceholderActions',
  'createFakeRustyD20Transport',
];
const failures = [];

for (const relativeRoot of productionRoots) {
  for (const file of sourceFiles(join(root, relativeRoot))) {
    const source = readFileSync(file, 'utf8');
    for (const forbidden of forbiddenProductionReferences) {
      if (source.includes(forbidden)) {
        failures.push(`${file.slice(root.length + 1)} imports or names test-only wiring: ${forbidden}`);
      }
    }
  }
}

const manifest = readFileSync(join(root, 'rust/crates/rusty-d20/Cargo.toml'), 'utf8');
const engineRevision = 'fb608e323a8b44a55195f5720101224ff37fd5db';
for (const crateName of ['core-ids', 'entity-state', 'gameplay-mechanics', 'gameplay-rules']) {
  const expected = `${crateName} = { git = "https://github.com/FuzzySlipper/rusty-engine", rev = "${engineRevision}" }`;
  if (!manifest.includes(expected)) {
    failures.push(`Rusty Engine dependency is not exactly pinned: ${crateName}`);
  }
}
if (/^[a-z][a-z0-9-]*\s*=\s*\{[^}\n]*path\s*=/mu.test(manifest)) {
  failures.push('The downstream Rust manifest must not use path dependencies.');
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
