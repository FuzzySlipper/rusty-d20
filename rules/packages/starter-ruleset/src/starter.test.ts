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

test('starter catalog keeps rules, adventure content, and content-only extension exact', () => {
  const starter = authorStarterArtifacts();
  assert.deepEqual(
    starter.steelGuard.package.payload.actions?.map((action) => action.id),
    ['disrupt', 'longsword-strike', 'pin-in-place', 'precise-shot'],
  );
  assert.equal(starter.core.package.payload.abilities?.length, 6);
  assert.equal(starter.core.package.payload.defenses?.length, 4);
  assert.equal(starter.core.package.payload.activationBudgets?.length, 4);
  assert.deepEqual(
    starter.steelGuard.package.payload.implements?.map(
      (implement) => implement.id,
    ),
    ['field-bow', 'training-blade'],
  );
  assert.deepEqual(
    starter.emberWard.package.payload.actions?.map((action) => action.id),
    ['fire-bolt', 'mind-spike'],
  );
  assert.notEqual(starter.steelGuard.fingerprint, starter.emberWard.fingerprint);
  assert.deepEqual(
    starter.wardensGate.package.sources.map(({ path }) => path),
    [
      'rules/packages/starter-ruleset/src/content/adventures/warden_cast.ts',
      'rules/packages/starter-ruleset/src/content/adventures/warden_loadout.ts',
      'rules/packages/starter-ruleset/src/content/adventures/wardens_gate.ts',
    ],
  );
  assert.equal(
    starter.wardensGate.package.payload.adventures?.[0]?.id,
    'wardens-gate',
  );
  assert.deepEqual(
    starter.embersWake.package.sources.map(({ path }) => path),
    [
      'rules/packages/starter-ruleset/src/content/adventures/ember_cast.ts',
      'rules/packages/starter-ruleset/src/content/adventures/ember_loadout.ts',
      'rules/packages/starter-ruleset/src/content/adventures/embers_wake.ts',
    ],
  );
  assert.equal(starter.embersWake.package.payload.adventures?.[0]?.id, 'embers-wake');
  assert.equal(
    starter.embersWake.package.dependencies[0]?.fingerprint,
    starter.emberWard.fingerprint,
  );
  assert.equal(
    starter.catalogProbe.package.payload.adventures?.[0]?.id,
    'catalog-probe',
  );
  assert.equal(
    starter.catalogProbe.package.dependencies[0]?.fingerprint,
    starter.wardensGate.fingerprint,
  );
  assert.deepEqual(
    starter.wardensGate.package.provenance
      .filter(({ subject }) => subject.startsWith('adventure:'))
      .map(({ subject, source, line }) => ({ subject, source, line })),
    [
      {
        subject: 'adventure:wardens-gate',
        source: 'wardens-gate-adventure',
        line: 52,
      },
    ],
  );
  assert.deepEqual(
    starter.embersWake.package.provenance
      .filter(({ subject }) => subject.startsWith('adventure:'))
      .map(({ subject, source, line }) => ({ subject, source, line })),
    [
      {
        subject: 'adventure:embers-wake',
        source: 'embers-wake-adventure',
        line: 52,
      },
    ],
  );
});
