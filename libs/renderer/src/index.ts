import { ChangeDetectionStrategy, Component, input } from '@angular/core';
import { StatusLineComponent } from '@rusty-d20/components';
import type { RuntimeReadoutView } from '@rusty-d20/domain';

@Component({
  imports: [StatusLineComponent],
  selector: 'aui-status-renderer',
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
  selector: 'aui-dungeon-viewport',
  standalone: true,
  styles: [
    `
      :host {
        display: block;
      }

      .viewport {
        aspect-ratio: 16 / 9;
        background:
          linear-gradient(to bottom, #111923 0 48%, #332b22 48% 52%, #11100f 52%),
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
        content: '';
        inset: 0;
        pointer-events: none;
        position: absolute;
      }

      .depth {
        border-color: color-mix(in srgb, var(--rusty-engine-accent) 16%, #26313b);
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
