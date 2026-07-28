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
        defense: 'resolve',
        bonus: 2,
        slot: 'body',
      }),
      armor(16, {
        id: 'mindward-charm',
        defense: 'resolve',
        bonus: 1,
        slot: 'neck',
      }),
    ],
    effects: [
      effect(24, {
        id: 'ember-ward',
        defense: 'resolve',
        defenseBonus: 3,
        durationTurns: 1,
      }),
      effect(30, {
        id: 'scorched',
        defense: null,
        defenseBonus: 0,
        durationTurns: 2,
      }),
    ],
    reactions: [
      reaction(38, {
        id: 'ward-flare',
        defense: 'resolve',
        bonus: 3,
        resource: 'focus',
        cost: 1,
        effect: 'ember-ward',
      }),
    ],
    actions: [
      action(48, {
        id: 'fire-bolt',
        ability: 'wisdom',
        defense: 'resolve',
        damage: { kind: 'fire', dice: 2, sides: 6, bonus: 0 },
        effect: 'scorched',
      }),
      action(55, {
        id: 'mind-spike',
        ability: 'wisdom',
        defense: 'resolve',
        damage: { kind: 'psychic', dice: 1, sides: 8, bonus: 1 },
        effect: null,
      }),
    ],
  }),
);
