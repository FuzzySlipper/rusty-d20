import { defineD20Module } from '@rusty-d20/rules-authoring';

export const emberLoadoutModule = defineD20Module(
  {
    id: 'ember-loadout',
    path: 'rules/packages/starter-ruleset/src/content/adventures/ember_loadout.ts',
  },
  ({ itemInstance, storage }) => ({
    storage: [
      storage(10, {
        id: 'ember-camp-stash',
        entityId: 113,
        name: 'Ember camp stash',
        capacity: 8,
      }),
    ],
    itemInstances: [
      itemInstance(18, {
        id: 'seer-charm',
        entityId: 211,
        name: "Ash Seer's mindward charm",
        armor: 'mindward-charm',
        owner: 'ash-seer',
        icon: '✦',
        rarity: 'rare',
        equipped: true,
      }),
      itemInstance(28, {
        id: 'sera-robe',
        entityId: 212,
        name: "Sera's runed robe",
        armor: 'runed-robe',
        owner: 'sera-vale',
        icon: '♨',
        rarity: 'uncommon',
        equipped: true,
      }),
      itemInstance(38, {
        id: 'sera-charm',
        entityId: 213,
        name: "Sera's mindward charm",
        armor: 'mindward-charm',
        owner: 'sera-vale',
        icon: '✦',
        rarity: 'uncommon',
        equipped: true,
      }),
      itemInstance(48, {
        id: 'spare-robe',
        entityId: 214,
        name: 'Spare runed robe',
        armor: 'runed-robe',
        owner: 'ember-camp-stash',
        icon: '♨',
        rarity: 'common',
        equipped: false,
      }),
    ],
  }),
);
