import { describe, expect, it } from "vitest";
import type {
  GameSnapshotDto,
  Result,
  RuntimeReadoutDto,
  SaveStatusDto,
} from "@rusty-d20/protocol";
import type { RustyD20Transport } from "@rusty-d20/transport";
import { SessionStore } from "./index";

const SYNTHETIC_ENGINE_REVISION = "1111111111111111111111111111111111111111";

const readout: RuntimeReadoutDto = {
  engineRevision: SYNTHETIC_ENGINE_REVISION,
  entityCount: 0,
  product: "Rusty D20",
  status: "ready",
  version: "0.1.0",
};

const snapshot: GameSnapshotDto = {
  product: "Rusty D20",
  version: "0.1.0",
  engineRevision: SYNTHETIC_ENGINE_REVISION,
  rulesetFingerprint: "rules",
  revision: 1,
  saved: false,
  availableAdventures: [
    {
      id: "wardens-gate",
      title: "The Warden's Gate",
      summary: "Mara Venn prepares at the Warden's Gate camp.",
      details: [],
    },
    {
      id: "embers-wake",
      title: "Ember's Wake",
      summary: "Sera Vale prepares beside the ember reliquary.",
      details: [],
    },
  ],
  campaign: null,
  exploration: null,
  encounter: null,
};

const saveStatus: SaveStatusDto = {
  saveIdentity: "/tmp/rusty-d20.json",
  state: "empty",
  campaignId: null,
  campaignTitle: null,
  revision: 1,
  persistenceError: null,
};

function transport(
  overrides: Partial<RustyD20Transport> = {},
): RustyD20Transport {
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
    moveActor: async () => sessionResult,
    chooseAction: async () => sessionResult,
    applyReaction: async () => sessionResult,
    declineReaction: async () => sessionResult,
    beginOppositionTurn: async () => sessionResult,
    endActivation: async () => sessionResult,
    returnToCamp: async () => sessionResult,
    save: async () => sessionResult,
    ...overrides,
  };
}

describe("SessionStore", () => {
  it("publishes Rust-owned camp and encounter phases through named commands", async () => {
    let selectedAdventure: string | undefined;
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
    const campaign = {
      id: "wardens-gate",
      title: "The Warden's Gate",
      phase: "camp",
      party: [
        {
          character: hero,
          loadout: {
            ownerId: 101,
            stashOwnerId: 103,
            inventorySlots: [],
            equipmentSlots: [],
            stashItems: [],
            capacity: { metric: "carried-items", used: 0, maximum: 0 },
            defenses: [
              { id: "armor", label: "Armor", value: 12, sources: [] },
              { id: "resolve", label: "Resolve", value: 11, sources: [] },
            ],
          },
        },
      ],
      activeEncounterId: null,
      latestOutcome: null,
      completedEncounters: [],
      availableEncounters: [
        {
          id: "iron-warden",
          title: "The Iron Warden",
          summary: "Challenge the sentinel.",
        },
      ],
    } satisfies NonNullable<GameSnapshotDto["campaign"]>;
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
        phase: "encounter",
        activeEncounterId: "iron-warden",
      },
      encounter: {
        round: 0,
        currentActorId: 101,
        board: {
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
        },
        participants: [
          {
            character: hero,
            faction: "party",
            initiative: 18,
            defeated: false,
            x: 1,
            y: 1,
          },
          {
            character: {
              ...hero,
              id: 102,
              name: "Iron Warden",
              title: "Armored Sentinel",
            },
            faction: "opposition",
            initiative: 14,
            defeated: false,
            x: 2,
            y: 1,
          },
        ],
        actions: [],
        legalTargets: [],
        reactionPrompt: null,
        log: [],
      },
    };
    const encounterState = encounter.encounter;
    if (encounterState === null) {
      throw new Error("encounter test fixture is missing");
    }
    const store = new SessionStore(
      transport({
        newAdventure: async (request) => {
          selectedAdventure = request.adventureId;
          return { ok: true, value: camp };
        },
        beginExploration: async () => ({ ok: true, value: encounter }),
        moveActor: async (request) => ({
          ok: true,
          value: {
            ...encounter,
            revision: 4,
            encounter: {
              ...encounterState,
              board: { ...encounterState.board, legalMoves: [] },
              participants: encounterState.participants.map((participant) =>
                participant.character.id === request.actorId
                  ? { ...participant, x: request.x, y: request.y }
                  : participant,
              ),
            },
          },
        }),
      }),
    );

    await store.load();
    await store.newAdventure("wardens-gate");
    expect(selectedAdventure).toBe("wardens-gate");
    expect(store.session()).toMatchObject({
      kind: "data",
      value: { revision: 2, campaign: { phase: "camp" }, encounter: null },
    });
    await store.beginExploration();
    expect(store.session()).toMatchObject({
      kind: "data",
      value: {
        revision: 3,
        campaign: { phase: "encounter" },
        encounter: { round: 0, currentActorId: 101 },
      },
    });
    await store.moveActor(101, 1, 2);
    const moved = store.session();
    expect(moved).toMatchObject({ kind: "data", value: { revision: 4 } });
    expect(
      moved.kind === "data"
        ? moved.value.encounter?.participants.find(
            (participant) => participant.character.id === 101,
          )
        : undefined,
    ).toMatchObject({ character: { id: 101 }, x: 1, y: 2 });
  });

  it("routes one-step actions and reaction decisions without a resolve command", async () => {
    const calls: string[] = [];
    const store = new SessionStore(
      transport({
        chooseAction: async (request) => {
          calls.push(
            `action:${request.actionId}:${request.actorId}:${request.targetId}:${request.expectedRevision}`,
          );
          return { ok: true, value: { ...snapshot, revision: 2 } };
        },
        applyReaction: async (request) => {
          calls.push(
            `react:${request.promptToken}:${request.reactionId}:${request.expectedRevision}`,
          );
          return { ok: true, value: { ...snapshot, revision: 3 } };
        },
        declineReaction: async (request) => {
          calls.push(
            `decline:${request.promptToken}:${request.expectedRevision}`,
          );
          return { ok: true, value: { ...snapshot, revision: 4 } };
        },
      }),
    );
    await store.load();
    await store.chooseAction("longsword-strike", 101, 102);
    await store.applyReaction("prompt-1", "parry");
    await store.declineReaction("prompt-2");
    expect(calls).toEqual([
      "action:longsword-strike:101:102:1",
      "react:prompt-1:parry:2",
      "decline:prompt-2:3",
    ]);
    expect(store.session()).toMatchObject({
      kind: "data",
      value: { revision: 4 },
    });
  });

  it("routes loadout commands and preserves typed atomic rejection", async () => {
    const calls: string[] = [];
    const store = new SessionStore(
      transport({
        equipItem: async (request) => {
          calls.push(
            `equip:${request.itemId}:${request.slotId}:${request.expectedRevision}`,
          );
          return { ok: true, value: { ...snapshot, revision: 2 } };
        },
        transferItem: async (request) => {
          calls.push(
            `transfer:${request.itemId}:${request.fromOwnerId}:${request.toOwnerId}:${request.expectedRevision}`,
          );
          return {
            ok: false,
            error: {
              kind: "capacity",
              message: "inventory is full",
              retryable: false,
            },
          };
        },
      }),
    );
    await store.load();
    await store.equipItem(202, "body");
    expect(store.session()).toMatchObject({
      kind: "data",
      value: { revision: 2 },
    });
    await store.transferItem(204, 103, 101);
    expect(store.session()).toMatchObject({
      kind: "data",
      value: { revision: 2 },
    });
    expect(store.commandError()).toEqual({
      kind: "capacity",
      message: "inventory is full",
      retryable: false,
    });
    expect(calls).toEqual(["equip:202:body:1", "transfer:204:103:101:2"]);
  });

  it("projects the authoritative session and preserves typed command rejection", async () => {
    const store = new SessionStore(
      transport({
        beginOppositionTurn: async () => ({
          ok: false,
          error: {
            kind: "stale",
            message: "revision changed",
            retryable: true,
          },
        }),
      }),
    );
    await store.load();
    expect(store.session()).toMatchObject({
      kind: "data",
      value: { revision: 1, engineRevisionShort: "111111111111" },
    });
    await store.beginOppositionTurn();
    expect(store.session()).toMatchObject({
      kind: "data",
      value: { revision: 1 },
    });
    expect(store.commandError()).toEqual({
      kind: "stale",
      message: "revision changed",
      retryable: true,
    });
  });

  it("ignores a delayed response after a newer load has published", async () => {
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
      kind: "data",
      value: { revision: 2, saved: true },
    });
  });

  it("recovers a malformed save through the explicit identity-guarded reset", async () => {
    const recoveryStatus: SaveStatusDto = {
      saveIdentity: "/tmp/malformed.json",
      state: "recovery-required",
      campaignId: null,
      campaignTitle: null,
      revision: null,
      persistenceError: "save is malformed",
    };
    let statusCalls = 0;
    let resetRequest:
      | Parameters<RustyD20Transport["resetSession"]>[0]
      | undefined;
    const store = new SessionStore(
      transport({
        loadSession: async () => ({
          ok: false,
          error: {
            kind: "persistence",
            message: "save is malformed",
            retryable: false,
          },
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
    expect(store.session()).toMatchObject({ kind: "error" });
    expect(store.saveStatus()).toEqual({ kind: "data", value: recoveryStatus });
    await store.resetSession();
    expect(resetRequest).toEqual({
      expectedSaveIdentity: "/tmp/malformed.json",
      expectedRevision: null,
      expectedAdventureId: null,
    });
    expect(store.session()).toMatchObject({
      kind: "data",
      value: { revision: 0, campaign: null },
    });
    expect(store.saveStatus()).toEqual({ kind: "data", value: saveStatus });
  });
});
