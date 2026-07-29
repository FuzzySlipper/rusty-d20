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
        tags: ['attack'],
        activationCosts: [{ budget: 'standard-action', amount: 1 }],
        target: {
          kind: 'participant',
          team: 'hostile',
          maximumTargets: 1,
          lineOfEffect: 'required',
        },
        attack: {
          kind: 'fixed',
          ability: 'missing-ability',
          defense: 'armor',
          damage: { kind: 'impact', dice: 1, sides: 8, bonus: 0 },
          range: 1,
        },
        effect: null,
      }),
    ],
  }),
);
