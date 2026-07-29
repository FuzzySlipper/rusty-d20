import { defineD20Module } from '@rusty-d20/rules-authoring';

export const wardensGateModule = defineD20Module(
  {
    id: 'wardens-gate-adventure',
    path: 'rules/packages/starter-ruleset/src/content/adventures/wardens_gate.ts',
  },
  ({ adventure, encounter }) => ({
    encounters: [
      encounter(10, {
        id: 'iron-warden',
        title: 'The Iron Warden',
        summary: 'Challenge the armored sentinel guarding the mountain pass.',
        opponent: 'iron-warden',
        availableFromCamp: true,
        introductionSource: 'Encounter',
        introductionText: 'Mara Venn faces the Iron Warden.',
        introductionDetails: [
          "Iron Warden's chain armor and slashing resistance are active sources.",
        ],
        victory: {
          title: 'The Iron Warden defeated',
          summary:
            'Mara prevailed; her remaining vitality and resources carry forward.',
          logSource: 'Victory',
          logText:
            'The Iron Warden falls and yields the Warden chain armor.',
          logDetails: [
            'Warden chain armor was unequipped and transferred into canonical camp storage.',
          ],
          rewardItem: 'warden-chain',
          rewardLabel: 'Warden chain armor',
          recoveryVitality: null,
        },
        defeat: {
          title: 'Mara was defeated',
          summary:
            'No reward was granted; returning to camp applies bounded recovery.',
          logSource: 'Defeat',
          logText: 'Mara Venn falls and must recover before continuing.',
          logDetails: [
            'Mara reached zero vitality; no reward or inventory mutation occurred.',
            'Return to camp applies the explicit bounded recovery consequence.',
          ],
          rewardItem: null,
          rewardLabel: null,
          recoveryVitality: 12,
        },
      }),
    ],
    adventures: [
      adventure(52, {
        id: 'wardens-gate',
        title: "The Warden's Gate",
        default: true,
        hero: 'mara-venn',
        characters: ['mara-venn', 'iron-warden'],
        campStorage: 'camp-stash',
        storage: ['camp-stash'],
        items: [
          'warden-chain',
          'mara-chain',
          'mara-buckler',
          'spare-buckler',
        ],
        encounters: ['iron-warden'],
        startSource: 'Adventure',
        startText: "Mara Venn prepares at the Warden's Gate camp.",
        startDetails: [
          'Starter Core + Steel Guard authored packages compiled by Rust.',
          'The Iron Warden encounter is available from camp.',
        ],
      }),
    ],
  }),
);
