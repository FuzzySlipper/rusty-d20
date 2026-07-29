import { defineD20Module } from '@rusty-d20/rules-authoring';

export const emberWardModule = defineD20Module(
  {
    id: 'ember-ward-content',
    path: 'rules/packages/starter-ruleset/src/content/ember_ward.ts',
  },
  ({ action, armor, effect, reaction }) => ({
    armors: [
      armor(10, {
        id: 'runed-robe',
        defense: 'nerve',
        bonus: 2,
        slot: 'body',
      }),
      armor(16, {
        id: 'mindward-charm',
        defense: 'nerve',
        bonus: 1,
        slot: 'neck',
      }),
    ],
    effects: [
      effect(24, {
        id: 'ember-ward',
        defense: 'nerve',
        defenseBonus: 3,
        durationTurns: 1,
        conditions: [],
      }),
      effect(30, {
        id: 'scorched',
        defense: null,
        defenseBonus: 0,
        durationTurns: 2,
        conditions: [{ kind: 'attack-penalty', amount: -1 }],
      }),
    ],
    reactions: [
      reaction(38, {
        id: 'ward-flare',
        defense: 'nerve',
        bonus: 3,
        resource: 'focus',
        cost: 1,
        activationCosts: [{ budget: 'reaction', amount: 1 }],
        effect: 'ember-ward',
      }),
    ],
    actions: [
      action(48, {
        id: 'fire-bolt',
        tags: ['attack', 'energy', 'ranged'],
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
          damage: { kind: 'energy', dice: 2, sides: 6, bonus: 0 },
          range: 8,
        },
        effect: 'scorched',
      }),
      action(55, {
        id: 'mind-spike',
        tags: ['attack', 'mental', 'ranged'],
        activationCosts: [{ budget: 'standard-action', amount: 1 }],
        target: {
          kind: 'participant',
          team: 'hostile',
          maximumTargets: 1,
          lineOfEffect: 'required',
        },
        attack: {
          kind: 'fixed',
          ability: 'conviction',
          defense: 'nerve',
          damage: { kind: 'resolve', dice: 1, sides: 8, bonus: 1 },
          range: 8,
        },
        effect: null,
      }),
    ],
  }),
);
