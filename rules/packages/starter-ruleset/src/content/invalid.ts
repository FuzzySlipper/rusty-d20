import { defineD20Module } from '@rusty-d20/rules-authoring';

export const invalidSemanticsModule = defineD20Module(
  {
    id: 'invalid-semantics',
    path: 'rules/packages/starter-ruleset/src/content/invalid.ts',
  },
  ({ ability, action }) => ({
    abilities: [
      ability(10, { id: 'impossible-score', minimum: 0, maximum: 40 }),
    ],
    actions: [
      action(14, {
        id: 'broken-strike',
        ability: 'missing-ability',
        defense: 'armor',
        damage: { kind: 'slashing', dice: 1, sides: 8, bonus: 0 },
        effect: null,
      }),
    ],
  }),
);
