import { describe, expect, it } from 'vitest';
import type { RustyD20Transport } from '@rusty-d20/transport';
import { SessionStore } from './index';

const readout = {
  engineRevision: 'fb608e323a8b44a55195f5720101224ff37fd5db',
  entityCount: 1,
  product: 'Rusty D20',
  status: 'ready' as const,
  version: '0.1.0',
};

describe('SessionStore', () => {
  it('projects a real transport result into observable readout state', async () => {
    const transport: RustyD20Transport = { loadReadout: async () => ({ ok: true, value: readout }) };
    const store = new SessionStore(transport);
    await store.load();
    expect(store.readout()).toMatchObject({
      kind: 'data',
      value: { product: 'Rusty D20', entityCount: 1, engineRevisionShort: 'fb608e323a8b' },
    });
  });

  it('preserves classified transport failures for presentation', async () => {
    const transport: RustyD20Transport = {
      loadReadout: async () => ({
        ok: false,
        error: { kind: 'network', message: 'offline', retryable: true },
      }),
    };
    const store = new SessionStore(transport);
    await store.load();
    expect(store.readout()).toEqual({
      kind: 'error',
      error: { kind: 'network', message: 'offline', retryable: true },
    });
  });
});
