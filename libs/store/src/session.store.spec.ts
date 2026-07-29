import { describe, expect, it } from 'vitest';
import type { GameSnapshotDto, Result, RuntimeReadoutDto } from '@rusty-d20/protocol';
import type { RustyD20Transport } from '@rusty-d20/transport';
import { SessionStore } from './index';

const readout: RuntimeReadoutDto = {
  engineRevision: 'fb608e323a8b44a55195f5720101224ff37fd5db',
  entityCount: 0,
  product: 'Rusty D20',
  status: 'ready',
  version: '0.1.0',
};

const snapshot: GameSnapshotDto = {
  product: 'Rusty D20',
  version: '0.1.0',
  engineRevision: 'fb608e323a8b44a55195f5720101224ff37fd5db',
  rulesetFingerprint: 'rules',
  revision: 1,
  saved: false,
  availableAdventures: [
    {
      id: 'wardens-gate',
      title: "The Warden's Gate",
      summary: "Mara Venn prepares at the Warden's Gate camp.",
      details: [],
    },
    {
      id: 'embers-wake',
      title: "Ember's Wake",
      summary: 'Sera Vale prepares beside the ember reliquary.',
      details: [],
    },
  ],
  campaign: null,
  encounter: null,
};

function transport(overrides: Partial<RustyD20Transport> = {}): RustyD20Transport {
  const sessionResult: Result<GameSnapshotDto> = { ok: true, value: snapshot };
  return {
    loadReadout: async () => ({ ok: true, value: readout }),
    loadSession: async () => sessionResult,
    newAdventure: async () => sessionResult,
    enterEncounter: async () => sessionResult,
    equipItem: async () => sessionResult,
    unequipItem: async () => sessionResult,
    transferItem: async () => sessionResult,
    previewAction: async () => sessionResult,
    applyReaction: async () => sessionResult,
    applyAction: async () => sessionResult,
    beginOppositionTurn: async () => sessionResult,
    returnToCamp: async () => sessionResult,
    save: async () => sessionResult,
    ...overrides,
  };
}

describe('SessionStore', () => {
  it('publishes Rust-owned camp and encounter phases through named commands', async () => {
    let selectedAdventure: string | undefined;
    const hero = {
      id: 101,
      name: 'Mara Venn',
      title: 'Steel Adept',
      level: 1,
      healthCurrent: 24,
      healthMaximum: 24,
      resources: [],
      effects: [],
    };
    const campaign = {
      id: 'wardens-gate',
      title: "The Warden's Gate",
      phase: 'camp',
      hero,
      loadout: {
        ownerId: 101,
        stashOwnerId: 103,
        inventorySlots: [],
        equipmentSlots: [],
        stashItems: [],
        capacity: { metric: 'carried-items', used: 0, maximum: 0 },
        defenses: [
          { id: 'armor', label: 'Armor', value: 12, sources: [] },
          { id: 'resolve', label: 'Resolve', value: 11, sources: [] },
        ],
      },
      activeEncounterId: null,
      latestOutcome: null,
      availableEncounters: [
        {
          id: 'iron-warden',
          title: 'The Iron Warden',
          summary: 'Challenge the sentinel.',
        },
      ],
    } satisfies NonNullable<GameSnapshotDto['campaign']>;
    const camp: GameSnapshotDto = {
      ...snapshot,
      revision: 2,
      campaign,
    };
    const encounter: GameSnapshotDto = {
      ...camp,
      revision: 3,
      campaign: {
        ...campaign,
        phase: 'encounter',
        activeEncounterId: 'iron-warden',
      },
      encounter: {
        turn: 0,
        nextRoll: 0,
        playerId: 101,
        turnOwner: 'player',
        characters: [hero, { ...hero, id: 102, name: 'Iron Warden', title: 'Armored Sentinel' }],
        actions: [],
        pendingAction: null,
        log: [],
      },
    };
    const store = new SessionStore(
      transport({
        newAdventure: async (request) => {
          selectedAdventure = request.adventureId;
          return { ok: true, value: camp };
        },
        enterEncounter: async () => ({ ok: true, value: encounter }),
      }),
    );

    await store.load();
    await store.newAdventure('wardens-gate');
    expect(selectedAdventure).toBe('wardens-gate');
    expect(store.session()).toMatchObject({
      kind: 'data',
      value: { revision: 2, campaign: { phase: 'camp' }, encounter: null },
    });
    await store.enterEncounter('iron-warden');
    expect(store.session()).toMatchObject({
      kind: 'data',
      value: {
        revision: 3,
        campaign: { phase: 'encounter' },
        encounter: { turn: 0 },
      },
    });
  });

  it('routes loadout commands and preserves typed atomic rejection', async () => {
    const calls: string[] = [];
    const store = new SessionStore(
      transport({
        equipItem: async (request) => {
          calls.push(`equip:${request.itemId}:${request.slotId}:${request.expectedRevision}`);
          return { ok: true, value: { ...snapshot, revision: 2 } };
        },
        transferItem: async (request) => {
          calls.push(
            `transfer:${request.itemId}:${request.fromOwnerId}:${request.toOwnerId}:${request.expectedRevision}`,
          );
          return {
            ok: false,
            error: {
              kind: 'capacity',
              message: 'inventory is full',
              retryable: false,
            },
          };
        },
      }),
    );
    await store.load();
    await store.equipItem(202, 'body');
    expect(store.session()).toMatchObject({
      kind: 'data',
      value: { revision: 2 },
    });
    await store.transferItem(204, 103, 101);
    expect(store.session()).toMatchObject({
      kind: 'data',
      value: { revision: 2 },
    });
    expect(store.commandError()).toEqual({
      kind: 'capacity',
      message: 'inventory is full',
      retryable: false,
    });
    expect(calls).toEqual(['equip:202:body:1', 'transfer:204:103:101:2']);
  });

  it('projects the authoritative session and preserves typed command rejection', async () => {
    const store = new SessionStore(
      transport({
        beginOppositionTurn: async () => ({
          ok: false,
          error: {
            kind: 'stale',
            message: 'revision changed',
            retryable: true,
          },
        }),
      }),
    );
    await store.load();
    expect(store.session()).toMatchObject({
      kind: 'data',
      value: { revision: 1, engineRevisionShort: 'fb608e323a8b' },
    });
    await store.beginOppositionTurn();
    expect(store.session()).toMatchObject({
      kind: 'data',
      value: { revision: 1 },
    });
    expect(store.commandError()).toEqual({
      kind: 'stale',
      message: 'revision changed',
      retryable: true,
    });
  });

  it('ignores a delayed response after a newer load has published', async () => {
    let resolveFirst: ((result: Result<GameSnapshotDto>) => void) | undefined;
    const first = new Promise<Result<GameSnapshotDto>>((resolve) => {
      resolveFirst = resolve;
    });
    let calls = 0;
    const store = new SessionStore(
      transport({
        loadSession: async () => {
          calls += 1;
          return calls === 1
            ? first
            : { ok: true, value: { ...snapshot, revision: 2, saved: true } };
        },
      }),
    );

    const oldLoad = store.load();
    await store.load();
    resolveFirst?.({ ok: true, value: { ...snapshot, revision: 1 } });
    await oldLoad;

    expect(store.session()).toMatchObject({
      kind: 'data',
      value: { revision: 2, saved: true },
    });
  });
});
