import { defineD20Module } from '@rusty-d20/rules-authoring';

export const fundamentalsModule = defineD20Module(
  {
    id: 'starter-fundamentals',
    path: 'rules/packages/starter-ruleset/src/content/fundamentals.ts',
  },
  ({ damageType, defense, resource }) => ({
    defenses: [
      defense(10, { id: 'armor', base: 10, ability: 'dexterity' }),
      defense(11, { id: 'resolve', base: 10, ability: 'wisdom' }),
      defense(12, { id: 'fortitude', base: 10, ability: 'constitution' }),
    ],
    damageTypes: ['slashing', 'piercing', 'fire', 'psychic'].map(
      (id) => damageType(14, { id }),
    ),
    resources: [
      resource(18, { id: 'guard', maximum: 2 }),
      resource(19, { id: 'focus', maximum: 3 }),
      resource(20, { id: 'resolve-points', maximum: 2 }),
    ],
  }),
);
