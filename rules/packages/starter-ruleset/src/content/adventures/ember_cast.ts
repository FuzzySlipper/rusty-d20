import { defineD20Module } from '@rusty-d20/rules-authoring';

const completeResources = [
  { resource: 'focus', current: 3 },
  { resource: 'guard', current: 2 },
  { resource: 'resolve-points', current: 2 },
] as const;

export const emberCastModule = defineD20Module(
  {
    id: 'ember-cast',
    path: 'rules/packages/starter-ruleset/src/content/adventures/ember_cast.ts',
  },
  ({ characterTemplate }) => ({
    characterTemplates: [
      characterTemplate(16, {
        id: 'sera-vale',
        entityId: 111,
        name: 'Sera Vale',
        title: 'Ember Adept',
        level: 1,
        vitality: 22,
        inventoryCapacity: 2,
        abilities: [
          { ability: 'acuity', score: 18 },
          { ability: 'conviction', score: 18 },
          { ability: 'finesse', score: 12 },
          { ability: 'intellect', score: 14 },
          { ability: 'might', score: 12 },
          { ability: 'spirit', score: 12 },
        ],
        resources: [...completeResources],
        actions: ['fire-bolt', 'mind-spike'],
        reactions: ['ward-flare'],
        affinities: [{ damageType: 'energy', affinity: 'resistant' }],
      }),
      characterTemplate(35, {
        id: 'ash-seer',
        entityId: 112,
        name: 'Ash Seer',
        title: 'Reliquary Keeper',
        level: 1,
        vitality: 22,
        inventoryCapacity: 1,
        abilities: [
          { ability: 'acuity', score: 16 },
          { ability: 'conviction', score: 16 },
          { ability: 'finesse', score: 10 },
          { ability: 'intellect', score: 14 },
          { ability: 'might', score: 12 },
          { ability: 'spirit', score: 14 },
        ],
        resources: [...completeResources],
        actions: ['mind-spike', 'fire-bolt'],
        reactions: ['ward-flare'],
        affinities: [{ damageType: 'resolve', affinity: 'resistant' }],
      }),
    ],
  }),
);
