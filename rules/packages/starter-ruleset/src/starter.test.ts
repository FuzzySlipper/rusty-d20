import assert from 'node:assert/strict';
import test from 'node:test';

import {
  authorContentOnlyExtension,
  authorReorganizedCore,
  authorStarterArtifacts,
} from './index.js';

test('starter artifacts are deterministic and module reorganization is neutral', () => {
  const first = authorStarterArtifacts();
  const second = authorStarterArtifacts();
  assert.equal(first.core.canonicalJson, second.core.canonicalJson);
  assert.equal(first.steelGuard.fingerprint, second.steelGuard.fingerprint);
  assert.equal(first.emberWard.fingerprint, second.emberWard.fingerprint);
  assert.equal(first.core.canonicalJson, authorReorganizedCore().canonicalJson);
});

test('content-only addition uses the published authoring surface', () => {
  const starter = authorStarterArtifacts();
  const before = starter.core.canonicalJson;
  const extension = authorContentOnlyExtension(starter.core);
  assert.equal(starter.core.canonicalJson, before);
  assert.equal(extension.package.payload.actions?.[0]?.id, 'shield-bash');
  assert.equal(extension.package.dependencies[0]?.fingerprint, starter.core.fingerprint);
});

test('starter catalog contains two different complete compositions', () => {
  const starter = authorStarterArtifacts();
  assert.deepEqual(
    starter.steelGuard.package.payload.actions?.map((action) => action.id),
    ['longsword-strike', 'precise-shot'],
  );
  assert.deepEqual(
    starter.emberWard.package.payload.actions?.map((action) => action.id),
    ['fire-bolt', 'mind-spike'],
  );
  assert.notEqual(starter.steelGuard.fingerprint, starter.emberWard.fingerprint);
});
