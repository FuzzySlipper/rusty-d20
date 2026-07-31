import assert from "node:assert/strict";
import test from "node:test";

import {
  RuleContractError,
  admitRuleDiagnostics,
} from "@rusty-engine/gameplay-rules-contracts";

import {
  D20AuthoringError,
  authorD20Package,
  defineD20Module,
  mapD20Diagnostic,
} from "./index.js";

const source = Object.freeze({
  id: "test-source",
  path: "rules/test-content.ts",
});

test("module order does not change canonical package identity", () => {
  const abilities = defineD20Module(source, ({ ability }) => ({
    abilities: [ability(12, { id: "might", minimum: 1, maximum: 30 })],
  }));
  const combat = defineD20Module(
    { id: "combat-source", path: "rules/test-combat.ts" },
    ({ action, activationBudget, damageType, defense }) => ({
      defenses: [defense(8, { id: "armor", base: 10, abilities: ["might"] })],
      activationBudgets: [
        activationBudget(10, {
          id: "standard-action",
          timing: "action",
          initial: 1,
        }),
      ],
      damageTypes: [damageType(12, { id: "force" })],
      actions: [
        action(16, {
          id: "shove",
          tags: ["attack"],
          activationCosts: [{ budget: "standard-action", amount: 1 }],
          target: {
            kind: "participant",
            team: "hostile",
            maximumTargets: 1,
            lineOfEffect: "required",
          },
          attack: {
            kind: "fixed",
            ability: "might",
            defense: "armor",
            damage: { kind: "force", dice: 1, sides: 4, bonus: 0 },
            range: 1,
          },
          effect: null,
          forcedMovement: 0,
        }),
      ],
    }),
  );

  const left = authorD20Package({
    domain: "rusty-d20",
    package: "ordering",
    version: 1,
    modules: [abilities, combat],
  });
  const right = authorD20Package({
    domain: "rusty-d20",
    package: "ordering",
    version: 1,
    modules: [combat, abilities],
  });
  assert.equal(left.canonicalJson, right.canonicalJson);
  assert.equal(left.fingerprint, right.fingerprint);
  assert(Object.isFrozen(left.package.payload));
  assert(Object.isFrozen(left.package.payload.actions));
});

test("invalid d20 identities fail at their authored source", () => {
  assert.throws(
    () =>
      defineD20Module(source, ({ ability }) => ({
        abilities: [
          ability(27, { id: "Not Valid", minimum: 1, maximum: 30 }, 5),
        ],
      })),
    (error: unknown) =>
      error instanceof D20AuthoringError &&
      error.code === "invalid-d20-identity" &&
      error.message.startsWith("rules/test-content.ts:27:5:"),
  );

  assert.throws(
    () =>
      defineD20Module(source, ({ characterTemplate }) => ({
        characterTemplates: [
          characterTemplate(41, {
            id: "hero",
            entityId: 1,
            name: "Hero",
            title: "Tester",
            level: 1,
            experience: 0,
            vitality: 10,
            inventoryCapacity: 1,
            abilities: [],
            resources: [],
            actions: ["Not Valid"],
            reactions: [],
            affinities: [],
            features: [],
          }),
        ],
      })),
    (error: unknown) =>
      error instanceof D20AuthoringError &&
      error.code === "invalid-d20-identity" &&
      error.message.startsWith("rules/test-content.ts:41:1:") &&
      error.message.includes("actions is not a valid d20 identity"),
  );
});

test("authored adventure payloads are deeply immutable and retain provenance", () => {
  const module = defineD20Module(source, ({ adventure, storage }) => ({
    storage: [
      storage(10, {
        id: "camp",
        entityId: 1,
        name: "Camp",
        capacity: 4,
      }),
    ],
    adventures: [
      adventure(20, {
        id: "test-adventure",
        title: "Test Adventure",
        default: true,
        selectable: true,
        party: ["hero"],
        characters: ["hero", "opponent"],
        campStorage: "camp",
        storage: ["camp"],
        items: [],
        encounters: ["encounter"],
        dungeon: {
          title: "Test Dungeon",
          wallStyle: "test-stone",
          width: 5,
          height: 5,
          rows: ["#####", "#...#", "#.#.#", "#...#", "#####"],
          startX: 1,
          startY: 1,
          startCheckpoint: "test-camp",
          startFacing: "east",
          encounters: [{ encounter: "encounter", x: 3, y: 3 }],
          landmarks: [],
          doors: [],
          treasures: [],
          checkpoints: [
            {
              id: "test-camp",
              x: 1,
              y: 1,
              title: "Test camp",
              text: "The test party can return here.",
            },
          ],
        },
        startSource: "Adventure",
        startText: "The test starts.",
        startDetails: ["Authored, not hardcoded."],
        completion: {
          source: "Adventure",
          victoryTitle: "Test complete",
          victoryText: "The test adventure ended in victory.",
          defeatTitle: "Test ended",
          defeatText: "The test adventure ended in defeat.",
          details: ["Authored completion text is immutable."],
        },
      }),
    ],
  }));
  const artifact = authorD20Package({
    domain: "rusty-d20",
    package: "authored-adventure",
    version: 1,
    modules: [module],
  });
  assert(Object.isFrozen(artifact.package.payload.adventures));
  assert(Object.isFrozen(artifact.package.payload.adventures?.[0]?.characters));
  assert.deepEqual(
    artifact.package.provenance
      .filter(({ subject }) => subject.startsWith("adventure:"))
      .map(({ subject, source: sourceId, line }) => ({
        subject,
        source: sourceId,
        line,
      })),
    [
      {
        subject: "adventure:test-adventure",
        source: "test-source",
        line: 20,
      },
    ],
  );
});

test("neutral envelope version rejection stays typed", () => {
  const module = defineD20Module(source, ({ ability }) => ({
    abilities: [ability(4, { id: "strength", minimum: 1, maximum: 30 })],
  }));
  assert.throws(
    () =>
      authorD20Package({
        domain: "rusty-d20",
        package: "bad-version",
        version: 0,
        modules: [module],
      }),
    (error: unknown) =>
      error instanceof RuleContractError && error.logicalPath === "$/version",
  );
});

test("canonical Rust diagnostic correlation maps to a source path", () => {
  const module = defineD20Module(source, ({ ability }) => ({
    abilities: [ability(31, { id: "strength", minimum: 0, maximum: 40 }, 3)],
  }));
  const artifact = authorD20Package({
    domain: "rusty-d20",
    package: "diagnostic-map",
    version: 1,
    modules: [module],
  });
  const [diagnostic] = admitRuleDiagnostics([
    {
      code: "D20_INVALID_ABILITY_RANGE",
      severity: "error",
      logicalPath: "$/payload/abilities/strength",
      message: "ability score bounds must be ordered inside 1..=30",
      package: {
        domain: "rusty-d20",
        package: "diagnostic-map",
        version: 1,
      },
      correlation: {
        subject: "ability:strength",
        source: "test-source",
        line: 31,
        column: 3,
      },
    },
  ]);
  assert(diagnostic !== undefined);
  assert.deepEqual(mapD20Diagnostic(artifact, diagnostic).source, {
    path: "rules/test-content.ts",
    line: 31,
    column: 3,
  });
});
