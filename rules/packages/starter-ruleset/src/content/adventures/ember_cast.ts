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
          { ability: 'constitution', score: 12 },
          { ability: 'dexterity', score: 12 },
          { ability: 'strength', score: 10 },
          { ability: 'wisdom', score: 18 },
        ],
        resources: [...completeResources],
        actions: ['fire-bolt', 'mind-spike'],
        reactions: ['ward-flare'],
        affinities: [{ damageType: 'fire', affinity: 'resistant' }],
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
          { ability: 'constitution', score: 12 },
          { ability: 'dexterity', score: 10 },
          { ability: 'strength', score: 10 },
          { ability: 'wisdom', score: 16 },
        ],
        resources: [...completeResources],
        actions: ['mind-spike', 'fire-bolt'],
        reactions: ['ward-flare'],
        affinities: [{ damageType: 'psychic', affinity: 'resistant' }],
      }),
    ],
  }),
);
