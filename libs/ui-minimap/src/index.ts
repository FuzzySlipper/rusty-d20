import { ChangeDetectionStrategy, Component, input } from '@angular/core';

/** View model for one minimap marker. Local to the widget — no game types. */
export interface MinimapMarkerView {
  readonly id: string;
  readonly label: string;
  readonly kind: 'quest' | 'enemy' | 'ally' | 'poi';
  /** Position on the map in percent, 0–100. */
  readonly x: number;
  readonly y: number;
}

/**
 * Minimap widget: a stylized map surface with positioned markers.
 * Purely presentational — markers are passed in as plain data.
 */
@Component({
  changeDetection: ChangeDetectionStrategy.OnPush,
  selector: 'aui-minimap',
  standalone: true,
  styles: [
    `
      :host {
        display: block;
        width: 180px;
      }

      .map {
        aspect-ratio: 1;
        background:
          radial-gradient(circle at 30% 40%, rgba(138, 220, 205, 0.08), transparent 55%),
          var(--rusty-engine-surface-solid);
        border: 1px solid var(--rusty-engine-border);
        border-radius: var(--rusty-engine-radius-sm);
        overflow: hidden;
        position: relative;
      }

      .marker {
        border-radius: 50%;
        height: 10px;
        position: absolute;
        transform: translate(-50%, -50%);
        width: 10px;
      }

      .marker--quest {
        background: var(--rusty-engine-warn);
      }

      .marker--enemy {
        background: var(--rusty-engine-danger);
      }

      .marker--ally {
        background: var(--rusty-engine-cool);
      }

      .marker--poi {
        background: var(--rusty-engine-accent);
      }

      .player {
        background: var(--rusty-engine-text);
        border: 2px solid var(--rusty-engine-accent);
        border-radius: 50%;
        height: 10px;
        position: absolute;
        transform: translate(-50%, -50%);
        width: 10px;
      }

      .region {
        color: var(--rusty-engine-muted);
        font-size: 0.7rem;
        margin: 0;
        text-align: center;
      }
    `,
  ],
  template: `
    <section class="rusty-engine-panel" aria-label="Minimap">
      <h2 class="rusty-engine-panel__title">{{ regionName() }}</h2>
      <div class="map" role="img" [attr.aria-label]="'Map of ' + regionName()">
        <span
          class="player"
          title="You"
          [style.left.%]="playerXPercent()"
          [style.top.%]="playerYPercent()"
        ></span>
        @for (marker of markers(); track marker.id) {
          <span
            class="marker"
            [class]="'marker marker--' + marker.kind"
            [style.left.%]="marker.x"
            [style.top.%]="marker.y"
            [title]="marker.label"
          ></span>
        }
      </div>
      <p class="region">{{ regionName() }}</p>
    </section>
  `,
})
export class MinimapComponent {
  readonly regionName = input.required<string>();
  readonly markers = input.required<readonly MinimapMarkerView[]>();
  readonly playerXPercent = input(50);
  readonly playerYPercent = input(50);
}
