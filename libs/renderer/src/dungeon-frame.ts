import {
  renderHandle,
  type RenderDiff,
  type RenderFrameDiff,
  type RenderHandle,
  type Vec4,
} from "@rusty-engine/render-contracts";

export interface DungeonDepthView {
  readonly depth: number;
  readonly frontBlocked: boolean;
  readonly leftBlocked: boolean;
  readonly rightBlocked: boolean;
}

export interface DungeonViewportView {
  readonly title: string;
  readonly wallStyle: string;
  readonly facing: "north" | "east" | "south" | "west";
  readonly x: number;
  readonly y: number;
  readonly depths: readonly DungeonDepthView[];
}

export interface DungeonRenderFrame {
  readonly frame: RenderFrameDiff;
  readonly handles: readonly RenderHandle[];
}

const CELL_SIZE = 2;
const WALL_HEIGHT = 2.5;
const WALL_THICKNESS = 0.2;
const FIRST_SCENE_HANDLE = 100;

interface DungeonPalette {
  readonly ceiling: Vec4;
  readonly floor: Vec4;
  readonly wall: Vec4;
}

const PALETTES: Readonly<Record<string, DungeonPalette>> = {
  "ember-vault": {
    ceiling: [0.16, 0.08, 0.05, 1],
    floor: [0.17, 0.09, 0.05, 1],
    wall: [0.46, 0.19, 0.08, 1],
  },
  "mountain-fortress": {
    ceiling: [0.12, 0.15, 0.17, 1],
    floor: [0.16, 0.17, 0.16, 1],
    wall: [0.31, 0.36, 0.37, 1],
  },
};
const DEFAULT_PALETTE: DungeonPalette = {
  ceiling: [0.11, 0.13, 0.15, 1],
  floor: [0.14, 0.14, 0.13, 1],
  wall: [0.3, 0.33, 0.34, 1],
};

/**
 * Adapts Rust's camera-relative, occlusion-safe dungeon facts into the shared
 * Engine retained-frame contract. It deliberately stops at the first opaque
 * front wall instead of interpreting the neutral hidden suffix as topology.
 */
export function createDungeonRenderFrame(
  view: DungeonViewportView,
  previousHandles: readonly RenderHandle[] = [],
): DungeonRenderFrame {
  const ops: RenderDiff[] = previousHandles.map((handle) => ({
    op: "destroy",
    handle,
  }));
  const handles: RenderHandle[] = [];
  const palette = PALETTES[view.wallStyle] ?? DEFAULT_PALETTE;
  let nextHandle = FIRST_SCENE_HANDLE;

  const addCuboid = (
    kind: "ceiling" | "floor" | "front" | "left" | "right",
    depth: number,
    translation: readonly [number, number, number],
    scale: readonly [number, number, number],
    color: Vec4,
  ): void => {
    const handle = renderHandle(nextHandle);
    nextHandle += 1;
    handles.push(handle);
    ops.push({
      op: "create",
      handle,
      parent: null,
      node: {
        geometry: { kind: "cube" },
        material: {
          color: shadeForDepth(color, depth),
          wireframe: false,
        },
        transform: {
          translation,
          rotation: [0, 0, 0, 1],
          scale,
        },
        visible: true,
        layer: "scene",
        metadata: {
          sourceEntity: null,
          sourceSceneNode: null,
          tags: ["rusty-d20", "dungeon", kind, `depth-${String(depth)}`],
          label: `dungeon-${kind}-${String(depth)}`,
        },
      },
    });
  };

  for (const depthView of view.depths) {
    const depth = depthView.depth;
    const centerZ = -(depth * CELL_SIZE + CELL_SIZE / 2);
    addCuboid(
      "floor",
      depth,
      [0, -WALL_THICKNESS / 2, centerZ],
      [CELL_SIZE + WALL_THICKNESS * 2, WALL_THICKNESS, CELL_SIZE],
      palette.floor,
    );
    addCuboid(
      "ceiling",
      depth,
      [0, WALL_HEIGHT + WALL_THICKNESS / 2, centerZ],
      [CELL_SIZE + WALL_THICKNESS * 2, WALL_THICKNESS, CELL_SIZE],
      palette.ceiling,
    );
    if (depthView.leftBlocked) {
      addCuboid(
        "left",
        depth,
        [-(CELL_SIZE + WALL_THICKNESS) / 2, WALL_HEIGHT / 2, centerZ],
        [WALL_THICKNESS, WALL_HEIGHT, CELL_SIZE],
        palette.wall,
      );
    }
    if (depthView.rightBlocked) {
      addCuboid(
        "right",
        depth,
        [(CELL_SIZE + WALL_THICKNESS) / 2, WALL_HEIGHT / 2, centerZ],
        [WALL_THICKNESS, WALL_HEIGHT, CELL_SIZE],
        palette.wall,
      );
    }
    if (depthView.frontBlocked) {
      addCuboid(
        "front",
        depth,
        [0, WALL_HEIGHT / 2, -(depth + 1) * CELL_SIZE],
        [CELL_SIZE + WALL_THICKNESS * 2, WALL_HEIGHT, WALL_THICKNESS],
        palette.wall,
      );
      break;
    }
  }

  return {
    frame: { schemaVersion: 1, ops },
    handles,
  };
}

function shadeForDepth(color: Vec4, depth: number): Vec4 {
  const shade = Math.max(0.62, 1 - depth * 0.14);
  return [color[0] * shade, color[1] * shade, color[2] * shade, color[3]];
}
