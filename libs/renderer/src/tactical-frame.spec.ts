import { describe, expect, it } from "vitest";

import {
  createTacticalRenderFrame,
  type TacticalBoardView,
} from "./tactical-frame";

const board: TacticalBoardView = {
  width: 3,
  height: 2,
  interactionMode: "targeting",
  targetingActionId: "longsword-strike",
  targetingActionLabel: "Longsword Strike",
  cells: [
    {
      id: "0:0",
      x: 0,
      y: 0,
      terrain: "wall",
      participantId: null,
      participantName: null,
      faction: null,
      defeated: false,
      current: false,
      legalActionTarget: false,
      legalMoveCost: null,
      route: null,
    },
    {
      id: "1:0",
      x: 1,
      y: 0,
      terrain: "floor",
      participantId: 101,
      participantName: "Mara Venn",
      faction: "party",
      defeated: false,
      current: true,
      legalActionTarget: false,
      legalMoveCost: null,
      route: null,
    },
    {
      id: "2:0",
      x: 2,
      y: 0,
      terrain: "floor",
      participantId: null,
      participantName: null,
      faction: null,
      defeated: false,
      current: false,
      legalActionTarget: false,
      legalMoveCost: 2,
      route: [
        { x: 1, y: 0 },
        { x: 1, y: 1 },
        { x: 2, y: 1 },
        { x: 2, y: 0 },
      ],
    },
    {
      id: "0:1",
      x: 0,
      y: 1,
      terrain: "floor",
      participantId: null,
      participantName: null,
      faction: null,
      defeated: false,
      current: false,
      legalActionTarget: false,
      legalMoveCost: null,
      route: null,
    },
    {
      id: "1:1",
      x: 1,
      y: 1,
      terrain: "floor",
      participantId: 102,
      participantName: "Iron Warden",
      faction: "opposition",
      defeated: false,
      current: false,
      legalActionTarget: true,
      legalMoveCost: null,
      route: null,
    },
    {
      id: "2:1",
      x: 2,
      y: 1,
      terrain: "floor",
      participantId: null,
      participantName: null,
      faction: null,
      defeated: false,
      current: false,
      legalActionTarget: false,
      legalMoveCost: 1,
      route: [
        { x: 1, y: 0 },
        { x: 1, y: 1 },
        { x: 2, y: 1 },
      ],
    },
  ],
};

describe("createTacticalRenderFrame", () => {
  it("maps immutable board facts, occupancy, routes, and states into retained nodes", () => {
    const scene = createTacticalRenderFrame(board);
    const creates = scene.frame.ops.filter((op) => op.op === "create");

    expect(
      creates.filter((op) => op.node.metadata.tags.includes("tactical-cell")),
    ).toHaveLength(6);
    expect(
      creates.filter((op) =>
        op.node.metadata.tags.includes("tactical-participant"),
      ),
    ).toHaveLength(2);
    expect(
      creates.filter((op) => op.node.metadata.tags.includes("movement-route")),
    ).toHaveLength(3);
    expect(
      creates.find((op) => op.node.metadata.label === "tactical-entity-101")
        ?.node.metadata,
    ).toMatchObject({
      sourceEntity: 101,
      tags: expect.arrayContaining(["party", "active"]),
    });
    expect(
      creates.find((op) => op.node.metadata.label === "tactical-entity-102")
        ?.node.metadata.tags,
    ).toEqual(expect.arrayContaining(["opposition", "legal-action-target"]));
  });

  it("keeps stable handles and typed cell/entity pick identities across frames", () => {
    const first = createTacticalRenderFrame(board);
    const reordered: TacticalBoardView = {
      ...board,
      cells: [...board.cells].reverse(),
    };
    const second = createTacticalRenderFrame(reordered, first.handles);

    expect(second.frame.ops.slice(0, first.handles.length)).toEqual(
      first.handles.map((handle) => ({ op: "destroy", handle })),
    );
    expect(second.picks).toEqual(first.picks);
    expect(
      first.picks.find((pick) => pick.identity === "cell:2:1"),
    ).toMatchObject({
      label: "Move to 2, 1, cost 1",
      selection: { x: 2, y: 1, participantId: null },
    });
    expect(
      first.picks.find((pick) => pick.identity === "entity:102"),
    ).toMatchObject({
      label:
        "Iron Warden, opposition, at 1, 1, legal target for Longsword Strike",
      selection: { x: 1, y: 1, participantId: 102 },
    });
  });
});
