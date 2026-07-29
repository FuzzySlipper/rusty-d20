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
        ],
        encounters: ['iron-warden'],
        startSource: 'Adventure',
        startText: 'The content-only catalog probe is ready.',
        startDetails: [
          'This non-default adventure reuses admitted primitives without Rust orchestration changes.',
        ],
      }),
    ],
  }),
);
