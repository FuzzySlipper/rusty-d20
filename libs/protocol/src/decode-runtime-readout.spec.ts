import { describe, expect, it } from 'vitest';
import {
  D20_PROTOCOL_LIMITS,
  decodeGameSnapshot,
  decodeRuntimeReadout,
  decodeSaveStatus,
} from './index';

const validReadout = {
  engineRevision: 'fb608e323a8b44a55195f5720101224ff37fd5db',
  entityCount: 0,
  product: 'Rusty D20',
  status: 'ready',
  version: '0.1.0',
};

describe('decodeRuntimeReadout', () => {
  it('accepts the exact Rust-owned shape', () => {
    expect(decodeRuntimeReadout(validReadout)).toEqual({
      ok: true,
      value: validReadout,
    });
  });

  it('rejects unknown fields and unsafe counts', () => {
    expect(decodeRuntimeReadout({ ...validReadout, semanticStatus: 'pretend' })).toMatchObject({
      ok: false,
    });
    expect(
      decodeRuntimeReadout({
        ...validReadout,
        entityCount: Number.MAX_SAFE_INTEGER + 1,
      }),
    ).toMatchObject({ ok: false });
  });
});

describe('decodeGameSnapshot', () => {
  const empty = {
    product: 'Rusty D20',
    version: '0.1.0',
    engineRevision: 'fb608e323a8b44a55195f5720101224ff37fd5db',
    rulesetFingerprint: 'rules',
    revision: 0,
    saved: false,
    availableAdventures: [
      {
        id: 'wardens-gate',
        title: "The Warden's Gate",
        summary: "Mara Venn prepares at the Warden's Gate camp.",
        details: ['Steel path'],
      },
      {
        id: 'embers-wake',
        title: "Ember's Wake",
        summary: 'Sera Vale prepares beside the ember reliquary.',
        details: ['Ember path'],
      },
    ],
    campaign: null,
    exploration: null,
    encounter: null,
  };

  it('accepts the exact empty-session shape', () => {
    expect(decodeGameSnapshot(empty)).toEqual({ ok: true, value: empty });
  });

  it('rejects unknown fields and unsafe revisions', () => {
    expect(decodeGameSnapshot({ ...empty, liveRules: [] })).toMatchObject({
      ok: false,
    });
    expect(decodeGameSnapshot({ ...empty, revision: Number.MAX_SAFE_INTEGER + 1 })).toMatchObject({
      ok: false,
    });
    expect(decodeGameSnapshot({ ...empty, availableAdventures: [] })).toMatchObject({
      ok: false,
    });
  });

  it('uses Rust-owned exact adventure projection limits', () => {
    const choice = empty.availableAdventures[0];
    const exactDetails = {
      ...choice,
      details: Array.from(
        { length: D20_PROTOCOL_LIMITS.maxAdventureDetails },
        (_, index) => `Detail ${index}`,
      ),
    };
    const exactChoices = Array.from(
      { length: D20_PROTOCOL_LIMITS.maxAvailableAdventures },
      (_, index) => ({
        ...exactDetails,
        id: `adventure-${index}`,
      }),
    );

    expect(
      decodeGameSnapshot({ ...empty, availableAdventures: exactChoices }),
    ).toMatchObject({ ok: true });
    expect(
      decodeGameSnapshot({
        ...empty,
        availableAdventures: [...exactChoices, { ...exactDetails, id: 'one-over' }],
      }),
    ).toMatchObject({ ok: false });
    expect(
      decodeGameSnapshot({
        ...empty,
        availableAdventures: [
          {
            ...exactDetails,
            details: [...exactDetails.details, 'One over'],
          },
        ],
      }),
    ).toMatchObject({ ok: false });
  });

  it('strictly validates campaign phase ownership', () => {
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
        inventorySlots: [
          {
            entityId: 202,
            definitionId: 'chain-armor',
            name: 'Chain Armor',
            icon: 'shield',
            rarity: 'uncommon',
            quantity: 1,
            equipmentSlotId: 'body',
            equippedSlotId: 'body',
          },
          null,
        ],
        equipmentSlots: [
          {
            id: 'body',
            label: 'Body',
            equipped: {
              entityId: 202,
              definitionId: 'chain-armor',
              name: 'Chain Armor',
              icon: 'shield',
              rarity: 'uncommon',
              quantity: 1,
              equipmentSlotId: 'body',
              equippedSlotId: 'body',
            },
          },
        ],
        stashItems: [],
        capacity: { metric: 'carried-items', used: 1, maximum: 2 },
        defenses: [
          {
            id: 'armor',
            label: 'Armor',
            value: 16,
            sources: ['Equipped item 202: +4 defense (applied)'],
          },
          {
            id: 'resolve',
            label: 'Resolve',
            value: 11,
            sources: [],
          },
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
    };
    expect(decodeGameSnapshot({ ...empty, campaign })).toMatchObject({
      ok: true,
    });
    expect(
      decodeGameSnapshot({
        ...empty,
        campaign: {
          ...campaign,
          loadout: {
            ...campaign.loadout,
            capacity: { ...campaign.loadout.capacity, used: 2 },
          },
        },
      }),
    ).toMatchObject({ ok: false });
    expect(
      decodeGameSnapshot({
        ...empty,
        campaign: {
          ...campaign,
          loadout: {
            ...campaign.loadout,
            equipmentSlots: [
              {
                ...campaign.loadout.equipmentSlots[0],
                id: 'off-hand',
              },
            ],
          },
        },
      }),
    ).toMatchObject({ ok: false });
    expect(
      decodeGameSnapshot({
        ...empty,
        campaign: { ...campaign, phase: 'encounter', activeEncounterId: null },
      }),
    ).toMatchObject({ ok: false });

    const target = {
      ...hero,
      id: 102,
      name: 'Iron Warden',
      title: 'Armored Sentinel',
    };
    expect(
      decodeGameSnapshot({
        ...empty,
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
          characters: [hero, target],
          actions: [],
          pendingAction: null,
          log: [],
        },
      }),
    ).toMatchObject({ ok: true });

    const victory = {
      kind: 'victory',
      encounterId: 'iron-warden',
      title: 'The Iron Warden defeated',
      summary: 'Mara prevailed.',
      rewardItemId: 201,
      reward: 'Warden chain armor',
    };
    expect(
      decodeGameSnapshot({
        ...empty,
        campaign: {
          ...campaign,
          phase: 'outcome',
          activeEncounterId: 'iron-warden',
          latestOutcome: victory,
          completedEncounters: [
            {
              encounterId: 'iron-warden',
              title: 'The Iron Warden',
              outcome: 'victory',
            },
          ],
        },
        encounter: {
          turn: 4,
          nextRoll: 8,
          playerId: 101,
          turnOwner: null,
          characters: [hero, target],
          actions: [],
          pendingAction: null,
          log: [],
        },
      }),
    ).toMatchObject({ ok: true });
    expect(
      decodeGameSnapshot({
        ...empty,
        campaign: {
          ...campaign,
          phase: 'outcome',
          activeEncounterId: 'iron-warden',
          latestOutcome: victory,
          completedEncounters: [
            {
              encounterId: 'iron-warden',
              title: 'The Iron Warden',
              outcome: 'victory',
            },
          ],
        },
        encounter: {
          turn: 4,
          nextRoll: 8,
          playerId: 101,
          turnOwner: 'player',
          characters: [hero, target],
          actions: [],
          pendingAction: null,
          log: [],
        },
      }),
    ).toMatchObject({ ok: false });
  });
});

describe('decodeSaveStatus', () => {
  it('strictly distinguishes ready, empty, and recovery identities', () => {
    expect(
      decodeSaveStatus({
        saveIdentity: '/tmp/campaign.json',
        state: 'ready',
        campaignId: 'wardens-gate',
        campaignTitle: "The Warden's Gate",
        revision: 9,
        persistenceError: null,
      }),
    ).toMatchObject({ ok: true });
    expect(
      decodeSaveStatus({
        saveIdentity: '/tmp/campaign.json',
        state: 'recovery-required',
        campaignId: null,
        campaignTitle: null,
        revision: null,
        persistenceError: 'save is malformed',
      }),
    ).toMatchObject({ ok: true });
    expect(
      decodeSaveStatus({
        saveIdentity: '/tmp/campaign.json',
        state: 'recovery-required',
        campaignId: 'forged',
        campaignTitle: null,
        revision: null,
        persistenceError: 'save is malformed',
      }),
    ).toMatchObject({ ok: false });
  });
});
