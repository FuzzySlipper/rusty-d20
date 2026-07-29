import { defineD20Module } from '@rusty-d20/rules-authoring';

const completeResources = [
  { resource: 'focus', current: 3 },
  { resource: 'guard', current: 2 },
  { resource: 'resolve-points', current: 2 },
] as const;

export const wardenCastModule = defineD20Module(
  {
    id: 'warden-cast',
    path: 'rules/packages/starter-ruleset/src/content/adventures/warden_cast.ts',
  },
  ({ characterTemplate }) => ({
    characterTemplates: [
      characterTemplate(16, {
        id: 'mara-venn',
        entityId: 101,
        name: 'Mara Venn',
        title: 'Steel Adept',
        level: 1,
        vitality: 24,
        inventoryCapacity: 4,
        abilities: [
          { ability: 'acuity', score: 12 },
          { ability: 'conviction', score: 12 },
          { ability: 'finesse', score: 14 },
          { ability: 'intellect', score: 10 },
          { ability: 'might', score: 18 },
          { ability: 'spirit', score: 10 },
        ],
        resources: [...completeResources],
        actions: [
          'longsword-strike',
          'precise-shot',
          'pin-in-place',
          'disrupt',
        ],
        reactions: ['parry'],
        affinities: [],
      }),
      characterTemplate(35, {
        id: 'iron-warden',
        entityId: 102,
        name: 'Iron Warden',
        title: 'Gate Sentinel',
        level: 1,
        vitality: 24,
        inventoryCapacity: 3,
        abilities: [
          { ability: 'acuity', score: 10 },
          { ability: 'conviction', score: 12 },
          { ability: 'finesse', score: 12 },
          { ability: 'intellect', score: 12 },
          { ability: 'might', score: 14 },
          { ability: 'spirit', score: 14 },
        ],
        resources: [...completeResources],
        actions: [
          'longsword-strike',
          'precise-shot',
          'pin-in-place',
          'disrupt',
        ],
        reactions: ['parry'],
        affinities: [{ damageType: 'impact', affinity: 'resistant' }],
      }),
    ],
  }),
);
