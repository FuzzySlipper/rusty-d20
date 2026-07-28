import { defineD20Module } from '@rusty-d20/rules-authoring';

export const steelGuardModule = defineD20Module(
  {
    id: 'steel-guard-content',
    path: 'rules/packages/starter-ruleset/src/content/steel_guard.ts',
  },
  ({ action, armor, effect, reaction }) => ({
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
    effects: [
      effect(24, {
        id: 'parry-stance',
        defense: 'armor',
        defenseBonus: 2,
        durationTurns: 1,
      }),
      effect(30, {
        id: 'bleeding',
        defense: null,
        defenseBonus: 0,
        durationTurns: 2,
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
        ability: 'strength',
        defense: 'armor',
        damage: { kind: 'slashing', dice: 1, sides: 8, bonus: 2 },
        effect: 'bleeding',
      }),
      action(55, {
        id: 'precise-shot',
        ability: 'dexterity',
        defense: 'armor',
        damage: { kind: 'piercing', dice: 1, sides: 6, bonus: 1 },
        effect: null,
      }),
    ],
  }),
);
