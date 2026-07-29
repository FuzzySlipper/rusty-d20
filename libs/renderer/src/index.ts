import {
  ChangeDetectionStrategy,
  Component,
  computed,
  input,
  output,
} from "@angular/core";
import { StatusLineComponent } from "@rusty-d20/components";
import type { RuntimeReadoutView } from "@rusty-d20/domain";

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

export interface DungeonDepthView {
  readonly depth: number;
  readonly frontBlocked: boolean;
  readonly leftBlocked: boolean;
  readonly rightBlocked: boolean;
}

export interface DungeonViewportView {
  readonly title: string;
  readonly wallStyle: string;
  readonly facing: string;
  readonly x: number;
  readonly y: number;
  readonly depths: readonly DungeonDepthView[];
}

@Component({
  changeDetection: ChangeDetectionStrategy.OnPush,
  selector: "aui-dungeon-viewport",
  standalone: true,
  styles: [
    `
      :host {
        display: block;
      }

      .viewport {
        aspect-ratio: 16 / 9;
        background:
          linear-gradient(
            to bottom,
            #111923 0 48%,
            #332b22 48% 52%,
            #11100f 52%
          ),
          #090d12;
        border: 2px solid var(--rusty-engine-border);
        border-radius: var(--rusty-engine-radius);
        box-shadow: inset 0 0 70px rgb(0 0 0 / 0.8);
        min-height: 320px;
        overflow: hidden;
        position: relative;
      }

      .viewport::after {
        background: repeating-linear-gradient(
          0deg,
          transparent 0 3px,
          rgb(255 255 255 / 0.018) 3px 4px
        );
        content: "";
        inset: 0;
        pointer-events: none;
        position: absolute;
      }

      .depth {
        border-color: color-mix(
          in srgb,
          var(--rusty-engine-accent) 16%,
          #26313b
        );
        border-style: solid;
        border-width: 0;
        inset: calc(var(--depth) * 10%);
        position: absolute;
      }

      .depth--left {
        border-left-width: clamp(16px, 5vw, 70px);
      }

      .depth--right {
        border-right-width: clamp(16px, 5vw, 70px);
      }

      .depth--front {
        background:
          linear-gradient(135deg, rgb(255 255 255 / 0.04), transparent 44%),
          color-mix(in srgb, var(--rusty-engine-surface-solid) 88%, #413626);
        border-width: 1px;
        box-shadow: inset 0 0 34px rgb(0 0 0 / 0.55);
      }

      .reticle {
        color: var(--rusty-engine-accent);
        font-size: 1.3rem;
        left: 50%;
        position: absolute;
        text-shadow: 0 0 8px rgb(0 0 0);
        top: 50%;
        transform: translate(-50%, -50%);
        z-index: 2;
      }

      .caption {
        align-items: center;
        background: rgb(3 7 12 / 0.76);
        bottom: 0;
        display: flex;
        font-size: 0.72rem;
        gap: 10px;
        justify-content: space-between;
        left: 0;
        padding: 8px 10px;
        position: absolute;
        right: 0;
        z-index: 3;
      }
    `,
  ],
  template: `
    <section
      class="viewport"
      role="img"
      [attr.aria-label]="
        view().title +
        ', facing ' +
        view().facing +
        ' at cell ' +
        view().x +
        ', ' +
        view().y
      "
      [attr.data-wall-style]="view().wallStyle"
    >
      @for (depth of reversedDepths(); track depth.depth) {
        <span
          class="depth"
          [class.depth--left]="depth.leftBlocked"
          [class.depth--right]="depth.rightBlocked"
          [class.depth--front]="depth.frontBlocked"
          [style.--depth]="depth.depth"
        ></span>
      }
      <span class="reticle" aria-hidden="true">◇</span>
      <div class="caption">
        <strong>{{ view().title }}</strong>
        <span>Cell {{ view().x }},{{ view().y }} · {{ view().facing }}</span>
      </div>
    </section>
  `,
})
export class DungeonViewportComponent {
  readonly view = input.required<DungeonViewportView>();

  protected reversedDepths(): readonly DungeonDepthView[] {
    return [...this.view().depths].reverse();
  }
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
  readonly selectedTarget: boolean;
  readonly selectable: boolean;
  readonly legalMoveCost: number | null;
}

export interface TacticalBoardView {
  readonly width: number;
  readonly height: number;
  readonly cells: readonly TacticalBoardCellView[];
}

export interface TacticalBoardSelection {
  readonly x: number;
  readonly y: number;
  readonly participantId: number | null;
}

@Component({
  changeDetection: ChangeDetectionStrategy.OnPush,
  selector: "aui-tactical-board",
  standalone: true,
  styles: [
    `
      :host {
        display: block;
        max-width: 100%;
        min-width: 0;
      }

      .board-frame {
        background:
          radial-gradient(
            circle at 50% 40%,
            rgb(55 74 78 / 0.34),
            transparent 58%
          ),
          #070b0e;
        border: 2px solid var(--rusty-engine-border);
        border-radius: var(--rusty-engine-radius);
        box-sizing: border-box;
        box-shadow: inset 0 0 42px rgb(0 0 0 / 0.68);
        max-width: 100%;
        overflow-x: auto;
        padding: clamp(8px, 1.5vw, 14px);
        width: 100%;
      }

      .board {
        display: grid;
        gap: 2px;
        margin: 0 auto;
        min-width: 480px;
        width: min(100%, 780px);
      }

      .cell {
        align-items: center;
        aspect-ratio: 1;
        background: rgb(36 45 48 / 0.9);
        border: 1px solid rgb(121 143 142 / 0.2);
        border-radius: 2px;
        color: var(--rusty-engine-text);
        display: grid;
        font-size: clamp(0.62rem, 1.4vw, 0.88rem);
        min-height: 0;
        padding: 0;
        place-items: center;
        position: relative;
      }

      .cell--wall {
        background:
          linear-gradient(135deg, rgb(255 255 255 / 0.05), transparent 45%),
          #181c1c;
        border-color: #2b302e;
      }

      .cell--legal {
        background: color-mix(
          in srgb,
          var(--rusty-engine-cool) 18%,
          rgb(36 45 48 / 0.9)
        );
        border-color: var(--rusty-engine-cool);
        cursor: pointer;
      }

      .cell--party,
      .cell--opposition {
        border-width: 2px;
        font-weight: 900;
      }

      .cell--selectable {
        cursor: pointer;
      }

      .cell--party {
        background: color-mix(in srgb, var(--rusty-engine-cool) 38%, #162126);
        border-color: var(--rusty-engine-cool);
      }

      .cell--opposition {
        background: color-mix(in srgb, var(--rusty-engine-danger) 34%, #25191a);
        border-color: var(--rusty-engine-danger);
      }

      .cell--current {
        box-shadow:
          0 0 0 2px var(--rusty-engine-accent),
          0 0 16px
            color-mix(in srgb, var(--rusty-engine-accent) 75%, transparent);
        z-index: 2;
      }

      .cell--target {
        outline: 3px solid var(--rusty-engine-warn);
        outline-offset: -4px;
      }

      .cell--defeated {
        filter: grayscale(1);
        opacity: 0.42;
      }

      .token {
        line-height: 1;
        text-shadow: 0 1px 4px rgb(0 0 0 / 0.85);
      }

      .move-cost {
        bottom: 1px;
        color: var(--rusty-engine-cool);
        font-size: 0.52rem;
        position: absolute;
        right: 2px;
      }
    `,
  ],
  template: `
    <section
      class="board-frame"
      aria-label="Authoritative tactical combat board"
    >
      <div
        class="board"
        role="grid"
        [style.grid-template-columns]="'repeat(' + view().width + ', 1fr)'"
        [attr.aria-rowcount]="view().height"
        [attr.aria-colcount]="view().width"
      >
        @for (cell of cells(); track cell.id) {
          <button
            class="cell"
            type="button"
            role="gridcell"
            [class.cell--wall]="cell.terrain === 'wall'"
            [class.cell--legal]="cell.legalMoveCost !== null"
            [class.cell--party]="cell.faction === 'party'"
            [class.cell--opposition]="cell.faction === 'opposition'"
            [class.cell--current]="cell.current"
            [class.cell--target]="cell.selectedTarget"
            [class.cell--selectable]="cell.selectable"
            [class.cell--defeated]="cell.defeated"
            [disabled]="
              cell.terrain === 'wall' ||
              (cell.legalMoveCost === null && !cell.selectable)
            "
            [attr.aria-label]="cellLabel(cell)"
            [attr.data-x]="cell.x"
            [attr.data-y]="cell.y"
            [attr.data-participant-id]="cell.participantId"
            [attr.data-move-cost]="cell.legalMoveCost"
            (click)="select(cell)"
          >
            @if (cell.participantName !== null) {
              <span class="token" aria-hidden="true">{{
                token(cell.participantName)
              }}</span>
            }
            @if (cell.legalMoveCost !== null) {
              <span class="move-cost" aria-hidden="true">{{
                cell.legalMoveCost
              }}</span>
            }
          </button>
        }
      </div>
    </section>
  `,
})
export class TacticalBoardComponent {
  readonly view = input.required<TacticalBoardView>();
  readonly cellSelected = output<TacticalBoardSelection>();

  protected readonly cells = computed(() => this.view().cells);

  protected select(cell: TacticalBoardCellView): void {
    this.cellSelected.emit({
      x: cell.x,
      y: cell.y,
      participantId: cell.participantId,
    });
  }

  protected token(name: string): string {
    return name
      .split(/\s+/)
      .map((part) => part[0] ?? "")
      .join("")
      .slice(0, 2)
      .toUpperCase();
  }

  protected cellLabel(cell: TacticalBoardCellView): string {
    if (cell.terrain === "wall") {
      return `Wall at ${cell.x}, ${cell.y}`;
    }
    if (cell.participantName !== null) {
      return `${cell.participantName}, ${cell.faction}, at ${cell.x}, ${cell.y}${
        cell.current ? ", acting" : ""
      }${cell.defeated ? ", defeated" : ""}`;
    }
    return cell.legalMoveCost === null
      ? `Open terrain at ${cell.x}, ${cell.y}`
      : `Move to ${cell.x}, ${cell.y}, cost ${cell.legalMoveCost}`;
  }
}
