import { describe, expect, it } from 'vitest';
import type {
  GameSnapshotDto,
  Result,
  RuntimeReadoutDto,
  SaveStatusDto,
} from '@rusty-d20/protocol';
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
  exploration: null,
  encounter: null,
};

const saveStatus: SaveStatusDto = {
  saveIdentity: '/tmp/rusty-d20.json',
  state: 'empty',
  campaignId: null,
  campaignTitle: null,
  revision: 1,
  persistenceError: null,
};

function transport(overrides: Partial<RustyD20Transport> = {}): RustyD20Transport {
  const sessionResult: Result<GameSnapshotDto> = { ok: true, value: snapshot };
  return {
    loadReadout: async () => ({ ok: true, value: readout }),
    loadSession: async () => sessionResult,
    loadSaveStatus: async () => ({ ok: true, value: saveStatus }),
    resetSession: async () => sessionResult,
    newAdventure: async () => sessionResult,
    beginExploration: async () => sessionResult,
    explorationCommand: async () => sessionResult,
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
      completedEncounters: [],
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
        beginExploration: async () => ({ ok: true, value: encounter }),
      }),
    );

    await store.load();
    await store.newAdventure('wardens-gate');
    expect(selectedAdventure).toBe('wardens-gate');
    expect(store.session()).toMatchObject({
      kind: 'data',
      value: { revision: 2, campaign: { phase: 'camp' }, encounter: null },
    });
    await store.beginExploration();
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
    let markFirstStarted: (() => void) | undefined;
    const firstStarted = new Promise<void>((resolve) => {
      markFirstStarted = resolve;
    });
    let calls = 0;
    const store = new SessionStore(
      transport({
        loadSession: async () => {
          calls += 1;
          if (calls === 1) {
            markFirstStarted?.();
            return first;
          }
          return { ok: true, value: { ...snapshot, revision: 2, saved: true } };
        },
      }),
    );

    const oldLoad = store.load();
    await firstStarted;
    await store.load();
    resolveFirst?.({ ok: true, value: { ...snapshot, revision: 1 } });
    await oldLoad;

    expect(store.session()).toMatchObject({
      kind: 'data',
      value: { revision: 2, saved: true },
    });
  });

  it('recovers a malformed save through the explicit identity-guarded reset', async () => {
    const recoveryStatus: SaveStatusDto = {
      saveIdentity: '/tmp/malformed.json',
      state: 'recovery-required',
      campaignId: null,
      campaignTitle: null,
      revision: null,
      persistenceError: 'save is malformed',
    };
    let statusCalls = 0;
    let resetRequest: Parameters<RustyD20Transport['resetSession']>[0] | undefined;
    const store = new SessionStore(
      transport({
        loadSession: async () => ({
          ok: false,
          error: { kind: 'persistence', message: 'save is malformed', retryable: false },
        }),
        loadSaveStatus: async () => {
          statusCalls += 1;
          return {
            ok: true,
            value: statusCalls === 1 ? recoveryStatus : saveStatus,
          };
        },
        resetSession: async (request) => {
          resetRequest = request;
          return { ok: true, value: { ...snapshot, revision: 0 } };
        },
      }),
    );

    await store.load();
    expect(store.session()).toMatchObject({ kind: 'error' });
    expect(store.saveStatus()).toEqual({ kind: 'data', value: recoveryStatus });
    await store.resetSession();
    expect(resetRequest).toEqual({
      expectedSaveIdentity: '/tmp/malformed.json',
      expectedRevision: null,
      expectedAdventureId: null,
    });
    expect(store.session()).toMatchObject({
      kind: 'data',
      value: { revision: 0, campaign: null },
    });
    expect(store.saveStatus()).toEqual({ kind: 'data', value: saveStatus });
  });
});
