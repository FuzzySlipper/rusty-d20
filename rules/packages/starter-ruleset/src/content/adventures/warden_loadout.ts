import { defineD20Module } from '@rusty-d20/rules-authoring';

export const wardenLoadoutModule = defineD20Module(
  {
    id: 'warden-loadout',
    path: 'rules/packages/starter-ruleset/src/content/adventures/warden_loadout.ts',
  },
  ({ itemInstance, storage }) => ({
    storage: [
      storage(10, {
        id: 'camp-stash',
        entityId: 103,
        name: 'Camp stash',
        capacity: 8,
      }),
    ],
    itemInstances: [
      itemInstance(18, {
        id: 'warden-chain',
        entityId: 201,
        name: 'Warden chain armor',
        armor: 'chain-armor',
        owner: 'iron-warden',
        icon: '🛡️',
        rarity: 'uncommon',
        equipped: true,
      }),
      itemInstance(28, {
        id: 'mara-chain',
        entityId: 202,
        name: "Mara's chain armor",
        armor: 'chain-armor',
        owner: 'mara-venn',
        icon: '🛡️',
        rarity: 'uncommon',
        equipped: true,
      }),
      itemInstance(38, {
        id: 'mara-buckler',
        entityId: 203,
        name: "Mara's buckler",
        armor: 'buckler',
        owner: 'mara-venn',
        icon: '◈',
        rarity: 'common',
        equipped: true,
      }),
      itemInstance(48, {
        id: 'spare-buckler',
        entityId: 204,
        name: 'Spare buckler',
        armor: 'buckler',
        owner: 'camp-stash',
        icon: '◈',
        rarity: 'common',
        equipped: false,
      }),
    ],
  }),
);
