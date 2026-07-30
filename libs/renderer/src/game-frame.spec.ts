import { describe, expect, it } from "vitest";

import { createGameRenderFrame, type GameViewportView } from "./game-frame";

const campView: GameViewportView = {
  mode: "camp",
  label: "Warden's Gate camp",
  dungeon: null,
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
      { mode: "catalog", label: "Choose an adventure", dungeon: null },
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
});
