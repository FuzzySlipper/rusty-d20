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
        inventoryCapacity: 2,
        abilities: [
          { ability: 'constitution', score: 14 },
          { ability: 'dexterity', score: 14 },
          { ability: 'strength', score: 18 },
          { ability: 'wisdom', score: 12 },
        ],
        resources: [...completeResources],
        actions: ['longsword-strike', 'precise-shot'],
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
        inventoryCapacity: 1,
        abilities: [
          { ability: 'constitution', score: 14 },
          { ability: 'dexterity', score: 12 },
          { ability: 'strength', score: 14 },
          { ability: 'wisdom', score: 12 },
        ],
        resources: [...completeResources],
        actions: ['longsword-strike', 'precise-shot'],
        reactions: ['parry'],
        affinities: [
          { damageType: 'slashing', affinity: 'resistant' },
        ],
      }),
    ],
  }),
);
