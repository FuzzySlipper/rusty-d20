import { describe, expect, it } from 'vitest';
import type { HttpPort, HttpResponse } from '@rusty-d20/platform';
import { createHttpRustyD20Transport } from './index';

const readout = {
  engineRevision: 'fb608e323a8b44a55195f5720101224ff37fd5db',
  entityCount: 0,
  product: 'Rusty D20',
  status: 'ready',
  version: '0.1.0',
};

const snapshot = {
  product: 'Rusty D20',
  version: '0.1.0',
  engineRevision: 'fb608e323a8b44a55195f5720101224ff37fd5db',
  rulesetFingerprint: 'rules',
  revision: 0,
  saved: false,
  encounter: null,
};

function http(response: () => Promise<HttpResponse>): HttpPort {
  return { getJson: response, postJson: async () => response() };
}

describe('createHttpRustyD20Transport', () => {
  it('decodes successful Rust readout and session responses', async () => {
    const port: HttpPort = {
      getJson: async (path) => ({ status: 200, body: path.endsWith('readout') ? readout : snapshot }),
      postJson: async () => ({ status: 200, body: snapshot }),
    };
    const transport = createHttpRustyD20Transport(port);
    await expect(transport.loadReadout()).resolves.toEqual({ ok: true, value: readout });
    await expect(transport.loadSession()).resolves.toEqual({ ok: true, value: snapshot });
    await expect(transport.startEncounter(0)).resolves.toEqual({ ok: true, value: snapshot });
  });

  it('preserves typed stale errors and classifies network and invalid-body failures', async () => {
    const stale = createHttpRustyD20Transport(
      http(async () => ({
        status: 409,
        body: { kind: 'stale', message: 'revision changed', retryable: true },
      })),
    );
    const network = createHttpRustyD20Transport(
      http(async () => Promise.reject(new Error('connection refused'))),
    );
    const invalid = createHttpRustyD20Transport(
      http(async () => ({ status: 200, body: { product: 'Rusty D20' } })),
    );

    await expect(stale.advanceTurn(1)).resolves.toEqual({
      ok: false,
      error: { kind: 'stale', message: 'revision changed', retryable: true },
    });
    await expect(network.loadSession()).resolves.toMatchObject({
      ok: false,
      error: { kind: 'network', retryable: true },
    });
    await expect(invalid.loadSession()).resolves.toMatchObject({
      ok: false,
      error: { kind: 'unknown', retryable: false },
    });
  });
});
