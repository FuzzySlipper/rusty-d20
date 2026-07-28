import { defineD20Module } from '@rusty-d20/rules-authoring';

const abilityNames = [
  'strength',
  'dexterity',
  'wisdom',
  'constitution',
] as const;

export const abilitiesModule = defineD20Module(
  {
    id: 'starter-abilities',
    path: 'rules/packages/starter-ruleset/src/content/abilities.ts',
  },
  ({ ability }) => ({
    abilities: abilityNames.map((id, index) =>
      ability(4 + index, { id, minimum: 1, maximum: 30 }),
    ),
  }),
);
