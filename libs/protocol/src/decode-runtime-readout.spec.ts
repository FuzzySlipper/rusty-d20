import { describe, expect, it } from 'vitest';
import { decodeGameSnapshot, decodeRuntimeReadout } from './index';

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
    campaign: null,
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
  });

  it('strictly validates campaign phase ownership', () => {
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
        armorDefense: 16,
        armorDefenseSources: ['Equipped item 202: +4 defense (applied)'],
      },
      activeEncounterId: null,
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
          characters: [hero, target],
          actions: [],
          pendingAction: null,
          log: [],
        },
      }),
    ).toMatchObject({ ok: true });
  });
});
