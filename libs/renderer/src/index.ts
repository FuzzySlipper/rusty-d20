import { ChangeDetectionStrategy, Component, input, output } from "@angular/core";
import { StatusLineComponent } from "@rusty-d20/components";
import type { RuntimeReadoutView } from "@rusty-d20/domain";

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

export interface TacticalCellCoordinate {
  readonly x: number;
  readonly y: number;
}

export interface TacticalBoardCellView {
  readonly id: string;
  readonly x: number;
  readonly y: number;
  readonly terrain: "floor" | "wall";
  readonly participantId: number | null;
  readonly participantName: string | null;
  readonly faction: "party" | "opposition" | null;
  readonly defeated: boolean;
  readonly current: boolean;
  readonly legalActionTarget: boolean;
  readonly legalMoveCost: number | null;
  readonly movementPreview: boolean;
  readonly route: readonly TacticalCellCoordinate[] | null;
}

export interface TacticalBoardView {
  readonly width: number;
  readonly height: number;
  readonly interactionMode: "movement" | "targeting" | "readonly";
  readonly targetingActionId: string | null;
  readonly targetingActionLabel: string | null;
  readonly cells: readonly TacticalBoardCellView[];
}

export interface TacticalBoardSelection {
  readonly x: number;
  readonly y: number;
  readonly participantId: number | null;
}

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

@Component({
  imports: [StatusLineComponent],
  selector: "aui-status-renderer",
  standalone: true,
  template: `<aui-status-line [label]="status().statusLabel" />`,
})
export class StatusRendererComponent {
  readonly status = input.required<RuntimeReadoutView>();
}

/**
 * Browser-shell marker for the separately mounted Engine-owned native renderer.
 *
 * The shell may describe the current Rust projection for accessibility and
 * layout, but it deliberately has no renderer package, canvas, input bridge,
 * resource ownership, or retained-scene authority.
 */
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
        pointer-events: auto;
        position: absolute;
      }

      .native-boundary {
        background:
          radial-gradient(circle at 50% 42%, rgb(30 61 64 / 0.3), transparent 48%),
          #090d12;
        box-shadow: inset 0 0 70px rgb(0 0 0 / 0.8);
        height: 100%;
        inset: 0;
        position: absolute;
        width: 100%;
      }
    `,
  ],
  template: `
    <section
      class="native-boundary"
      [attr.role]="view().mode === 'encounter' || view().mode === 'outcome' ? 'application' : 'img'"
      [attr.aria-label]="boardAriaLabel()"
      [attr.data-scene-mode]="view().mode"
      [attr.data-interaction-mode]="view().tactical?.interactionMode ?? null"
      [attr.tabindex]="view().mode === 'encounter' ? 0 : -1"
      data-renderer-boundary="native-engine-host"
      (click)="selectProjectedTarget($event)"
      (keydown)="handleKeydown($event)"
    ></section>
  `,
})
export class GameViewportComponent {
  readonly view = input.required<GameViewportView>();
  readonly sceneSelected = output<TacticalBoardSelection>();
  readonly sceneCancelled = output<void>();

  protected boardAriaLabel(): string {
    const tactical = this.view().tactical;
    if (tactical?.interactionMode === "targeting" && tactical.targetingActionLabel !== null) {
      return `Rendered tactical combat board. Targeting ${tactical.targetingActionLabel}.`;
    }
    if (tactical !== null) {
      return "Rendered tactical combat board.";
    }
    return this.view().label;
  }

  protected selectProjectedTarget(event?: MouseEvent): void {
    const tactical = this.view().tactical;
    let cell = tactical?.cells.find((entry) => entry.legalActionTarget || entry.legalMoveCost !== null);
    if (tactical !== null && tactical !== undefined && event !== undefined) {
      const bounds = (event.currentTarget as HTMLElement).getBoundingClientRect();
      const cellSize = 0.84;
      const fitWidth = tactical.width * cellSize;
      const fitHeight = tactical.height * cellSize;
      const aspect = bounds.width / bounds.height;
      const halfFovRadians = (58 * Math.PI) / 360;
      const distance = Math.max(
        fitHeight / (2 * Math.tan(halfFovRadians)),
        fitWidth / (2 * Math.tan(halfFovRadians) * aspect),
      ) * 1.12 + 0.8;
      const visibleHeight = 2 * distance * Math.tan(halfFovRadians);
      const x = Math.round(
        ((event.clientX - bounds.left - bounds.width / 2) / bounds.height) *
          visibleHeight /
          cellSize +
          (tactical.width - 1) / 2,
      );
      const y = Math.round(
        ((event.clientY - bounds.top - bounds.height / 2) / bounds.height) *
          visibleHeight /
          cellSize +
          (tactical.height - 1) / 2,
      );
      cell = tactical.cells.find((entry) => entry.x === x && entry.y === y);
    }
    if (cell !== undefined) {
      if (event !== undefined) {
        (event.currentTarget as HTMLElement).dataset["lastPickIdentity"] =
          `cell:${cell.x}:${cell.y}`;
      }
      this.sceneSelected.emit({ x: cell.x, y: cell.y, participantId: cell.participantId });
    }
  }

  protected handleKeydown(event: KeyboardEvent): void {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      this.selectProjectedTarget();
    } else if (event.key === "Escape") {
      event.preventDefault();
      this.sceneCancelled.emit();
    }
  }
}
