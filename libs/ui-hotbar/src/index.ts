import {
  ChangeDetectionStrategy,
  Component,
  input,
  output,
} from "@angular/core";

/** View model for one hotbar slot. Local to the widget — no game types. */
export interface HotbarSlotView {
  readonly index: number;
  readonly keybind: string;
  readonly label: string;
  readonly icon: string;
  readonly empty: boolean;
  readonly selected: boolean;
  readonly disabled: boolean;
}

/**
 * Hotbar widget: a row of activatable slots with keybind labels.
 * Emits `slotSelected` when a slot is activated; what "use" means is up to
 * the host screen.
 */
@Component({
  changeDetection: ChangeDetectionStrategy.OnPush,
  selector: "aui-hotbar",
  standalone: true,
  styles: [
    `
      :host {
        display: block;
      }

      .slots {
        display: flex;
        flex-wrap: wrap;
        gap: 6px;
        list-style: none;
        margin: 0;
        padding: 0;
      }

      .slot {
        align-items: center;
        aspect-ratio: 1;
        background: var(--rusty-engine-surface-solid);
        border: 1px solid var(--rusty-engine-border);
        border-radius: var(--rusty-engine-radius-sm);
        color: var(--rusty-engine-text);
        cursor: pointer;
        display: grid;
        font-size: 1.3rem;
        justify-items: center;
        padding: 0;
        position: relative;
        width: 52px;
      }

      .slot:hover {
        background: var(--rusty-engine-hover-bg);
        border-color: var(--rusty-engine-border-strong);
      }

      .slot--selected {
        background: var(--rusty-engine-accent-strong);
        border-color: var(--rusty-engine-accent);
        box-shadow: 0 0 0 2px rgb(58 221 203 / 0.2);
      }

      .slot--empty {
        cursor: default;
        opacity: 0.45;
      }

      .slot__keybind {
        bottom: 2px;
        color: var(--rusty-engine-muted);
        font-size: 0.62rem;
        position: absolute;
        right: 4px;
      }
    `,
  ],
  template: `
    <section class="rusty-engine-panel" aria-label="Hotbar">
      <h2 class="rusty-engine-panel__title">Hotbar</h2>
      <ul class="slots">
        @for (slot of slots(); track slot.index) {
          <li>
            <button
              class="slot"
              [class.slot--empty]="slot.empty"
              [class.slot--selected]="slot.selected"
              [disabled]="slot.empty || slot.disabled"
              [attr.aria-pressed]="slot.empty ? null : slot.selected"
              type="button"
              [attr.aria-label]="
                slot.empty ? 'Empty slot ' + slot.keybind : slot.label
              "
              [title]="slot.empty ? '' : slot.label"
              (click)="slotSelected.emit(slot)"
            >
              @if (!slot.empty) {
                <span aria-hidden="true">{{ slot.icon }}</span>
              }
              <span class="slot__keybind">{{ slot.keybind }}</span>
            </button>
          </li>
        }
      </ul>
    </section>
  `,
})
export class HotbarComponent {
  readonly slots = input.required<readonly HotbarSlotView[]>();
  readonly slotSelected = output<HotbarSlotView>();
}
