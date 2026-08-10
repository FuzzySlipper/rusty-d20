import { describe, expect, it } from 'vitest';
import type { HttpPort, HttpResponse } from '@rusty-d20/platform';
import { createHttpRustyD20Transport } from './index';

const readout = {
  entityCount: 0,
  product: 'Rusty D20',
  status: 'ready',
  version: '0.1.0',
};

const snapshot = {
  product: 'Rusty D20',
  version: '0.1.0',
  rulesetFingerprint: 'rules',
  revision: 0,
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

const saveStatus = {
  saveIdentity: '/tmp/rusty-d20.json',
  state: 'empty',
  campaignId: null,
  campaignTitle: null,
  revision: 0,
  persistenceError: null,
};

function http(response: () => Promise<HttpResponse>): HttpPort {
  return { getJson: response, postJson: async () => response() };
}

describe('createHttpRustyD20Transport', () => {
  it('decodes successful Rust readout and session responses', async () => {
    const posts: Array<{ path: string; body: unknown }> = [];
    const port: HttpPort = {
      getJson: async (path) => ({
        status: 200,
        body: path.endsWith('readout')
          ? readout
          : path.endsWith('save-status')
            ? saveStatus
            : snapshot,
      }),
      postJson: async (path, body) => {
        posts.push({ path, body });
        return { status: 200, body: snapshot };
      },
    };
    const transport = createHttpRustyD20Transport(port);
    await expect(transport.loadReadout()).resolves.toEqual({
      ok: true,
      value: readout,
    });
    await expect(transport.loadSession()).resolves.toEqual({
      ok: true,
      value: snapshot,
    });
    await expect(transport.loadSaveStatus()).resolves.toEqual({
      ok: true,
      value: saveStatus,
    });
    await expect(
      transport.resetSession({
        expectedSaveIdentity: '/tmp/rusty-d20.json',
        expectedRevision: 0,
        expectedAdventureId: null,
      }),
    ).resolves.toEqual({ ok: true, value: snapshot });
    expect(posts[0]).toEqual({
      path: '/api/v1/session/reset',
      body: {
        expectedSaveIdentity: '/tmp/rusty-d20.json',
        expectedRevision: 0,
        expectedAdventureId: null,
      },
    });
    await expect(
      transport.newAdventure({
        expectedRevision: 0,
        adventureId: 'embers-wake',
      }),
    ).resolves.toEqual({
      ok: true,
      value: snapshot,
    });
    expect(posts[1]).toEqual({
      path: '/api/v1/session/new',
      body: {
        expectedRevision: 0,
        adventureId: 'embers-wake',
      },
    });
    await expect(transport.beginExploration(0)).resolves.toEqual({
      ok: true,
      value: snapshot,
    });
    await expect(
      transport.explorationCommand({
        expectedRevision: 0,
        command: 'step-forward',
      }),
    ).resolves.toEqual({ ok: true, value: snapshot });
    await expect(
      transport.equipItem({ expectedRevision: 0, itemId: 202, slotId: 'body' }),
    ).resolves.toEqual({ ok: true, value: snapshot });
    await expect(
      transport.unequipItem({ expectedRevision: 0, itemId: 202 }),
    ).resolves.toEqual({
      ok: true,
      value: snapshot,
    });
    await expect(
      transport.transferItem({
        expectedRevision: 0,
        itemId: 202,
        fromOwnerId: 101,
        toOwnerId: 103,
      }),
    ).resolves.toEqual({ ok: true, value: snapshot });
    await expect(
      transport.moveLoadoutItem({
        expectedRevision: 0,
        itemId: 204,
        fromOwnerId: 103,
        toOwnerId: 101,
        destinationSlotId: 'off-hand',
      }),
    ).resolves.toEqual({ ok: true, value: snapshot });
    expect(posts.at(-1)).toEqual({
      path: '/api/v1/session/loadout/move',
      body: {
        expectedRevision: 0,
        itemId: 204,
        fromOwnerId: 103,
        toOwnerId: 101,
        destinationSlotId: 'off-hand',
      },
    });
    await expect(
      transport.moveActor({
        expectedRevision: 0,
        actorId: 101,
        x: 2,
        y: 3,
      }),
    ).resolves.toEqual({ ok: true, value: snapshot });
    expect(posts.at(-1)).toEqual({
      path: '/api/v1/session/move',
      body: { expectedRevision: 0, actorId: 101, x: 2, y: 3 },
    });
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
    const capacity = createHttpRustyD20Transport(
      http(async () => ({
        status: 422,
        body: {
          kind: 'capacity',
          message: 'inventory is full',
          retryable: false,
        },
      })),
    );

    await expect(stale.endActivation(1)).resolves.toEqual({
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
    await expect(
      capacity.transferItem({
        expectedRevision: 1,
        itemId: 204,
        fromOwnerId: 103,
        toOwnerId: 101,
      }),
    ).resolves.toEqual({
      ok: false,
      error: {
        kind: 'capacity',
        message: 'inventory is full',
        retryable: false,
      },
    });
  });
});
