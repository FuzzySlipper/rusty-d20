import { describe, expect, it } from "vitest";

import {
  startTargeting,
  targetingCommand,
  targetingIsCurrent,
  type TargetingProjection,
} from "./targeting";

const projection: TargetingProjection = {
  campaignId: "wardens-gate",
  encounterId: "iron-warden",
  phase: "encounter",
  revision: 12,
  currentActorId: 101,
  currentFaction: "party",
  reactionPending: false,
  actions: [
    { id: "strike", label: "Strike" },
    { id: "aid", label: "Aid" },
    { id: "guard", label: "Guard Self" },
  ],
  legalTargets: [
    { actionId: "strike", targetIds: [201, 202] },
    { actionId: "aid", targetIds: [102] },
    { actionId: "guard", targetIds: [101] },
  ],
};

describe("tactical targeting presentation state", () => {
  it("uses the exact Rust-projected hostile, ally, and self target sets", () => {
    for (const [actionId, targetId] of [
      ["strike", 201],
      ["aid", 102],
      ["guard", 101],
    ] as const) {
      const started = startTargeting(projection, actionId);
      expect(started.ok).toBe(true);
      if (!started.ok) {
        throw new Error(started.message);
      }
      expect(targetingCommand(started.mode, projection, targetId)).toEqual({
        revision: 12,
        actorId: 101,
        actionId,
        targetId,
      });
      expect(targetingCommand(started.mode, projection, 999)).toBeNull();
    }
  });

  it("cancels on revision, actor, phase, reaction, action, or target changes", () => {
    const started = startTargeting(projection, "strike");
    if (!started.ok) {
      throw new Error(started.message);
    }
    for (const stale of [
      { ...projection, revision: 13 },
      { ...projection, currentActorId: 102 },
      { ...projection, phase: "outcome" },
      { ...projection, reactionPending: true },
      { ...projection, actions: projection.actions.slice(1) },
      {
        ...projection,
        legalTargets: [
          { actionId: "strike", targetIds: [202] },
          ...projection.legalTargets.slice(1),
        ],
      },
    ]) {
      expect(targetingIsCurrent(started.mode, stale)).toBe(false);
      expect(targetingCommand(started.mode, stale, 201)).toBeNull();
    }
  });

  it("rejects action selection outside an eligible party decision", () => {
    expect(
      startTargeting({ ...projection, currentFaction: "opposition" }, "strike"),
    ).toEqual({
      ok: false,
      message: "Actions can be targeted only during a party activation.",
    });
    expect(startTargeting(projection, "missing")).toEqual({
      ok: false,
      message: "That action is no longer available in the Rust projection.",
    });
  });
});
