import {
  renderHandle,
  type RenderDiff,
  type RenderFrameDiff,
  type RenderHandle,
  type Vec4,
} from "@rusty-engine/render-contracts";

import {
  createDungeonRenderFrame,
  type DungeonViewportView,
} from "./dungeon-frame";
import {
  createTacticalRenderFrame,
  type TacticalBoardView,
  type TacticalCameraFit,
  type TacticalScenePick,
} from "./tactical-frame";

export type GameSceneMode =
  | "loading"
  | "catalog"
  | "camp"
  | "exploration"
  | "encounter"
  | "outcome"
  | "complete"
  | "error";

export interface GameViewportView {
  readonly mode: GameSceneMode;
  readonly label: string;
  readonly dungeon: DungeonViewportView | null;
  readonly tactical: TacticalBoardView | null;
}

export interface GameCameraPose {
  readonly position: readonly [number, number, number];
  readonly pitchDegrees: number;
  readonly yawDegrees: number;
}

export interface GameRenderFrame {
  readonly frame: RenderFrameDiff;
  readonly handles: readonly RenderHandle[];
  readonly camera: GameCameraPose;
  readonly cameraFit: TacticalCameraFit | null;
  readonly picks: readonly TacticalScenePick[];
}

const FIRST_SCENE_HANDLE = 100;

const MODE_COLORS: Readonly<
  Record<
    Exclude<GameSceneMode, "exploration">,
    { readonly floor: Vec4; readonly horizon: Vec4; readonly accent: Vec4 }
  >
> = {
  loading: {
    floor: [0.07, 0.09, 0.1, 1],
    horizon: [0.1, 0.14, 0.15, 1],
    accent: [0.25, 0.52, 0.54, 1],
  },
  catalog: {
    floor: [0.08, 0.11, 0.12, 1],
    horizon: [0.12, 0.18, 0.2, 1],
    accent: [0.35, 0.72, 0.68, 1],
  },
  camp: {
    floor: [0.12, 0.1, 0.08, 1],
    horizon: [0.18, 0.14, 0.1, 1],
    accent: [0.72, 0.46, 0.2, 1],
  },
  encounter: {
    floor: [0.09, 0.11, 0.1, 1],
    horizon: [0.12, 0.15, 0.14, 1],
    accent: [0.42, 0.66, 0.62, 1],
  },
  outcome: {
    floor: [0.09, 0.11, 0.1, 1],
    horizon: [0.12, 0.16, 0.14, 1],
    accent: [0.52, 0.72, 0.44, 1],
  },
  complete: {
    floor: [0.08, 0.1, 0.11, 1],
    horizon: [0.1, 0.16, 0.18, 1],
    accent: [0.42, 0.74, 0.7, 1],
  },
  error: {
    floor: [0.12, 0.06, 0.06, 1],
    horizon: [0.2, 0.08, 0.07, 1],
    accent: [0.76, 0.26, 0.18, 1],
  },
};

/**
 * Builds the presentation scene behind Rust-owned game overlays.
 *
 * Exploration delegates to the bounded, occlusion-safe dungeon adapter.
 * Every other mode is deliberately an abstract backdrop: its nodes contain no
 * entity IDs, navigation facts, targets, inventory facts, or gameplay state.
 */
export function createGameRenderFrame(
  view: GameViewportView,
  previousHandles: readonly RenderHandle[] = [],
): GameRenderFrame {
  if (view.mode === "exploration" && view.dungeon !== null) {
    const dungeon = createDungeonRenderFrame(view.dungeon, previousHandles);
    return {
      ...dungeon,
      camera: {
        position: [0, 1.35, 0.55],
        pitchDegrees: 0,
        yawDegrees: 0,
      },
      cameraFit: null,
      picks: [],
    };
  }

  if (
    (view.mode === "encounter" || view.mode === "outcome") &&
    view.tactical !== null
  ) {
    const tactical = createTacticalRenderFrame(view.tactical, previousHandles);
    return {
      ...tactical,
      camera: tacticalCameraPose(tactical.cameraFit, 16 / 9),
    };
  }

  const mode = view.mode === "exploration" ? "loading" : view.mode;
  const colors = MODE_COLORS[mode];
  const ops: RenderDiff[] = previousHandles.map((handle) => ({
    op: "destroy",
    handle,
  }));
  const handles: RenderHandle[] = [];
  let nextHandle = FIRST_SCENE_HANDLE;

  const addCuboid = (
    label: string,
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
        material: { color, wireframe: false },
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
          tags: ["rusty-d20", "game-backdrop", mode],
          label: `game-backdrop-${mode}-${label}`,
        },
      },
    });
  };

  addCuboid("floor", [0, -0.15, -5], [16, 0.3, 18], colors.floor);
  addCuboid("horizon", [0, 3.2, -13.5], [18, 6.5, 0.4], colors.horizon);
  addCuboid("left-marker", [-5.2, 1.4, -7], [0.45, 3.1, 0.45], colors.accent);
  addCuboid("right-marker", [5.2, 1.4, -7], [0.45, 3.1, 0.45], colors.accent);
  addCuboid("dais", [0, 0.12, -6.2], [5.6, 0.24, 3.6], colors.accent);

  return {
    frame: { schemaVersion: 1, ops },
    handles,
    camera: {
      position: [0, 2.1, 4.8],
      pitchDegrees: -8,
      yawDegrees: 0,
    },
    cameraFit: null,
    picks: [],
  };
}

export function tacticalCameraPose(
  fit: TacticalCameraFit,
  aspectRatio: number,
): GameCameraPose {
  const safeAspect =
    Number.isFinite(aspectRatio) && aspectRatio > 0 ? aspectRatio : 1;
  const halfFovRadians = (58 * Math.PI) / 360;
  const distanceForHeight = fit.height / (2 * Math.tan(halfFovRadians));
  const distanceForWidth =
    fit.width / (2 * Math.tan(halfFovRadians) * safeAspect);
  const distance = Math.max(distanceForHeight, distanceForWidth) * 1.12 + 0.8;
  return {
    position: [0, distance, 0],
    pitchDegrees: -90,
    yawDegrees: 0,
  };
}
