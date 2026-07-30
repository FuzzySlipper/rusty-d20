import { describe, expect, it } from "vitest";

import {
  createGameRenderFrame,
  tacticalCameraPose,
  type GameViewportView,
} from "./game-frame";

const campView: GameViewportView = {
  mode: "camp",
  label: "Warden's Gate camp",
  dungeon: null,
  tactical: null,
};

describe("createGameRenderFrame", () => {
  it("creates an abstract mode backdrop without gameplay source identities", () => {
    const scene = createGameRenderFrame(campView);
    const creates = scene.frame.ops.filter((op) => op.op === "create");

    expect(creates).toHaveLength(5);
    expect(
      creates.every(
        (op) =>
          op.node.metadata.sourceEntity === null &&
          op.node.metadata.sourceSceneNode === null &&
          op.node.metadata.tags.includes("game-backdrop") &&
          op.node.metadata.tags.includes("camp"),
      ),
    ).toBe(true);
  });

  it("destroys the previous mode before creating the next backdrop", () => {
    const camp = createGameRenderFrame(campView);
    const catalog = createGameRenderFrame(
      {
        mode: "catalog",
        label: "Choose an adventure",
        dungeon: null,
        tactical: null,
      },
      camp.handles,
    );

    expect(catalog.frame.ops.slice(0, camp.handles.length)).toEqual(
      camp.handles.map((handle) => ({ op: "destroy", handle })),
    );
    expect(
      catalog.frame.ops
        .filter((op) => op.op === "create")
        .every((op) => op.node.metadata.tags.includes("catalog")),
    ).toBe(true);
  });

  it("delegates exploration to the bounded dungeon frame and camera", () => {
    const scene = createGameRenderFrame({
      mode: "exploration",
      label: "Warden's Gate Pass, facing east",
      dungeon: {
        title: "Warden's Gate Pass",
        wallStyle: "mountain-fortress",
        facing: "east",
        x: 1,
        y: 1,
        depths: [
          {
            depth: 0,
            frontBlocked: true,
            leftBlocked: false,
            rightBlocked: true,
          },
          {
            depth: 1,
            frontBlocked: false,
            leftBlocked: true,
            rightBlocked: false,
          },
        ],
      },
      tactical: null,
    });
    const labels = scene.frame.ops
      .filter((op) => op.op === "create")
      .map((op) => op.node.metadata.label);

    expect(labels).toContain("dungeon-front-0");
    expect(labels).not.toContain("dungeon-left-1");
    expect(scene.camera).toEqual({
      position: [0, 1.35, 0.55],
      pitchDegrees: 0,
      yawDegrees: 0,
    });
  });

  it("promotes encounter and outcome boards into the persistent renderer scene", () => {
    const tactical = {
      width: 1,
      height: 1,
      interactionMode: "movement" as const,
      targetingActionId: null,
      targetingActionLabel: null,
      cells: [
        {
          id: "0:0",
          x: 0,
          y: 0,
          terrain: "floor" as const,
          participantId: 101,
          participantName: "Mara Venn",
          faction: "party" as const,
          defeated: false,
          current: true,
          legalActionTarget: false,
          legalMoveCost: null,
          route: null,
        },
      ],
    };
    const encounter = createGameRenderFrame({
      mode: "encounter",
      label: "Rendered encounter",
      dungeon: null,
      tactical,
    });
    const outcome = createGameRenderFrame(
      {
        mode: "outcome",
        label: "Rendered outcome",
        dungeon: null,
        tactical: {
          ...tactical,
          cells: tactical.cells.map((cell) => ({
            ...cell,
            current: false,
            defeated: true,
          })),
        },
      },
      encounter.handles,
    );

    expect(
      encounter.frame.ops
        .filter((op) => op.op === "create")
        .some((op) => op.node.metadata.tags.includes("game-backdrop")),
    ).toBe(false);
    expect(encounter.picks.map((pick) => pick.identity)).toContain(
      "entity:101",
    );
    expect(outcome.frame.ops.slice(0, encounter.handles.length)).toEqual(
      encounter.handles.map((handle) => ({ op: "destroy", handle })),
    );
    expect(
      outcome.frame.ops
        .filter((op) => op.op === "create")
        .find((op) => op.node.metadata.label === "tactical-entity-101")?.node
        .metadata.tags,
    ).toContain("defeated");
  });

  it("fits the same overhead board at desktop and narrow viewport aspects", () => {
    const desktop = tacticalCameraPose({ width: 10, height: 7 }, 16 / 9);
    const mobile = tacticalCameraPose({ width: 10, height: 7 }, 390 / 844);

    expect(desktop.pitchDegrees).toBe(-90);
    expect(mobile.pitchDegrees).toBe(-90);
    expect(mobile.position[1]).toBeGreaterThan(desktop.position[1]);
  });
});
