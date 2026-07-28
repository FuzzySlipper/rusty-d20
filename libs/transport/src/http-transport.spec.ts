import { describe, expect, it } from 'vitest';
import type { HttpPort } from '@rusty-d20/platform';
import { createHttpRustyD20Transport } from './index';

const readout = {
  engineRevision: 'fb608e323a8b44a55195f5720101224ff37fd5db',
  entityCount: 1,
  product: 'Rusty D20',
  status: 'ready',
  version: '0.1.0',
};

describe('createHttpRustyD20Transport', () => {
  it('decodes a successful Rust readout', async () => {
    const http: HttpPort = { getJson: async () => ({ status: 200, body: readout }) };
    await expect(createHttpRustyD20Transport(http).loadReadout()).resolves.toEqual({ ok: true, value: readout });
  });

  it('classifies network, not-found, and invalid-body failures', async () => {
    const network: HttpPort = { getJson: async () => Promise.reject(new Error('connection refused')) };
    const missing: HttpPort = { getJson: async () => ({ status: 404, body: {} }) };
    const invalid: HttpPort = { getJson: async () => ({ status: 200, body: { status: 'ready' } }) };

    await expect(createHttpRustyD20Transport(network).loadReadout()).resolves.toMatchObject({
      ok: false,
      error: { kind: 'network', retryable: true },
    });
    await expect(createHttpRustyD20Transport(missing).loadReadout()).resolves.toMatchObject({
      ok: false,
      error: { kind: 'not-found', retryable: false },
    });
    await expect(createHttpRustyD20Transport(invalid).loadReadout()).resolves.toMatchObject({
      ok: false,
      error: { kind: 'unknown', retryable: false },
    });
  });
});
