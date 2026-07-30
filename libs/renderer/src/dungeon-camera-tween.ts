import type {
  AnimationFramePort,
  MotionPreferencePort,
} from "@rusty-d20/platform";
import type {
  RendererCameraSnapshot,
  RendererCameraTransitionReadout,
  RendererSurfaceCameraPose,
} from "@rusty-engine/renderer-host";
import type {
  CameraBasis,
  PerspectiveProjection,
} from "@rusty-engine/render-contracts";

import type { DungeonViewportView } from "./dungeon-frame";

export const DUNGEON_CAMERA_TWEEN_DURATION_MS = 110;

export type DungeonCameraMotionKind =
  | "step-forward"
  | "step-backward"
  | "turn-left"
  | "turn-right";

export interface DungeonCameraTransition {
  readonly kind: DungeonCameraMotionKind;
  readonly from: RendererSurfaceCameraPose;
  readonly to: RendererSurfaceCameraPose;
  readonly durationMilliseconds: number;
}

export interface DungeonCameraTweenStatus {
  readonly kind: DungeonCameraMotionKind | null;
  readonly state: "running" | "settled";
  readonly reducedMotion: boolean;
}

export type CameraTransitionSampler = (
  transition: RendererCameraTransitionReadout,
  elapsedMilliseconds: number,
) => RendererCameraSnapshot;

export interface DungeonCameraTweenStart {
  readonly applyPose: (pose: RendererSurfaceCameraPose) => void;
  readonly onError: (error: unknown) => void;
  readonly onStatus: (status: DungeonCameraTweenStatus) => void;
  readonly projection: PerspectiveProjection;
  readonly transition: DungeonCameraTransition;
  readonly viewport: {
    readonly width: number;
    readonly height: number;
  };
}

const CELL_SIZE = 2;
const FACING_ORDER = ["north", "east", "south", "west"] as const;

/**
 * Derives a presentation-only camera motion from two accepted Rust dungeon
 * projections. The newly accepted occlusion-safe frame is never interpolated.
 */
export function dungeonCameraTransition(
  previous: DungeonViewportView,
  next: DungeonViewportView,
  canonicalPose: RendererSurfaceCameraPose,
): DungeonCameraTransition | null {
  if (previous.title !== next.title || previous.wallStyle !== next.wallStyle) {
    return null;
  }

  if (previous.x === next.x && previous.y === next.y) {
    const previousFacing = FACING_ORDER.indexOf(previous.facing);
    const nextFacing = FACING_ORDER.indexOf(next.facing);
    if (previousFacing < 0 || nextFacing < 0) {
      return null;
    }
    const quarterTurns =
      (nextFacing - previousFacing + FACING_ORDER.length) % FACING_ORDER.length;
    const kind =
      quarterTurns === 1
        ? "turn-right"
        : quarterTurns === 3
          ? "turn-left"
          : null;
    if (kind === null) {
      return null;
    }
    return {
      kind,
      from: {
        ...canonicalPose,
        yawDegrees: kind === "turn-right" ? -90 : 90,
      },
      to: canonicalPose,
      durationMilliseconds: DUNGEON_CAMERA_TWEEN_DURATION_MS,
    };
  }

  if (previous.facing !== next.facing) {
    return null;
  }
  const facingVector = vectorForFacing(next.facing);
  if (facingVector === null) {
    return null;
  }
  const deltaX = next.x - previous.x;
  const deltaY = next.y - previous.y;
  if (Math.abs(deltaX) + Math.abs(deltaY) !== 1) {
    return null;
  }
  const direction = deltaX * facingVector[0] + deltaY * facingVector[1];
  const kind =
    direction === 1
      ? "step-forward"
      : direction === -1
        ? "step-backward"
        : null;
  if (kind === null) {
    return null;
  }
  return {
    kind,
    from: {
      ...canonicalPose,
      position: [
        canonicalPose.position[0],
        canonicalPose.position[1],
        canonicalPose.position[2] +
          (kind === "step-forward" ? CELL_SIZE : -CELL_SIZE),
      ],
    },
    to: canonicalPose,
    durationMilliseconds: DUNGEON_CAMERA_TWEEN_DURATION_MS,
  };
}

/**
 * Owns at most one browser animation frame. A newer accepted projection
 * cancels the prior tween and starts from its own deterministic offset.
 */
export class DungeonCameraTween {
  private frameHandle: number | null = null;
  private generation = 0;
  private pose: RendererSurfaceCameraPose | null = null;

  constructor(
    private readonly animationFrame: AnimationFramePort,
    private readonly motionPreference: MotionPreferencePort,
    private readonly sampleTransition: CameraTransitionSampler,
  ) {}

  start(options: DungeonCameraTweenStart): void {
    this.cancel();
    const reducedMotion = this.motionPreference.prefersReducedMotion();
    if (reducedMotion) {
      this.pose = options.transition.to;
      if (!this.apply(options, options.transition.to)) {
        return;
      }
      options.onStatus({
        kind: options.transition.kind,
        state: "settled",
        reducedMotion: true,
      });
      return;
    }

    const generation = this.generation;
    const readout: RendererCameraTransitionReadout = {
      from: cameraSnapshot(
        options.transition.from,
        options.projection,
        options.viewport,
      ),
      to: cameraSnapshot(
        options.transition.to,
        options.projection,
        options.viewport,
      ),
      durationMilliseconds: options.transition.durationMilliseconds,
      easing: "smoothStep",
    };
    let startedAt: number | null = null;
    this.pose = options.transition.from;
    options.onStatus({
      kind: options.transition.kind,
      state: "running",
      reducedMotion: false,
    });
    if (!this.apply(options, options.transition.from)) {
      return;
    }

    const sample = (timestamp: number): void => {
      if (generation !== this.generation) {
        return;
      }
      startedAt ??= timestamp;
      const elapsed = Math.max(0, timestamp - startedAt);
      const snapshot = this.sampleTransition(readout, elapsed);
      this.pose = snapshot.pose;
      if (!this.apply(options, snapshot.pose)) {
        return;
      }
      if (elapsed >= options.transition.durationMilliseconds) {
        this.frameHandle = null;
        this.pose = options.transition.to;
        options.onStatus({
          kind: options.transition.kind,
          state: "settled",
          reducedMotion: false,
        });
        return;
      }
      this.frameHandle = this.animationFrame.request(sample);
    };
    this.frameHandle = this.animationFrame.request(sample);
  }

  settle(
    pose: RendererSurfaceCameraPose,
    applyPose: (pose: RendererSurfaceCameraPose) => void,
  ): void {
    this.cancel();
    this.pose = pose;
    applyPose(pose);
  }

  reapply(applyPose: (pose: RendererSurfaceCameraPose) => void): boolean {
    if (this.pose === null) {
      return false;
    }
    applyPose(this.pose);
    return true;
  }

  dispose(): void {
    this.cancel();
    this.pose = null;
  }

  private apply(
    options: DungeonCameraTweenStart,
    pose: RendererSurfaceCameraPose,
  ): boolean {
    try {
      options.applyPose(pose);
      return true;
    } catch (error) {
      this.cancel();
      options.onError(error);
      return false;
    }
  }

  private cancel(): void {
    this.generation += 1;
    if (this.frameHandle !== null) {
      this.animationFrame.cancel(this.frameHandle);
      this.frameHandle = null;
    }
  }
}

function vectorForFacing(
  facing: DungeonViewportView["facing"],
): readonly [number, number] | null {
  return facing === "north"
    ? [0, -1]
    : facing === "east"
      ? [1, 0]
      : facing === "south"
        ? [0, 1]
        : facing === "west"
          ? [-1, 0]
          : null;
}

function cameraSnapshot(
  pose: RendererSurfaceCameraPose,
  projection: PerspectiveProjection,
  viewport: { readonly width: number; readonly height: number },
): RendererCameraSnapshot {
  return {
    pose,
    basis: cameraBasis(pose),
    projection,
    viewport,
  };
}

function cameraBasis(pose: RendererSurfaceCameraPose): CameraBasis {
  const yaw = (pose.yawDegrees * Math.PI) / 180;
  const pitch = (pose.pitchDegrees * Math.PI) / 180;
  const cosPitch = Math.cos(pitch);
  return {
    forward: [
      Math.sin(yaw) * cosPitch,
      Math.sin(pitch),
      -Math.cos(yaw) * cosPitch,
    ],
    right: [Math.cos(yaw), 0, Math.sin(yaw)],
    up: [
      -Math.sin(yaw) * Math.sin(pitch),
      Math.cos(pitch),
      Math.cos(yaw) * Math.sin(pitch),
    ],
  };
}
