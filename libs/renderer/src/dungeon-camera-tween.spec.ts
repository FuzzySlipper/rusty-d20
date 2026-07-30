import { describe, expect, it } from "vitest";

import type {
  AnimationFramePort,
  MotionPreferencePort,
} from "@rusty-d20/platform";
import { sampleCameraTransition } from "@rusty-engine/renderer-host";

import {
  DUNGEON_CAMERA_TWEEN_DURATION_MS,
  dungeonCameraTransition,
  DungeonCameraTween,
  type DungeonCameraTransition,
  type DungeonCameraTweenStatus,
} from "./dungeon-camera-tween";
import type { DungeonViewportView } from "./dungeon-frame";
import type { GameCameraPose } from "./game-frame";

const canonicalPose: GameCameraPose = {
  position: [0, 1.35, 0.55],
  pitchDegrees: 0,
  yawDegrees: 0,
};

const baseView: DungeonViewportView = {
  title: "Warden's Gate Pass",
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
  ],
};

const facingCases = [
  { facing: "north", forward: [0, -1], right: "east", left: "west" },
  { facing: "east", forward: [1, 0], right: "south", left: "north" },
  { facing: "south", forward: [0, 1], right: "west", left: "east" },
  { facing: "west", forward: [-1, 0], right: "north", left: "south" },
] as const;

describe("dungeonCameraTransition", () => {
  it("derives forward, backward, left, and right camera offsets for every facing", () => {
    for (const facingCase of facingCases) {
      const previous = { ...baseView, facing: facingCase.facing };
      const forward = {
        ...previous,
        x: previous.x + facingCase.forward[0],
        y: previous.y + facingCase.forward[1],
      };
      const backward = {
        ...previous,
        x: previous.x - facingCase.forward[0],
        y: previous.y - facingCase.forward[1],
      };

      expect(
        dungeonCameraTransition(previous, forward, canonicalPose),
      ).toMatchObject({
        kind: "step-forward",
        from: { position: [0, 1.35, 2.55] },
        to: canonicalPose,
      });
      expect(
        dungeonCameraTransition(previous, backward, canonicalPose),
      ).toMatchObject({
        kind: "step-backward",
        from: { position: [0, 1.35, -1.45] },
        to: canonicalPose,
      });
      expect(
        dungeonCameraTransition(
          previous,
          { ...previous, facing: facingCase.right },
          canonicalPose,
        ),
      ).toMatchObject({
        kind: "turn-right",
        from: { yawDegrees: -90 },
        to: canonicalPose,
      });
      expect(
        dungeonCameraTransition(
          previous,
          { ...previous, facing: facingCase.left },
          canonicalPose,
        ),
      ).toMatchObject({
        kind: "turn-left",
        from: { yawDegrees: 90 },
        to: canonicalPose,
      });
    }
  });

  it("does not animate rejected, non-adjacent, cross-dungeon, or topology-only updates", () => {
    expect(
      dungeonCameraTransition(baseView, baseView, canonicalPose),
    ).toBeNull();
    expect(
      dungeonCameraTransition(
        baseView,
        { ...baseView, x: baseView.x + 2 },
        canonicalPose,
      ),
    ).toBeNull();
    expect(
      dungeonCameraTransition(
        baseView,
        { ...baseView, x: baseView.x + 1 },
        canonicalPose,
      ),
    ).toBeNull();
    expect(
      dungeonCameraTransition(
        baseView,
        { ...baseView, facing: "south" },
        canonicalPose,
      ),
    ).toBeNull();
    expect(
      dungeonCameraTransition(
        baseView,
        { ...baseView, title: "Another dungeon" },
        canonicalPose,
      ),
    ).toBeNull();
    expect(
      dungeonCameraTransition(
        baseView,
        {
          ...baseView,
          depths: [
            {
              depth: 0,
              frontBlocked: true,
              leftBlocked: true,
              rightBlocked: true,
            },
          ],
        },
        canonicalPose,
      ),
    ).toBeNull();
  });
});

describe("DungeonCameraTween", () => {
  it("samples the public Engine camera transition and preserves the current pose across resize", () => {
    const animationFrame = new FakeAnimationFrame();
    const motionPreference = new FakeMotionPreference();
    const poses: GameCameraPose[] = [];
    const statuses: DungeonCameraTweenStatus[] = [];
    const tween = new DungeonCameraTween(
      animationFrame,
      motionPreference,
      sampleCameraTransition,
    );

    tween.start(
      startOptions(forwardTransition(), poses, statuses, () => {
        throw new Error("unexpected transition error");
      }),
    );
    expect(poses).toEqual([forwardTransition().from]);
    expect(animationFrame.pendingCount).toBe(1);

    animationFrame.flush(1_000);
    animationFrame.flush(1_055);
    expect(poses.at(-1)?.position[0]).toBeCloseTo(0);
    expect(poses.at(-1)?.position[1]).toBeCloseTo(1.35);
    expect(poses.at(-1)?.position[2]).toBeCloseTo(1.55);

    const resizePoses: GameCameraPose[] = [];
    expect(tween.reapply((pose) => resizePoses.push(pose))).toBe(true);
    expect(resizePoses).toEqual([poses.at(-1)]);
    expect(animationFrame.pendingCount).toBe(1);

    animationFrame.flush(1_110);
    expect(poses.at(-1)).toEqual(canonicalPose);
    expect(animationFrame.pendingCount).toBe(0);
    expect(statuses).toEqual([
      {
        kind: "step-forward",
        state: "running",
        reducedMotion: false,
      },
      {
        kind: "step-forward",
        state: "settled",
        reducedMotion: false,
      },
    ]);
  });

  it("uses latest-projection-wins interruption with at most one pending frame", () => {
    const animationFrame = new FakeAnimationFrame();
    const motionPreference = new FakeMotionPreference();
    const poses: GameCameraPose[] = [];
    const statuses: DungeonCameraTweenStatus[] = [];
    const tween = new DungeonCameraTween(
      animationFrame,
      motionPreference,
      sampleCameraTransition,
    );
    const onError = (): never => {
      throw new Error("unexpected transition error");
    };

    tween.start(startOptions(forwardTransition(), poses, statuses, onError));
    const firstHandle = animationFrame.pendingHandles[0];
    tween.start(startOptions(turnTransition(), poses, statuses, onError));

    expect(animationFrame.cancelledHandles).toContain(firstHandle);
    expect(animationFrame.pendingCount).toBe(1);
    expect(poses.at(-1)?.yawDegrees).toBe(-90);
    animationFrame.flush(2_000);
    animationFrame.flush(2_110);
    expect(poses.at(-1)).toEqual(canonicalPose);
    expect(statuses.map((status) => status.kind)).toEqual([
      "step-forward",
      "turn-right",
      "turn-right",
    ]);
  });

  it("settles synchronously without requesting a frame under reduced motion", () => {
    const animationFrame = new FakeAnimationFrame();
    const motionPreference = new FakeMotionPreference();
    motionPreference.reduced = true;
    const poses: GameCameraPose[] = [];
    const statuses: DungeonCameraTweenStatus[] = [];
    const tween = new DungeonCameraTween(
      animationFrame,
      motionPreference,
      sampleCameraTransition,
    );

    tween.start(
      startOptions(forwardTransition(), poses, statuses, () => {
        throw new Error("unexpected transition error");
      }),
    );

    expect(poses).toEqual([canonicalPose]);
    expect(animationFrame.pendingCount).toBe(0);
    expect(statuses).toEqual([
      {
        kind: "step-forward",
        state: "settled",
        reducedMotion: true,
      },
    ]);
  });

  it("cancels its sole pending frame on disposal", () => {
    const animationFrame = new FakeAnimationFrame();
    const tween = new DungeonCameraTween(
      animationFrame,
      new FakeMotionPreference(),
      sampleCameraTransition,
    );
    tween.start(
      startOptions(forwardTransition(), [], [], () => {
        throw new Error("unexpected transition error");
      }),
    );
    const handle = animationFrame.pendingHandles[0];

    tween.dispose();

    expect(animationFrame.cancelledHandles).toContain(handle);
    expect(animationFrame.pendingCount).toBe(0);
    expect(tween.reapply(() => undefined)).toBe(false);
  });
});

function forwardTransition(): DungeonCameraTransition {
  return {
    kind: "step-forward",
    from: {
      ...canonicalPose,
      position: [0, 1.35, 2.55],
    },
    to: canonicalPose,
    durationMilliseconds: DUNGEON_CAMERA_TWEEN_DURATION_MS,
  };
}

function turnTransition(): DungeonCameraTransition {
  return {
    kind: "turn-right",
    from: { ...canonicalPose, yawDegrees: -90 },
    to: canonicalPose,
    durationMilliseconds: DUNGEON_CAMERA_TWEEN_DURATION_MS,
  };
}

function startOptions(
  transition: DungeonCameraTransition,
  poses: GameCameraPose[],
  statuses: DungeonCameraTweenStatus[],
  onError: (error: unknown) => void,
) {
  return {
    applyPose: (pose: GameCameraPose) => poses.push(pose),
    onError,
    onStatus: (status: DungeonCameraTweenStatus) => statuses.push(status),
    projection: { fovYDegrees: 58, near: 0.1, far: 64 },
    transition,
    viewport: { width: 1280, height: 720 },
  };
}

class FakeMotionPreference implements MotionPreferencePort {
  reduced = false;

  readonly prefersReducedMotion = (): boolean => this.reduced;
}

class FakeAnimationFrame implements AnimationFramePort {
  private readonly callbacks = new Map<number, FrameRequestCallback>();
  private nextHandle = 1;
  readonly cancelledHandles: number[] = [];

  readonly request = (callback: FrameRequestCallback): number => {
    const handle = this.nextHandle;
    this.nextHandle += 1;
    this.callbacks.set(handle, callback);
    return handle;
  };

  readonly cancel = (handle: number): void => {
    this.cancelledHandles.push(handle);
    this.callbacks.delete(handle);
  };

  get pendingCount(): number {
    return this.callbacks.size;
  }

  get pendingHandles(): readonly number[] {
    return [...this.callbacks.keys()];
  }

  flush(timestamp: number): void {
    const callbacks = [...this.callbacks.values()];
    this.callbacks.clear();
    for (const callback of callbacks) {
      callback(timestamp);
    }
  }
}
