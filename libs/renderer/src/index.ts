import {
  ChangeDetectionStrategy,
  Component,
  input,
  output,
  signal,
  viewChild,
} from "@angular/core";
import type {
  AfterViewInit,
  ElementRef,
  OnChanges,
  OnDestroy,
} from "@angular/core";
import { StatusLineComponent } from "@rusty-d20/components";
import type { RuntimeReadoutView } from "@rusty-d20/domain";
import {
  browserAnimationFrame,
  browserDevicePixelRatio,
  browserElementResize,
  browserMotionPreference,
} from "@rusty-d20/platform";
import type { RendererSurface } from "@rusty-engine/renderer-host";

import {
  dungeonCameraTransition,
  DungeonCameraTween,
  type DungeonCameraTweenStatus,
} from "./dungeon-camera-tween";
import {
  createGameRenderFrame,
  tacticalCameraPose,
  type GameRenderFrame,
  type GameViewportView,
} from "./game-frame";
import {
  tacticalCellLabel,
  tacticalSelectionAt,
  type TacticalBoardSelection,
} from "./tactical-frame";

export {
  DUNGEON_CAMERA_TWEEN_DURATION_MS,
  dungeonCameraTransition,
  DungeonCameraTween,
  type CameraTransitionSampler,
  type DungeonCameraMotionKind,
  type DungeonCameraTransition,
  type DungeonCameraTweenStart,
  type DungeonCameraTweenStatus,
} from "./dungeon-camera-tween";
export {
  createDungeonRenderFrame,
  type DungeonDepthView,
  type DungeonRenderFrame,
  type DungeonViewportView,
} from "./dungeon-frame";
export {
  createGameRenderFrame,
  type GameCameraPose,
  type GameRenderFrame,
  type GameSceneMode,
  type GameViewportView,
} from "./game-frame";
export {
  createTacticalRenderFrame,
  tacticalCellLabel,
  tacticalSelectionAt,
  type TacticalBoardCellView,
  type TacticalBoardSelection,
  type TacticalBoardView,
  type TacticalCameraFit,
  type TacticalCellCoordinate,
  type TacticalRenderFrame,
  type TacticalScenePick,
} from "./tactical-frame";

@Component({
  imports: [StatusLineComponent],
  selector: "aui-status-renderer",
  standalone: true,
  styles: [
    `
      :host {
        display: block;
      }
    `,
  ],
  template: `<aui-status-line [label]="status().statusLabel" />`,
})
export class StatusRendererComponent {
  readonly status = input.required<RuntimeReadoutView>();
}

@Component({
  changeDetection: ChangeDetectionStrategy.OnPush,
  selector: "aui-game-viewport",
  standalone: true,
  styles: [
    `
      :host {
        display: block;
        inset: 0;
        min-width: 0;
        position: absolute;
      }

      .viewport {
        background: #090d12;
        box-shadow: inset 0 0 70px rgb(0 0 0 / 0.8);
        height: 100%;
        inset: 0;
        min-width: 0;
        overflow: hidden;
        position: absolute;
        width: 100%;
      }

      .viewport::after {
        background:
          linear-gradient(180deg, rgb(0 0 0 / 0.08), rgb(0 0 0 / 0.3)),
          repeating-linear-gradient(
            0deg,
            transparent 0 3px,
            rgb(255 255 255 / 0.014) 3px 4px
          );
        content: "";
        inset: 0;
        pointer-events: none;
        position: absolute;
      }

      .surface {
        box-sizing: border-box;
        display: block;
        height: 100%;
        inset: 0;
        max-width: 100%;
        min-width: 0;
        position: absolute;
        width: 100%;
      }

      .surface:focus-visible {
        outline: 3px solid var(--rusty-engine-accent);
        outline-offset: -5px;
      }

      .board-focus {
        backdrop-filter: blur(10px);
        background: rgb(6 11 14 / 0.82);
        border: 1px solid var(--rusty-engine-accent);
        border-radius: var(--rusty-engine-radius-sm);
        bottom: 18px;
        color: var(--rusty-engine-text);
        left: 50%;
        max-width: min(80%, 560px);
        opacity: 0;
        padding: 7px 10px;
        pointer-events: none;
        position: absolute;
        transform: translateX(-50%);
        transition: opacity 80ms linear;
        z-index: 3;
      }

      .surface:focus-visible ~ .board-focus {
        opacity: 1;
      }

      .reticle {
        color: var(--rusty-engine-accent);
        font-size: 1.3rem;
        left: 50%;
        pointer-events: none;
        position: absolute;
        text-shadow: 0 0 8px rgb(0 0 0);
        top: 50%;
        transform: translate(-50%, -50%);
        z-index: 2;
      }

      .renderer-error {
        background: rgb(62 14 16 / 0.94);
        border: 1px solid rgb(255 135 135 / 0.5);
        left: 50%;
        max-width: min(86%, 560px);
        padding: 12px 14px;
        position: absolute;
        top: 50%;
        transform: translate(-50%, -50%);
        z-index: 4;
      }
    `,
  ],
  template: `
    <section
      class="viewport"
      [attr.role]="
        view().mode === 'encounter' || view().mode === 'outcome'
          ? 'region'
          : 'img'
      "
      [attr.aria-label]="view().label"
      [attr.data-scene-mode]="view().mode"
      data-renderer-backend="rusty-engine-three"
    >
      <canvas
        #gameCanvas
        class="surface"
        [attr.aria-hidden]="
          view().mode === 'encounter' || view().mode === 'outcome'
            ? null
            : 'true'
        "
        [attr.aria-label]="
          view().mode === 'encounter' || view().mode === 'outcome'
            ? boardAriaLabel()
            : null
        "
        [attr.aria-roledescription]="
          view().mode === 'encounter' || view().mode === 'outcome'
            ? 'tactical combat board'
            : null
        "
        [attr.role]="
          view().mode === 'encounter' || view().mode === 'outcome'
            ? 'application'
            : null
        "
        [attr.data-interaction-mode]="view().tactical?.interactionMode ?? null"
        [attr.tabindex]="view().mode === 'encounter' ? 0 : -1"
        width="960"
        height="540"
        (click)="pickScene($event)"
        (keydown)="handleSceneKeydown($event)"
      ></canvas>
      @if (view().mode === "encounter") {
        <p class="board-focus" aria-live="polite">
          {{ keyboardCellLabel() }} · {{ boardKeyboardInstructions() }}
        </p>
      }
      @if (rendererError(); as message) {
        <p class="renderer-error" role="alert">{{ message }}</p>
      }
      @if (view().mode === "exploration") {
        <span class="reticle" aria-hidden="true">◇</span>
      }
    </section>
  `,
})
export class GameViewportComponent
  implements AfterViewInit, OnChanges, OnDestroy
{
  readonly view = input.required<GameViewportView>();
  readonly sceneSelected = output<TacticalBoardSelection>();
  readonly sceneCancelled = output<void>();
  protected readonly rendererError = signal<string | null>(null);
  protected readonly keyboardCellLabel = signal(
    "Focus the board to inspect its Rust-projected cells",
  );
  private readonly canvas =
    viewChild.required<ElementRef<HTMLCanvasElement>>("gameCanvas");
  private surface: RendererSurface | null = null;
  private activeHandles: GameRenderFrame["handles"] = [];
  private activeScene: GameRenderFrame | null = null;
  private activeView: GameViewportView | null = null;
  private cameraTween: DungeonCameraTween | null = null;
  private keyboardCell: readonly [number, number] | null = null;
  private keyboardInteractionKey: string | null = null;
  private stopResizeObservation: (() => void) | null = null;
  private destroyed = false;

  async ngAfterViewInit(): Promise<void> {
    try {
      const { mountRendererSurface, sampleCameraTransition } = await import(
        "@rusty-engine/renderer-host"
      );
      if (this.destroyed) {
        return;
      }
      const initialView = this.view();
      const scene = createGameRenderFrame(initialView);
      this.cameraTween = new DungeonCameraTween(
        browserAnimationFrame,
        browserMotionPreference,
        sampleCameraTransition,
      );
      this.surface = mountRendererSurface(this.canvas().nativeElement, {
        autoStart: true,
        clearColor: 0x070b0e,
        frame: scene.frame,
        pixelRatio: browserDevicePixelRatio(),
        projection: { fovYDegrees: 58, near: 0.1, far: 64 },
      });
      this.activeScene = scene;
      this.activeView = initialView;
      this.syncKeyboardCell();
      this.applyCameraForSize(
        this.canvas().nativeElement.clientWidth,
        this.canvas().nativeElement.clientHeight,
      );
      this.surface.renderOnce();
      this.activeHandles = scene.handles;
      this.stopResizeObservation = browserElementResize.observe(
        this.canvas().nativeElement,
        ({ width, height }) => this.applyCameraForSize(width, height),
      );
      this.rendererError.set(null);
    } catch (error) {
      this.rendererError.set(rendererFailureMessage(error));
    }
  }

  ngOnChanges(): void {
    if (this.surface === null) {
      return;
    }
    try {
      const previousView = this.activeView;
      const nextView = this.view();
      const scene = createGameRenderFrame(nextView, this.activeHandles);
      this.surface.applyFrame(scene.frame);
      this.activeScene = scene;
      this.activeView = nextView;
      this.syncKeyboardCell();
      this.applyCameraForProjectionChange(
        previousView,
        nextView,
        this.canvas().nativeElement.clientWidth,
        this.canvas().nativeElement.clientHeight,
      );
      this.surface.renderOnce();
      this.activeHandles = scene.handles;
      this.rendererError.set(null);
    } catch (error) {
      this.rendererError.set(rendererFailureMessage(error));
    }
  }

  ngOnDestroy(): void {
    this.destroyed = true;
    this.stopResizeObservation?.();
    this.stopResizeObservation = null;
    this.cameraTween?.dispose();
    this.cameraTween = null;
    this.surface?.dispose();
    this.surface = null;
    this.activeHandles = [];
    this.activeScene = null;
    this.activeView = null;
  }

  protected pickScene(event: MouseEvent): void {
    if (
      this.surface === null ||
      (this.view().mode !== "encounter" && this.view().mode !== "outcome")
    ) {
      return;
    }
    const bounds = this.canvas().nativeElement.getBoundingClientRect();
    if (bounds.width <= 0 || bounds.height <= 0) {
      return;
    }
    const receipt = this.surface.pick({
      filter: { tags: ["tactical-pickable"] },
      ray: {
        kind: "viewport",
        point: [
          ((event.clientX - bounds.left) / bounds.width) * 2 - 1,
          1 - ((event.clientY - bounds.top) / bounds.height) * 2,
        ],
      },
    });
    const pick = this.activeScene?.picks.find(
      (entry) => entry.handle === receipt.hint?.handle,
    );
    if (pick !== undefined) {
      this.keyboardCell = [pick.selection.x, pick.selection.y];
      this.keyboardCellLabel.set(pick.label);
      this.canvas().nativeElement.dataset["lastPickIdentity"] = pick.identity;
      this.sceneSelected.emit(pick.selection);
    }
  }

  protected handleSceneKeydown(event: KeyboardEvent): void {
    const tactical = this.view().tactical;
    if (this.view().mode !== "encounter" || tactical === null) {
      return;
    }
    this.syncKeyboardCell();
    const current = this.keyboardCell;
    if (current === null) {
      return;
    }
    let [x, y] = current;
    if (event.key === "ArrowLeft") {
      x -= 1;
    } else if (event.key === "ArrowRight") {
      x += 1;
    } else if (event.key === "ArrowUp") {
      y -= 1;
    } else if (event.key === "ArrowDown") {
      y += 1;
    } else if (event.key === "Enter" || event.key === " ") {
      const selection = tacticalSelectionAt(tactical, x, y);
      if (selection !== null) {
        event.preventDefault();
        this.sceneSelected.emit(selection);
      }
      return;
    } else if (
      event.key === "Escape" &&
      tactical.interactionMode !== "readonly"
    ) {
      event.preventDefault();
      this.sceneCancelled.emit();
      return;
    } else {
      return;
    }
    event.preventDefault();
    x = Math.max(0, Math.min(tactical.width - 1, x));
    y = Math.max(0, Math.min(tactical.height - 1, y));
    this.keyboardCell = [x, y];
    this.keyboardCellLabel.set(
      tacticalCellLabel(tactical, x, y) ?? "Unknown tactical cell",
    );
  }

  private syncKeyboardCell(): void {
    const tactical = this.view().tactical;
    if (tactical === null) {
      this.keyboardCell = null;
      this.keyboardInteractionKey = null;
      this.keyboardCellLabel.set(
        "Focus the board to inspect its Rust-projected cells",
      );
      return;
    }
    const interactionKey = `${tactical.interactionMode}:${
      tactical.targetingActionId ?? ""
    }`;
    if (interactionKey !== this.keyboardInteractionKey) {
      this.keyboardCell = null;
      this.keyboardInteractionKey = interactionKey;
    }
    if (
      this.keyboardCell !== null &&
      tacticalSelectionAt(
        tactical,
        this.keyboardCell[0],
        this.keyboardCell[1],
      ) !== null
    ) {
      this.keyboardCellLabel.set(
        tacticalCellLabel(
          tactical,
          this.keyboardCell[0],
          this.keyboardCell[1],
        ) ?? "Unknown tactical cell",
      );
      return;
    }
    const initial =
      tactical.cells.find((cell) => cell.legalActionTarget) ??
      tactical.cells.find((cell) => cell.current) ??
      tactical.cells.find((cell) => cell.legalMoveCost !== null) ??
      tactical.cells.find((cell) => cell.terrain === "floor");
    if (initial !== undefined) {
      this.keyboardCell = [initial.x, initial.y];
      this.keyboardCellLabel.set(
        tacticalCellLabel(tactical, initial.x, initial.y) ??
          "Unknown tactical cell",
      );
    }
  }

  protected boardAriaLabel(): string {
    const tactical = this.view().tactical;
    if (
      tactical?.interactionMode === "targeting" &&
      tactical.targetingActionLabel !== null
    ) {
      return `Rendered tactical combat board. Targeting ${tactical.targetingActionLabel}. Use arrow keys to inspect cells, Enter to choose a legal target, and Escape to cancel.`;
    }
    if (tactical?.interactionMode === "movement") {
      const preview = tactical.cells.find((cell) => cell.movementPreview);
      return preview === undefined
        ? "Rendered tactical combat board. Movement selected. Use arrow keys to inspect Rust-projected destinations, Enter to preview a route, and Escape to cancel."
        : `Rendered tactical combat board. Previewing movement to ${preview.x}, ${preview.y}. Press Enter on the same destination to confirm, choose another destination to replace the preview, or Escape to cancel.`;
    }
    return "Rendered tactical combat board. Choose Move or an action from the hotbar, then use arrow keys to inspect cells.";
  }

  protected boardKeyboardInstructions(): string {
    const mode = this.view().tactical?.interactionMode;
    return mode === "targeting"
      ? "Arrow keys inspect · Enter targets · Escape cancels"
      : mode === "movement"
        ? "Arrow keys inspect · Enter previews/confirms · Escape cancels"
        : "Choose Move or an action from the hotbar";
  }

  private applyCameraForSize(width: number, height: number): void {
    if (this.surface === null || this.activeScene === null) {
      return;
    }
    const fit = this.activeScene.cameraFit;
    if (
      fit === null &&
      this.activeView?.mode === "exploration" &&
      this.cameraTween?.reapply((pose) => this.setSurfaceCameraPose(pose))
    ) {
      return;
    }
    const pose =
      fit === null
        ? this.activeScene.camera
        : tacticalCameraPose(fit, width > 0 && height > 0 ? width / height : 1);
    this.settleCamera(pose);
  }

  private applyCameraForProjectionChange(
    previousView: GameViewportView | null,
    nextView: GameViewportView,
    width: number,
    height: number,
  ): void {
    if (this.surface === null || this.activeScene === null) {
      return;
    }
    const fit = this.activeScene.cameraFit;
    const canonicalPose =
      fit === null
        ? this.activeScene.camera
        : tacticalCameraPose(fit, width > 0 && height > 0 ? width / height : 1);
    const previousDungeon =
      previousView?.mode === "exploration" ? previousView.dungeon : null;
    const nextDungeon =
      nextView.mode === "exploration" ? nextView.dungeon : null;
    const transition =
      previousDungeon === null || nextDungeon === null
        ? null
        : dungeonCameraTransition(previousDungeon, nextDungeon, canonicalPose);
    if (transition === null || this.cameraTween === null || fit !== null) {
      this.settleCamera(canonicalPose);
      return;
    }
    this.cameraTween.start({
      applyPose: (pose) => this.setSurfaceCameraPose(pose),
      onError: (error) => {
        this.setCameraTransitionStatus(
          transition.kind,
          "settled",
          browserMotionPreference.prefersReducedMotion(),
        );
        this.rendererError.set(rendererFailureMessage(error));
      },
      onStatus: (status) =>
        this.setCameraTransitionStatus(
          status.kind,
          status.state,
          status.reducedMotion,
        ),
      projection: this.surface.cameraProjection(),
      transition,
      viewport: {
        width: Math.max(1, width),
        height: Math.max(1, height),
      },
    });
  }

  private settleCamera(pose: GameRenderFrame["camera"]): void {
    if (this.cameraTween === null) {
      this.setSurfaceCameraPose(pose);
    } else {
      this.cameraTween.settle(pose, (nextPose) =>
        this.setSurfaceCameraPose(nextPose),
      );
    }
    this.setCameraTransitionStatus(
      null,
      "settled",
      browserMotionPreference.prefersReducedMotion(),
    );
  }

  private setSurfaceCameraPose(pose: GameRenderFrame["camera"]): void {
    if (this.surface === null) {
      return;
    }
    this.surface.setCameraPose(pose);
    this.canvas().nativeElement.dataset["cameraPose"] = [
      ...pose.position,
      pose.yawDegrees,
      pose.pitchDegrees,
    ]
      .map((value) => value.toFixed(3))
      .join(",");
  }

  private setCameraTransitionStatus(
    kind: DungeonCameraTweenStatus["kind"],
    state: DungeonCameraTweenStatus["state"],
    reducedMotion: boolean,
  ): void {
    const dataset = this.canvas().nativeElement.dataset;
    dataset["cameraMotion"] = kind ?? "none";
    dataset["cameraTransitionState"] = state;
    dataset["cameraReducedMotion"] = reducedMotion ? "true" : "false";
  }
}

function rendererFailureMessage(error: unknown): string {
  const detail = error instanceof Error ? error.message : String(error);
  return `The Rusty Engine game renderer could not present this scene: ${detail}`;
}
