import { defineD20Module } from '@rusty-d20/rules-authoring';

export const catalogProbeModule = defineD20Module(
  {
    id: 'catalog-probe-adventure',
    path: 'rules/packages/starter-ruleset/src/content/adventures/catalog_probe.ts',
  },
  ({ adventure }) => ({
    adventures: [
      adventure(10, {
        id: 'catalog-probe',
        title: 'Authored Catalog Probe',
        default: false,
        selectable: false,
        hero: 'mara-venn',
        characters: ['iron-warden', 'mara-venn'],
        campStorage: 'camp-stash',
        storage: ['camp-stash'],
        items: [
          'mara-buckler',
          'mara-chain',
          'spare-buckler',
          'warden-chain',
          'warden-blade',
          'mara-blade',
          'warden-bow',
          'mara-bow',
        ],
        encounters: ['iron-warden'],
        dungeon: {
          title: 'Catalog Probe',
          wallStyle: 'probe',
          width: 5,
          height: 5,
          rows: ['#####', '#...#', '#.#.#', '#...#', '#####'],
          startX: 1,
          startY: 1,
          checkpointX: 1,
          checkpointY: 1,
          startFacing: 'east',
          encounters: [{ encounter: 'iron-warden', x: 3, y: 3 }],
          landmarks: [],
        },
        startSource: 'Adventure',
        startText: 'The content-only catalog probe is ready.',
        startDetails: [
          'This non-default adventure reuses admitted primitives without Rust orchestration changes.',
        ],
      }),
    ],
  }),
);
