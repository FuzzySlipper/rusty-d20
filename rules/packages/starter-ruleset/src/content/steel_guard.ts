import { defineD20Module } from '@rusty-d20/rules-authoring';

export const steelGuardModule = defineD20Module(
  {
    id: 'steel-guard-content',
    path: 'rules/packages/starter-ruleset/src/content/steel_guard.ts',
  },
  ({ action, armor, effect, implement, reaction }) => ({
    armors: [
      armor(10, {
        id: 'chain-armor',
        defense: 'armor',
        bonus: 4,
        slot: 'body',
      }),
      armor(16, {
        id: 'buckler',
        defense: 'armor',
        bonus: 2,
        slot: 'off-hand',
      }),
    ],
    implements: [
      implement(20, {
        id: 'training-blade',
        slot: 'main-hand',
        tags: ['melee', 'weapon'],
        ability: 'might',
        defense: 'armor',
        damage: { kind: 'impact', dice: 1, sides: 8, bonus: 2 },
        range: 1,
      }),
      implement(21, {
        id: 'field-bow',
        slot: 'ranged-hand',
        tags: ['ranged', 'weapon'],
        ability: 'finesse',
        defense: 'armor',
        damage: { kind: 'projectile', dice: 1, sides: 6, bonus: 1 },
        range: 8,
      }),
    ],
    effects: [
      effect(24, {
        id: 'parry-stance',
        defense: 'armor',
        defenseBonus: 2,
        durationTurns: 1,
        conditions: [],
      }),
      effect(30, {
        id: 'bleeding',
        defense: null,
        defenseBonus: 0,
        durationTurns: 2,
        conditions: [],
      }),
      effect(31, {
        id: 'held',
        defense: null,
        defenseBonus: 0,
        durationTurns: 1,
        conditions: [{ kind: 'forbid-movement' }],
      }),
      effect(32, {
        id: 'unsettled',
        defense: null,
        defenseBonus: 0,
        durationTurns: 1,
        conditions: [
          { kind: 'forbid-action-tag', tag: 'control' },
          { kind: 'attack-penalty', amount: -2 },
        ],
      }),
    ],
    reactions: [
      reaction(38, {
        id: 'parry',
        defense: 'armor',
        bonus: 2,
        resource: 'guard',
        cost: 1,
        effect: 'parry-stance',
      }),
    ],
    actions: [
      action(48, {
        id: 'longsword-strike',
        tags: ['attack', 'melee', 'weapon'],
        activationCosts: [{ budget: 'standard-action', amount: 1 }],
        target: {
          kind: 'participant',
          team: 'hostile',
          maximumTargets: 1,
          lineOfEffect: 'required',
        },
        attack: { kind: 'implement', implement: 'training-blade' },
        effect: 'bleeding',
      }),
      action(55, {
        id: 'precise-shot',
        tags: ['attack', 'ranged', 'weapon'],
        activationCosts: [{ budget: 'standard-action', amount: 1 }],
        target: {
          kind: 'participant',
          team: 'hostile',
          maximumTargets: 1,
          lineOfEffect: 'required',
        },
        attack: { kind: 'implement', implement: 'field-bow' },
        effect: null,
      }),
      action(62, {
        id: 'pin-in-place',
        tags: ['attack', 'control', 'melee'],
        activationCosts: [{ budget: 'standard-action', amount: 1 }],
        target: {
          kind: 'participant',
          team: 'hostile',
          maximumTargets: 1,
          lineOfEffect: 'required',
        },
        attack: {
          kind: 'fixed',
          ability: 'might',
          defense: 'grit',
          damage: { kind: 'impact', dice: 1, sides: 4, bonus: 0 },
          range: 1,
        },
        effect: 'held',
      }),
      action(76, {
        id: 'disrupt',
        tags: ['attack', 'control', 'ranged'],
        activationCosts: [{ budget: 'standard-action', amount: 1 }],
        target: {
          kind: 'participant',
          team: 'hostile',
          maximumTargets: 1,
          lineOfEffect: 'required',
        },
        attack: {
          kind: 'fixed',
          ability: 'acuity',
          defense: 'wits',
          damage: { kind: 'resolve', dice: 1, sides: 4, bonus: 0 },
          range: 8,
        },
        effect: 'unsettled',
      }),
    ],
  }),
);
