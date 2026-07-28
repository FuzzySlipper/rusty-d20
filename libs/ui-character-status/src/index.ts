import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';

/**
 * View model for the character-status widget.
 *
 * Defined locally on purpose: widgets never import the demo config or any
 * game-specific type, so a bootstrapped game can feed this widget from its
 * own state without touching the widget.
 */
export interface CharacterStatusView {
  readonly name: string;
  readonly level: number;
  readonly title: string;
  readonly health: { readonly current: number; readonly max: number };
  readonly resource: { readonly label: string; readonly current: number; readonly max: number };
  readonly buffs: readonly string[];
}

/**
 * Character status widget: name/level/title, health and resource bars, and
 * active buff chips. Purely presentational — no logic, no dependencies.
 */
@Component({
  changeDetection: ChangeDetectionStrategy.OnPush,
  selector: 'aui-character-status',
  standalone: true,
  styles: [
    `
      :host {
        display: block;
        min-width: 220px;
      }

      .identity {
        align-items: baseline;
        display: flex;
        gap: 0.5rem;
      }

      .name {
        font-weight: 700;
        margin: 0;
      }

      .level {
        color: var(--rusty-engine-accent);
        font-size: 0.8rem;
        font-weight: 700;
      }

      .title {
        color: var(--rusty-engine-muted);
        font-size: 0.75rem;
        margin: 0;
      }

      .bar {
        display: grid;
        gap: 2px;
      }

      .bar__track {
        background: var(--rusty-engine-bar-track);
        border-radius: var(--rusty-engine-radius-sm);
        height: 10px;
        overflow: hidden;
      }

      .bar__fill {
        height: 100%;
      }

      .bar__fill--health {
        background: var(--rusty-engine-bar-health);
      }

      .bar__fill--resource {
        background: var(--rusty-engine-bar-resource);
      }

      .bar__label {
        color: var(--rusty-engine-muted);
        display: flex;
        font-size: 0.7rem;
        justify-content: space-between;
      }

      .buffs {
        display: flex;
        flex-wrap: wrap;
        gap: 4px;
        list-style: none;
        margin: 0;
        padding: 0;
      }

      .buffs li {
        border: 1px solid var(--rusty-engine-border);
        border-radius: var(--rusty-engine-radius-sm);
        color: var(--rusty-engine-accent);
        font-size: 0.68rem;
        padding: 1px 6px;
      }
    `,
  ],
  template: `
    <section class="rusty-engine-panel" aria-label="Character status">
      <div class="identity">
        <p class="name">{{ status().name }}</p>
        <span class="level">Lv {{ status().level }}</span>
      </div>
      <p class="title">{{ status().title }}</p>

      <div class="bar" aria-label="Health">
        <div class="bar__track">
          <div class="bar__fill bar__fill--health" [style.width.%]="healthPercent()"></div>
        </div>
        <div class="bar__label">
          <span>HP</span>
          <span>{{ status().health.current }} / {{ status().health.max }}</span>
        </div>
      </div>

      <div class="bar" [attr.aria-label]="status().resource.label">
        <div class="bar__track">
          <div class="bar__fill bar__fill--resource" [style.width.%]="resourcePercent()"></div>
        </div>
        <div class="bar__label">
          <span>{{ status().resource.label }}</span>
          <span>{{ status().resource.current }} / {{ status().resource.max }}</span>
        </div>
      </div>

      @if (status().buffs.length > 0) {
        <ul class="buffs" aria-label="Active buffs">
          @for (buff of status().buffs; track buff) {
            <li>{{ buff }}</li>
          }
        </ul>
      }
    </section>
  `,
})
export class CharacterStatusComponent {
  readonly status = input.required<CharacterStatusView>();

  protected readonly healthPercent = computed(() => {
    const { current, max } = this.status().health;
    return max > 0 ? Math.round((current / max) * 100) : 0;
  });

  protected readonly resourcePercent = computed(() => {
    const { current, max } = this.status().resource;
    return max > 0 ? Math.round((current / max) * 100) : 0;
  });
}
