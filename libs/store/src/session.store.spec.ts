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
    previewAction: async () => sessionResult,
    applyReaction: async () => sessionResult,
    applyAction: async () => sessionResult,
    advanceTurn: async () => sessionResult,
    save: async () => sessionResult,
    ...overrides,
  };
}

describe('SessionStore', () => {
  it('publishes Rust-owned camp and encounter phases through named commands', async () => {
    const hero = {
      id: 101,
      name: 'Mara Venn',
      title: 'Steel Adept',
      level: 1,
      healthCurrent: 100,
      healthMaximum: 100,
      resources: [],
      effects: [],
    };
    const campaign = {
      id: 'wardens-gate',
      title: "The Warden's Gate",
      phase: 'camp',
      hero,
      activeEncounterId: null,
      availableEncounters: [
        { id: 'iron-warden', title: 'The Iron Warden', summary: 'Challenge the sentinel.' },
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
        characters: [
          hero,
          { ...hero, id: 102, name: 'Iron Warden', title: 'Armored Sentinel' },
        ],
        actions: [],
        pendingAction: null,
        log: [],
      },
    };
    const store = new SessionStore(
      transport({
        newAdventure: async () => ({ ok: true, value: camp }),
        enterEncounter: async () => ({ ok: true, value: encounter }),
      }),
    );

    await store.load();
    await store.newAdventure();
    expect(store.session()).toMatchObject({
      kind: 'data',
      value: { revision: 2, campaign: { phase: 'camp' }, encounter: null },
    });
    await store.enterEncounter('iron-warden');
    expect(store.session()).toMatchObject({
      kind: 'data',
      value: { revision: 3, campaign: { phase: 'encounter' }, encounter: { turn: 0 } },
    });
  });

  it('projects the authoritative session and preserves typed command rejection', async () => {
    const store = new SessionStore(
      transport({
        advanceTurn: async () => ({
          ok: false,
          error: { kind: 'stale', message: 'revision changed', retryable: true },
        }),
      }),
    );
    await store.load();
    expect(store.session()).toMatchObject({
      kind: 'data',
      value: { revision: 1, engineRevisionShort: 'fb608e323a8b' },
    });
    await store.advanceTurn();
    expect(store.session()).toMatchObject({ kind: 'data', value: { revision: 1 } });
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
