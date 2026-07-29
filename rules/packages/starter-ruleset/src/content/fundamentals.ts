import { defineD20Module } from '@rusty-d20/rules-authoring';

export const fundamentalsModule = defineD20Module(
  {
    id: 'starter-fundamentals',
    path: 'rules/packages/starter-ruleset/src/content/fundamentals.ts',
  },
  ({ activationBudget, damageType, defense, resource }) => ({
    defenses: [
      defense(10, { id: 'armor', base: 10, abilities: ['finesse'] }),
      defense(11, { id: 'grit', base: 10, abilities: ['might'] }),
      defense(12, {
        id: 'wits',
        base: 10,
        abilities: ['acuity', 'intellect'],
      }),
      defense(13, {
        id: 'nerve',
        base: 10,
        abilities: ['conviction', 'spirit'],
      }),
    ],
    activationBudgets: [
      activationBudget(16, {
        id: 'standard-action',
        timing: 'action',
        initial: 1,
      }),
      activationBudget(17, {
        id: 'bonus-action',
        timing: 'action',
        initial: 1,
      }),
      activationBudget(18, {
        id: 'reaction',
        timing: 'reaction',
        initial: 1,
      }),
      activationBudget(19, {
        id: 'movement',
        timing: 'action',
        initial: 6,
      }),
    ],
    damageTypes: ['impact', 'projectile', 'energy', 'resolve'].map((id) =>
      damageType(14, { id }),
    ),
    resources: [
      resource(24, { id: 'guard', maximum: 2 }),
      resource(25, { id: 'focus', maximum: 3 }),
      resource(26, { id: 'resolve-points', maximum: 2 }),
    ],
  }),
);
