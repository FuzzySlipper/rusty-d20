import { defineD20Module } from '@rusty-d20/rules-authoring';

export const embersWakeModule = defineD20Module(
  {
    id: 'embers-wake-adventure',
    path: 'rules/packages/starter-ruleset/src/content/adventures/embers_wake.ts',
  },
  ({ adventure, encounter }) => ({
    encounters: [
      encounter(10, {
        id: 'ash-seer',
        title: 'The Ash Seer',
        summary: 'Break the psychic ward around the ember reliquary.',
        roster: [
          { character: 'sera-vale', faction: 'party' },
          { character: 'ash-seer', faction: 'opposition' },
        ],
        board: {
          width: 10,
          height: 7,
          rows: [
            '##########',
            '#........#',
            '#..#.....#',
            '#........#',
            '#.....#..#',
            '#........#',
            '##########',
          ],
          placements: [
            { character: 'sera-vale', x: 1, y: 3 },
            { character: 'ash-seer', x: 8, y: 3 },
          ],
        },
        availableFromCamp: true,
        introductionSource: 'Encounter',
        introductionText: 'Sera Vale enters the Ash Seer reliquary.',
        introductionDetails: [
          "The Ash Seer's mindward charm and psychic resistance are active sources.",
        ],
        victory: {
          title: 'The Ash Seer defeated',
          summary: 'Sera claimed the reliquary and kept her remaining focus.',
          logSource: 'Victory',
          logText: "The Ash Seer's ward breaks and yields its mindward charm.",
          logDetails: [
            "The Ash Seer's charm was unequipped and transferred into canonical ember camp storage.",
          ],
          rewardItem: 'seer-charm',
          rewardLabel: "Ash Seer's mindward charm",
          recoveryVitality: null,
        },
        defeat: {
          title: 'Sera was defeated',
          summary:
            'No reward was granted; returning to camp rekindles bounded vitality.',
          logSource: 'Defeat',
          logText: 'Sera Vale falls beneath the reliquary ward.',
          logDetails: [
            'Sera reached zero vitality; no reward or inventory mutation occurred.',
            'Return to camp applies the authored bounded recovery consequence.',
          ],
          rewardItem: null,
          rewardLabel: null,
          recoveryVitality: 11,
        },
      }),
    ],
    adventures: [
      adventure(52, {
        id: 'embers-wake',
        title: "Ember's Wake",
        default: false,
        selectable: true,
        party: ['sera-vale'],
        characters: ['sera-vale', 'ash-seer'],
        campStorage: 'ember-camp-stash',
        storage: ['ember-camp-stash'],
        items: ['seer-charm', 'sera-robe', 'sera-charm', 'spare-robe'],
        encounters: ['ash-seer'],
        dungeon: {
          title: 'Ember Reliquary',
          wallStyle: 'ember-vault',
          width: 9,
          height: 7,
          rows: [
            '#########',
            '#.......#',
            '#.#####.#',
            '#.#...#.#',
            '#.#.#.#.#',
            '#...#...#',
            '#########',
          ],
          startX: 1,
          startY: 1,
          startCheckpoint: 'ember-camp',
          startFacing: 'east',
          encounters: [{ encounter: 'ash-seer', x: 7, y: 5 }],
          landmarks: [
            {
              id: 'ember-inscription',
              x: 7,
              y: 1,
              title: 'Ash-written warning',
              text: 'Warm script coils across the stone: only a focused mind may pass.',
            },
          ],
          doors: [],
          treasures: [],
          checkpoints: [
            {
              id: 'ember-camp',
              x: 1,
              y: 1,
              title: 'Reliquary threshold',
              text: 'Sera can leave the reliquary safely from this threshold.',
            },
          ],
        },
        startSource: 'Adventure',
        startText: 'Sera Vale prepares beside the ember reliquary.',
        startDetails: [
          'Ruleweaver foundation + Ember Ward authored packages compiled by Rust.',
          'The Ash Seer encounter uses focus, Wits, Nerve, energy, and resolve mechanics.',
        ],
        completion: {
          source: 'Ember Reliquary',
          victoryTitle: "Ember's Wake complete",
          victoryText:
            'Sera carries the recovered charm beyond the cooling reliquary.',
          defeatTitle: "Ember's Wake ended",
          defeatText:
            'Sera survives the reliquary, but its ward remains unbroken.',
          details: [
            'The terminal outcome and every remaining resource are part of the complete Rust save.',
          ],
        },
      }),
    ],
  }),
);
