import { describe, expect, it } from "vitest";
import {
  D20_PROTOCOL_LIMITS,
  decodeGameSnapshot,
  decodeRuntimeReadout,
  decodeSaveStatus,
} from "./index";

const validReadout = {
  engineRevision: "fb608e323a8b44a55195f5720101224ff37fd5db",
  entityCount: 0,
  product: "Rusty D20",
  status: "ready",
  version: "0.1.0",
};

describe("decodeRuntimeReadout", () => {
  it("accepts the exact Rust-owned shape", () => {
    expect(decodeRuntimeReadout(validReadout)).toEqual({
      ok: true,
      value: validReadout,
    });
  });

  it("rejects unknown fields and unsafe counts", () => {
    expect(
      decodeRuntimeReadout({ ...validReadout, semanticStatus: "pretend" }),
    ).toMatchObject({
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

describe("decodeGameSnapshot", () => {
  const empty = {
    product: "Rusty D20",
    version: "0.1.0",
    engineRevision: "fb608e323a8b44a55195f5720101224ff37fd5db",
    rulesetFingerprint: "rules",
    revision: 0,
    saved: false,
    availableAdventures: [
      {
        id: "wardens-gate",
        title: "The Warden's Gate",
        summary: "Mara Venn prepares at the Warden's Gate camp.",
        details: ["Steel path"],
      },
      {
        id: "embers-wake",
        title: "Ember's Wake",
        summary: "Sera Vale prepares beside the ember reliquary.",
        details: ["Ember path"],
      },
    ],
    campaign: null,
    exploration: null,
    encounter: null,
  };

  it("accepts the exact empty-session shape", () => {
    expect(decodeGameSnapshot(empty)).toEqual({ ok: true, value: empty });
  });

  it("rejects unknown fields and unsafe revisions", () => {
    expect(decodeGameSnapshot({ ...empty, liveRules: [] })).toMatchObject({
      ok: false,
    });
    expect(
      decodeGameSnapshot({ ...empty, revision: Number.MAX_SAFE_INTEGER + 1 }),
    ).toMatchObject({
      ok: false,
    });
    expect(
      decodeGameSnapshot({ ...empty, availableAdventures: [] }),
    ).toMatchObject({
      ok: false,
    });
  });

  it("uses Rust-owned exact adventure projection limits", () => {
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
        availableAdventures: [
          ...exactChoices,
          { ...exactDetails, id: "one-over" },
        ],
      }),
    ).toMatchObject({ ok: false });
    expect(
      decodeGameSnapshot({
        ...empty,
        availableAdventures: [
          {
            ...exactDetails,
            details: [...exactDetails.details, "One over"],
          },
        ],
      }),
    ).toMatchObject({ ok: false });
  });

  it("strictly validates party, encounter, and phase ownership", () => {
    const hero = {
      id: 101,
      name: "Mara Venn",
      title: "Steel Adept",
      level: 1,
      healthCurrent: 24,
      healthMaximum: 24,
      resources: [],
      effects: [],
    };
    const loadout = {
      ownerId: 101,
      stashOwnerId: 103,
      inventorySlots: [
        {
          entityId: 202,
          definitionId: "chain-armor",
          name: "Chain Armor",
          icon: "shield",
          rarity: "uncommon",
          quantity: 1,
          equipmentSlotId: "body",
          equippedSlotId: "body",
        },
        null,
      ],
      equipmentSlots: [
        {
          id: "body",
          label: "Body",
          equipped: {
            entityId: 202,
            definitionId: "chain-armor",
            name: "Chain Armor",
            icon: "shield",
            rarity: "uncommon",
            quantity: 1,
            equipmentSlotId: "body",
            equippedSlotId: "body",
          },
        },
      ],
      stashItems: [],
      capacity: { metric: "carried-items", used: 1, maximum: 2 },
      defenses: [
        {
          id: "armor",
          label: "Armor",
          value: 16,
          sources: ["Equipped item 202: +4 defense (applied)"],
        },
        {
          id: "resolve",
          label: "Resolve",
          value: 11,
          sources: [],
        },
      ],
    };
    const campaign = {
      id: "wardens-gate",
      title: "The Warden's Gate",
      phase: "camp",
      party: [{ character: hero, loadout }],
      activeEncounterId: null,
      latestOutcome: null,
      completedEncounters: [],
      completion: null,
      availableEncounters: [
        {
          id: "iron-warden",
          title: "The Iron Warden",
          summary: "Challenge the sentinel.",
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
          party: [
            {
              character: hero,
              loadout: {
                ...loadout,
                capacity: { ...loadout.capacity, used: 2 },
              },
            },
          ],
        },
      }),
    ).toMatchObject({ ok: false });
    expect(
      decodeGameSnapshot({
        ...empty,
        campaign: {
          ...campaign,
          party: [
            {
              character: hero,
              loadout: {
                ...loadout,
                equipmentSlots: [
                  { ...loadout.equipmentSlots[0], id: "off-hand" },
                ],
              },
            },
          ],
        },
      }),
    ).toMatchObject({ ok: false });

    const occludedExploration = {
      dungeonTitle: "Warden's Gate Pass",
      wallStyle: "mountain-fortress",
      width: 11,
      height: 7,
      x: 1,
      y: 1,
      facing: "east",
      canStepForward: false,
      canStepBackward: false,
      view: [
        {
          depth: 0,
          frontBlocked: true,
          leftBlocked: false,
          rightBlocked: false,
        },
        { depth: 1, frontBlocked: true, leftBlocked: true, rightBlocked: true },
        { depth: 2, frontBlocked: true, leftBlocked: true, rightBlocked: true },
      ],
      discoveredCells: [{ x: 1, y: 1 }],
      landmark: null,
      doorAhead: null,
      treasure: null,
      checkpoint: {
        id: "gate-camp",
        title: "Pass camp",
        text: "The company can return safely to camp.",
        active: true,
      },
    };
    expect(
      decodeGameSnapshot({
        ...empty,
        campaign: { ...campaign, phase: "exploration" },
        exploration: occludedExploration,
      }),
    ).toMatchObject({ ok: true });
    expect(
      decodeGameSnapshot({
        ...empty,
        campaign: { ...campaign, phase: "exploration" },
        exploration: {
          ...occludedExploration,
          doorAhead: {
            id: "inner-sigil-gate",
            title: "Inner sigil gate",
            text: "The iron leaves bar the descent.",
            opened: true,
            locked: true,
          },
        },
      }),
    ).toMatchObject({ ok: false });
    for (const hiddenTopology of [
      [
        occludedExploration.view[0],
        {
          depth: 1,
          frontBlocked: true,
          leftBlocked: true,
          rightBlocked: false,
        },
        occludedExploration.view[2],
      ],
      [
        occludedExploration.view[0],
        occludedExploration.view[1],
        {
          depth: 2,
          frontBlocked: true,
          leftBlocked: false,
          rightBlocked: true,
        },
      ],
    ]) {
      expect(
        decodeGameSnapshot({
          ...empty,
          campaign: { ...campaign, phase: "exploration" },
          exploration: { ...occludedExploration, view: hiddenTopology },
        }),
      ).toMatchObject({ ok: false });
    }

    const target = {
      ...hero,
      id: 102,
      name: "Iron Warden",
      title: "Armored Sentinel",
    };
    const participants = [
      {
        character: hero,
        faction: "party",
        initiative: 18,
        defeated: false,
        x: 1,
        y: 1,
      },
      {
        character: target,
        faction: "opposition",
        initiative: 14,
        defeated: false,
        x: 2,
        y: 1,
      },
    ];
    const action = {
      id: "longsword-strike",
      label: "Longsword Strike",
      ability: "Might",
      defense: "Armor",
      damage: "1d8+2 Impact",
      activation: ["1 Standard Action"],
      target: "1 Hostile Participant · line of effect Required",
      range: 1,
      implement: "Training Blade",
      tags: ["Attack", "Melee", "Weapon"],
      effect: "Bleeding",
      forcedMovement: 1,
    };
    const board = {
      width: 5,
      height: 5,
      rows: ["#####", "#...#", "#...#", "#...#", "#####"],
      legalMoves: [
        {
          x: 1,
          y: 2,
          cost: 1,
          route: [
            { x: 1, y: 1 },
            { x: 1, y: 2 },
          ],
        },
      ],
    };
    const encounterWithAction = {
      round: 0,
      nextRoll: 0,
      currentActorId: 101,
      board,
      participants,
      actions: [action],
      legalTargets: [
        {
          actionId: "longsword-strike",
          targetIds: [102],
        },
      ],
      pendingAction: null,
      log: [],
    };
    const activeCampaign = {
      ...campaign,
      phase: "encounter",
      activeEncounterId: "iron-warden",
    };
    expect(
      decodeGameSnapshot({
        ...empty,
        campaign: activeCampaign,
        encounter: encounterWithAction,
      }),
    ).toMatchObject({ ok: true });
    expect(
      decodeGameSnapshot({
        ...empty,
        campaign: activeCampaign,
        encounter: {
          ...encounterWithAction,
          board: { ...board, hiddenTopology: "leaked" },
        },
      }),
    ).toMatchObject({ ok: false });
    expect(
      decodeGameSnapshot({
        ...empty,
        campaign: activeCampaign,
        encounter: {
          ...encounterWithAction,
          board: {
            ...board,
            legalMoves: [
              {
                x: 2,
                y: 2,
                cost: 2,
                route: [
                  { x: 1, y: 1 },
                  { x: 2, y: 1 },
                  { x: 2, y: 2 },
                ],
              },
            ],
          },
        },
      }),
    ).toMatchObject({ ok: false });
    expect(
      decodeGameSnapshot({
        ...empty,
        campaign: activeCampaign,
        encounter: {
          ...encounterWithAction,
          board: {
            ...board,
            legalMoves: [
              {
                ...board.legalMoves[0],
                route: [
                  { x: 1, y: 1 },
                  { x: 0, y: 1 },
                ],
                x: 0,
                y: 1,
              },
            ],
          },
        },
      }),
    ).toMatchObject({ ok: false });
    expect(
      decodeGameSnapshot({
        ...empty,
        campaign: { ...activeCampaign, activeEncounterId: null },
        encounter: encounterWithAction,
      }),
    ).toMatchObject({ ok: false });
    expect(
      decodeGameSnapshot({
        ...empty,
        campaign: activeCampaign,
        encounter: {
          ...encounterWithAction,
          legalTargets: [],
        },
      }),
    ).toMatchObject({ ok: false });
    expect(
      decodeGameSnapshot({
        ...empty,
        campaign: activeCampaign,
        encounter: {
          ...encounterWithAction,
          actions: [
            {
              ...action,
              activation: ["one", "two", "three", "four", "one-over"],
            },
          ],
        },
      }),
    ).toMatchObject({ ok: false });
    const missingBinding = Object.fromEntries(
      Object.entries(action).filter(([key]) => key !== "implement"),
    );
    expect(
      decodeGameSnapshot({
        ...empty,
        campaign: activeCampaign,
        encounter: {
          ...encounterWithAction,
          actions: [missingBinding],
        },
      }),
    ).toMatchObject({ ok: false });

    const victory = {
      kind: "victory",
      encounterId: "iron-warden",
      title: "The Iron Warden defeated",
      summary: "Mara prevailed.",
      rewardItemId: 201,
      reward: "Warden chain armor",
    };
    const outcomeCampaign = {
      ...campaign,
      phase: "outcome",
      activeEncounterId: "iron-warden",
      latestOutcome: victory,
      completedEncounters: [
        {
          encounterId: "iron-warden",
          title: "The Iron Warden",
          outcome: "victory",
        },
      ],
    };
    const outcomeEncounter = {
      ...encounterWithAction,
      round: 4,
      nextRoll: 8,
      currentActorId: null,
      participants: [
        participants[0],
        {
          ...participants[1],
          character: { ...target, healthCurrent: 0 },
          defeated: true,
        },
      ],
      actions: [],
      legalTargets: [],
      board: { ...board, legalMoves: [] },
    };
    expect(
      decodeGameSnapshot({
        ...empty,
        campaign: outcomeCampaign,
        encounter: outcomeEncounter,
      }),
    ).toMatchObject({ ok: true });
    const terminalCampaign = {
      ...outcomeCampaign,
      phase: "adventure-complete",
      activeEncounterId: null,
      availableEncounters: [],
      completion: {
        kind: "victory",
        source: "Warden's Gate",
        title: "The mountain pass is secure",
        text: "The company carries the Warden sigil into daylight.",
        details: ["The terminal expedition state is durable."],
      },
    };
    expect(
      decodeGameSnapshot({
        ...empty,
        campaign: terminalCampaign,
      }),
    ).toMatchObject({ ok: true });
    expect(
      decodeGameSnapshot({
        ...empty,
        campaign: {
          ...terminalCampaign,
          completion: { ...terminalCampaign.completion, hidden: "leak" },
        },
      }),
    ).toMatchObject({ ok: false });
    expect(
      decodeGameSnapshot({
        ...empty,
        campaign: {
          ...terminalCampaign,
          completion: { ...terminalCampaign.completion, kind: "defeat" },
        },
      }),
    ).toMatchObject({ ok: false });
    expect(
      decodeGameSnapshot({
        ...empty,
        campaign: outcomeCampaign,
        encounter: { ...outcomeEncounter, currentActorId: 101 },
      }),
    ).toMatchObject({ ok: false });
  });
});

describe("decodeSaveStatus", () => {
  it("strictly distinguishes ready, empty, and recovery identities", () => {
    expect(
      decodeSaveStatus({
        saveIdentity: "/tmp/campaign.json",
        state: "ready",
        campaignId: "wardens-gate",
        campaignTitle: "The Warden's Gate",
        revision: 9,
        persistenceError: null,
      }),
    ).toMatchObject({ ok: true });
    expect(
      decodeSaveStatus({
        saveIdentity: "/tmp/campaign.json",
        state: "recovery-required",
        campaignId: null,
        campaignTitle: null,
        revision: null,
        persistenceError: "save is malformed",
      }),
    ).toMatchObject({ ok: true });
    expect(
      decodeSaveStatus({
        saveIdentity: "/tmp/campaign.json",
        state: "recovery-required",
        campaignId: "forged",
        campaignTitle: null,
        revision: null,
        persistenceError: "save is malformed",
      }),
    ).toMatchObject({ ok: false });
  });
});
