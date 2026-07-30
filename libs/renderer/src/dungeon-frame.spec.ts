import { describe, expect, it } from "vitest";

import {
  createDungeonRenderFrame,
  type DungeonViewportView,
} from "./dungeon-frame";

const baseView: DungeonViewportView = {
  title: "Asymmetric passage",
  wallStyle: "mountain-fortress",
  facing: "north",
  x: 4,
  y: 3,
  depths: [
    {
      depth: 0,
      frontBlocked: false,
      leftBlocked: true,
      rightBlocked: false,
    },
    {
      depth: 1,
      frontBlocked: false,
      leftBlocked: false,
      rightBlocked: true,
    },
    {
      depth: 2,
      frontBlocked: true,
      leftBlocked: true,
      rightBlocked: true,
    },
  ],
};

describe("createDungeonRenderFrame", () => {
  it("projects side and front walls onto correctly oriented corridor planes", () => {
    const result = createDungeonRenderFrame(baseView);
    const creates = result.frame.ops.filter((op) => op.op === "create");
    const byLabel = new Map(
      creates.map((op) => [op.node.metadata.label, op.node.transform]),
    );

    expect(byLabel.get("dungeon-left-0")).toMatchObject({
      translation: [-1.1, 1.25, -1],
      scale: [0.2, 2.5, 2],
    });
    expect(byLabel.get("dungeon-right-1")).toMatchObject({
      translation: [1.1, 1.25, -3],
      scale: [0.2, 2.5, 2],
    });
    expect(byLabel.get("dungeon-front-2")).toMatchObject({
      translation: [0, 1.25, -6],
      scale: [2.4, 2.5, 0.2],
    });
    expect(byLabel.has("dungeon-right-0")).toBe(false);
    expect(byLabel.has("dungeon-left-1")).toBe(false);
  });

  it("does not sample or render neutral records behind an occluding wall", () => {
    const hiddenOpen = {
      ...baseView,
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
          leftBlocked: false,
          rightBlocked: false,
        },
        {
          depth: 2,
          frontBlocked: false,
          leftBlocked: true,
          rightBlocked: false,
        },
      ],
    };
    const hiddenClosed = {
      ...hiddenOpen,
      depths: [
        hiddenOpen.depths[0],
        {
          depth: 1,
          frontBlocked: true,
          leftBlocked: true,
          rightBlocked: true,
        },
        {
          depth: 2,
          frontBlocked: true,
          leftBlocked: true,
          rightBlocked: true,
        },
      ],
    };

    expect(createDungeonRenderFrame(hiddenOpen)).toEqual(
      createDungeonRenderFrame(hiddenClosed),
    );
    expect(createDungeonRenderFrame(hiddenOpen).handles).toHaveLength(4);
  });

  it("replaces the prior retained nodes before creating the next view", () => {
    const first = createDungeonRenderFrame(baseView);
    const second = createDungeonRenderFrame(
      { ...baseView, facing: "east" },
      first.handles,
    );

    expect(second.frame.ops.slice(0, first.handles.length)).toEqual(
      first.handles.map((handle) => ({ op: "destroy", handle })),
    );
    expect(second.handles).toEqual(first.handles);
  });

  it("uses Rust's relative projection identically for every absolute facing", () => {
    const frames = ["north", "east", "south", "west"].map((facing) =>
      createDungeonRenderFrame({ ...baseView, facing }),
    );

    expect(frames.slice(1)).toEqual([frames[0], frames[0], frames[0]]);
  });
});
