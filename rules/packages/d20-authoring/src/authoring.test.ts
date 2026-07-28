import assert from 'node:assert/strict';
import test from 'node:test';

import {
  RuleContractError,
  admitRuleDiagnostics,
} from '@rusty-engine/gameplay-rules-contracts';

import {
  D20AuthoringError,
  authorD20Package,
  defineD20Module,
  mapD20Diagnostic,
} from './index.js';

const source = Object.freeze({
  id: 'test-source',
  path: 'rules/test-content.ts',
});

test('module order does not change canonical package identity', () => {
  const abilities = defineD20Module(source, ({ ability }) => ({
    abilities: [
      ability(12, { id: 'strength', minimum: 1, maximum: 30 }),
    ],
  }));
  const combat = defineD20Module(
    { id: 'combat-source', path: 'rules/test-combat.ts' },
    ({ action, damageType, defense }) => ({
      defenses: [
        defense(8, { id: 'armor', base: 10, ability: 'strength' }),
      ],
      damageTypes: [damageType(12, { id: 'force' })],
      actions: [
        action(16, {
          id: 'shove',
          ability: 'strength',
          defense: 'armor',
          damage: { kind: 'force', dice: 1, sides: 4, bonus: 0 },
          effect: null,
        }),
      ],
    }),
  );

  const left = authorD20Package({
    domain: 'rusty-d20',
    package: 'ordering',
    version: 1,
    modules: [abilities, combat],
  });
  const right = authorD20Package({
    domain: 'rusty-d20',
    package: 'ordering',
    version: 1,
    modules: [combat, abilities],
  });
  assert.equal(left.canonicalJson, right.canonicalJson);
  assert.equal(left.fingerprint, right.fingerprint);
  assert(Object.isFrozen(left.package.payload));
  assert(Object.isFrozen(left.package.payload.actions));
});

test('invalid d20 identities fail at their authored source', () => {
  assert.throws(
    () =>
      defineD20Module(source, ({ ability }) => ({
        abilities: [
          ability(27, { id: 'Not Valid', minimum: 1, maximum: 30 }, 5),
        ],
      })),
    (error: unknown) =>
      error instanceof D20AuthoringError &&
      error.code === 'invalid-d20-identity' &&
      error.message.startsWith('rules/test-content.ts:27:5:'),
  );
});

test('neutral envelope version rejection stays typed', () => {
  const module = defineD20Module(source, ({ ability }) => ({
    abilities: [
      ability(4, { id: 'strength', minimum: 1, maximum: 30 }),
    ],
  }));
  assert.throws(
    () =>
      authorD20Package({
        domain: 'rusty-d20',
        package: 'bad-version',
        version: 0,
        modules: [module],
      }),
    (error: unknown) =>
      error instanceof RuleContractError &&
      error.logicalPath === '$/version',
  );
});

test('canonical Rust diagnostic correlation maps to a source path', () => {
  const module = defineD20Module(source, ({ ability }) => ({
    abilities: [
      ability(31, { id: 'strength', minimum: 0, maximum: 40 }, 3),
    ],
  }));
  const artifact = authorD20Package({
    domain: 'rusty-d20',
    package: 'diagnostic-map',
    version: 1,
    modules: [module],
  });
  const [diagnostic] = admitRuleDiagnostics([
    {
      code: 'D20_INVALID_ABILITY_RANGE',
      severity: 'error',
      logicalPath: '$/payload/abilities/strength',
      message: 'ability score bounds must be ordered inside 1..=30',
      package: {
        domain: 'rusty-d20',
        package: 'diagnostic-map',
        version: 1,
      },
      correlation: {
        subject: 'ability:strength',
        source: 'test-source',
        line: 31,
        column: 3,
      },
    },
  ]);
  assert(diagnostic !== undefined);
  assert.deepEqual(mapD20Diagnostic(artifact, diagnostic).source, {
    path: 'rules/test-content.ts',
    line: 31,
    column: 3,
  });
});
