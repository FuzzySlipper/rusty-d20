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
  encounter: null,
};

function transport(overrides: Partial<RustyD20Transport> = {}): RustyD20Transport {
  const sessionResult: Result<GameSnapshotDto> = { ok: true, value: snapshot };
  return {
    loadReadout: async () => ({ ok: true, value: readout }),
    loadSession: async () => sessionResult,
    startEncounter: async () => sessionResult,
    previewAction: async () => sessionResult,
    applyReaction: async () => sessionResult,
    applyAction: async () => sessionResult,
    advanceTurn: async () => sessionResult,
    save: async () => sessionResult,
    ...overrides,
  };
}

describe('SessionStore', () => {
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
