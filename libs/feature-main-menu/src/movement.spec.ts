import { describe, expect, it } from "vitest";

import {
  movementIsCurrent,
  selectMovementDestination,
  startMovement,
  type MovementProjection,
} from "./movement";

const projection: MovementProjection = {
  campaignId: "wardens-gate",
  encounterId: "iron-warden",
  phase: "encounter",
  revision: 12,
  currentActorId: 101,
  currentFaction: "party",
  reactionPending: false,
  legalMoves: [
    {
      x: 2,
      y: 1,
      cost: 1,
      route: [
        { x: 1, y: 1 },
        { x: 2, y: 1 },
      ],
    },
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
};

describe("tactical movement presentation state", () => {
  it("previews the exact projected route before confirming the same cell", () => {
    const started = startMovement(projection);
    expect(started.ok).toBe(true);
    if (!started.ok) {
      throw new Error(started.message);
    }

    const first = selectMovementDestination(started.mode, projection, 2, 2);
    expect(first).toMatchObject({
      kind: "preview",
      destination: {
        x: 2,
        y: 2,
        cost: 2,
        route: projection.legalMoves[1]?.route,
      },
    });
    if (first.kind !== "preview") {
      throw new Error("expected movement preview");
    }
    expect(
      selectMovementDestination(first.mode, projection, 2, 2),
    ).toMatchObject({
      kind: "confirm",
      command: { revision: 12, actorId: 101, x: 2, y: 2 },
    });
  });

  it("replaces a preview and rejects cells outside the projected move set", () => {
    const started = startMovement(projection);
    if (!started.ok) {
      throw new Error(started.message);
    }
    const first = selectMovementDestination(started.mode, projection, 2, 2);
    if (first.kind !== "preview") {
      throw new Error("expected movement preview");
    }
    const replaced = selectMovementDestination(first.mode, projection, 2, 1);
    expect(replaced).toMatchObject({
      kind: "preview",
      mode: { preview: { x: 2, y: 1, cost: 1 } },
    });
    expect(selectMovementDestination(first.mode, projection, 9, 9)).toEqual({
      kind: "rejected",
      message: "That cell is not a Rust-projected legal movement destination.",
    });
  });

  it("cancels on revision, actor, phase, reaction, or projected-route changes", () => {
    const started = startMovement(projection);
    if (!started.ok) {
      throw new Error(started.message);
    }
    const [nearMove, farMove] = projection.legalMoves;
    if (nearMove === undefined || farMove === undefined) {
      throw new Error("movement fixture is incomplete");
    }
    for (const stale of [
      { ...projection, revision: 13 },
      { ...projection, currentActorId: 102 },
      { ...projection, phase: "outcome" },
      { ...projection, reactionPending: true },
      {
        ...projection,
        legalMoves: [
          {
            ...nearMove,
            route: [
              { x: 1, y: 1 },
              { x: 1, y: 2 },
              { x: 2, y: 1 },
            ],
          },
          farMove,
        ],
      },
    ]) {
      expect(movementIsCurrent(started.mode, stale)).toBe(false);
      expect(
        selectMovementDestination(started.mode, stale, 2, 1),
      ).toMatchObject({ kind: "rejected" });
    }
  });

  it("rejects selection without an eligible party move", () => {
    expect(
      startMovement({ ...projection, currentFaction: "opposition" }),
    ).toEqual({
      ok: false,
      message: "Movement can be selected only during a party activation.",
    });
    expect(startMovement({ ...projection, legalMoves: [] })).toEqual({
      ok: false,
      message: "Rust projects no legal movement destinations.",
    });
  });
});
