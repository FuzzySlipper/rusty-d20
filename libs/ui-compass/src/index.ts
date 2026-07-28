import { ChangeDetectionStrategy, Component, input } from '@angular/core';

/** View model for one compass marker. Local to the widget — no game types. */
export interface CompassMarkerView {
  readonly label: string;
  readonly bearingDegrees: number;
}

const CARDINALS: readonly { readonly label: string; readonly bearing: number }[] = [
  { label: 'N', bearing: 0 },
  { label: 'E', bearing: 90 },
  { label: 'S', bearing: 180 },
  { label: 'W', bearing: 270 },
];

/**
 * Compass widget: a horizontal bearing strip. Cardinal letters and markers
 * shift horizontally based on the player heading. Purely presentational.
 */
@Component({
  changeDetection: ChangeDetectionStrategy.OnPush,
  selector: 'aui-compass',
  standalone: true,
  styles: [
    `
      :host {
        display: block;
        width: 260px;
      }

      .strip {
        background: var(--rusty-engine-surface-solid);
        border: 1px solid var(--rusty-engine-border);
        border-radius: var(--rusty-engine-radius-sm);
        height: 30px;
        overflow: hidden;
        position: relative;
      }

      .tick {
        color: var(--rusty-engine-muted);
        font-size: 0.72rem;
        font-weight: 700;
        position: absolute;
        top: 50%;
        transform: translate(-50%, -50%);
      }

      .marker {
        color: var(--rusty-engine-warn);
        font-size: 0.72rem;
        position: absolute;
        top: 50%;
        transform: translate(-50%, -50%);
      }

      .center {
        background: var(--rusty-engine-accent);
        height: 100%;
        left: 50%;
        position: absolute;
        top: 0;
        width: 2px;
      }
    `,
  ],
  template: `
    <section class="rusty-engine-panel" aria-label="Compass">
      <div class="strip" role="img" [attr.aria-label]="'Heading ' + headingDegrees() + ' degrees'">
        <span class="center"></span>
        @for (tick of cardinalTicks; track tick.label) {
          @if (isVisible(tick.bearing)) {
            <span class="tick" [style.left.%]="offsetPercent(tick.bearing)">{{ tick.label }}</span>
          }
        }
        @for (marker of markers(); track marker.label + marker.bearingDegrees) {
          @if (isVisible(marker.bearingDegrees)) {
            <span class="marker" [style.left.%]="offsetPercent(marker.bearingDegrees)">{{ marker.label }}</span>
          }
        }
      </div>
    </section>
  `,
})
export class CompassComponent {
  /** Player heading in degrees, 0 = north. */
  readonly headingDegrees = input.required<number>();
  readonly markers = input.required<readonly CompassMarkerView[]>();

  protected readonly cardinalTicks = CARDINALS;

  /**
   * Signed shortest angular distance from the current heading, in degrees
   * (-180..180]. A bearing dead ahead is 0.
   */
  private relativeBearing(bearing: number): number {
    return ((((bearing - this.headingDegrees()) % 360) + 540) % 360) - 180;
  }

  /** Field of view: bearings within ±75° of the heading are shown. */
  protected isVisible(bearing: number): boolean {
    return Math.abs(this.relativeBearing(bearing)) <= 75;
  }

  /** Maps a bearing to a horizontal percent position (50% = dead ahead). */
  protected offsetPercent(bearing: number): number {
    return 50 + (this.relativeBearing(bearing) / 75) * 50;
  }
}
