import { NgTemplateOutlet } from '@angular/common';
import { ChangeDetectionStrategy, Component, computed, input, output, signal } from '@angular/core';

/**
 * View model for an equipped item, as shown inside an equipment slot.
 * Local to the widget — no game types.
 */
export interface EquippedItemView {
  readonly id: string;
  readonly name: string;
  readonly description: string;
  readonly icon: string;
  readonly rarity: 'common' | 'uncommon' | 'rare' | 'epic';
}

/** View model for one equipment slot. */
export interface EquipmentSlotView {
  readonly id: string;
  readonly label: string;
  readonly equipped: EquippedItemView | null;
}

/** Emitted when a dragged item is dropped onto an equipment slot. */
export interface EquipmentDropEvent {
  readonly slotId: string;
  readonly itemId: string;
}

/**
 * Default drag-and-drop contract for equippable items. Hosts coordinating
 * drags between an inventory grid and this panel should pass the same
 * `dragType` to every participant instead of relying on this default.
 */
export const DEFAULT_EQUIP_DRAG_TYPE = 'application/x-rusty-engine-inventory-item';

/**
 * Equipment widget: a vague paper-doll panel with gear slots flanking a
 * silhouette. Accepts HTML5 item drags and reports drops/selections — it
 * never equips or unequips anything itself; the host screen interprets the
 * events.
 */
@Component({
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [NgTemplateOutlet],
  selector: 'aui-equipment-panel',
  standalone: true,
  styles: [
    `
      :host {
        display: block;
      }

      .doll {
        align-items: stretch;
        display: grid;
        gap: 8px;
        grid-template-columns: 1fr auto 1fr;
      }

      .column {
        display: grid;
        gap: 8px;
        list-style: none;
        margin: 0;
        padding: 0;
      }

      .silhouette {
        align-items: center;
        background:
          radial-gradient(ellipse 55% 70% at 50% 42%, rgba(138, 220, 205, 0.12), transparent 70%),
          var(--rusty-engine-surface-solid);
        border: 1px dashed var(--rusty-engine-border);
        border-radius: var(--rusty-engine-radius);
        color: var(--rusty-engine-muted);
        display: grid;
        font-size: 2.6rem;
        justify-items: center;
        min-height: 180px;
        padding: 0.5rem;
      }

      .slot {
        align-items: center;
        background: var(--rusty-engine-surface-solid);
        border: 1px solid var(--rusty-engine-border);
        border-radius: var(--rusty-engine-radius-sm);
        color: var(--rusty-engine-text);
        cursor: default;
        display: grid;
        gap: 2px;
        justify-items: center;
        min-height: 56px;
        min-width: 72px;
        padding: 4px;
      }

      .slot--filled {
        cursor: pointer;
      }

      .slot--filled:hover {
        background: var(--rusty-engine-hover-bg);
      }

      .slot--drop-target {
        border-color: var(--rusty-engine-accent);
        box-shadow: 0 0 0 1px var(--rusty-engine-accent);
      }

      .slot--compatible {
        border-color: var(--rusty-engine-accent);
      }

      .slot--incompatible {
        filter: saturate(0.35);
        opacity: 0.58;
      }

      .slot--uncommon {
        border-color: var(--rusty-engine-accent);
      }

      .slot--rare {
        border-color: var(--rusty-engine-cool);
      }

      .slot--epic {
        border-color: var(--rusty-engine-warn);
      }

      .slot__icon {
        font-size: 1.2rem;
      }

      .slot__label {
        color: var(--rusty-engine-muted);
        font-size: 0.62rem;
        text-transform: uppercase;
      }

      .slot__name {
        font-size: 0.68rem;
        text-align: center;
      }

      .slot__compatibility {
        color: var(--rusty-engine-accent);
        font-size: 0.58rem;
        text-transform: uppercase;
      }

      .instructions {
        color: var(--rusty-engine-muted);
        font-size: 0.72rem;
        margin: 0 0 0.65rem;
      }
    `,
  ],
  template: `
    <section class="rusty-engine-panel" [attr.aria-label]="label()">
      <h2 class="rusty-engine-panel__title">{{ label() }}</h2>
      @if (instructions() !== "") {
        <p class="instructions">{{ instructions() }}</p>
      }
      <div class="doll">
        <ul class="column">
          @for (slot of leftSlots(); track slot.id) {
            <li>
              <ng-container *ngTemplateOutlet="slotTemplate; context: { $implicit: slot }" />
            </li>
          }
        </ul>
        <div class="silhouette" role="img" aria-label="Character silhouette">🧍</div>
        <ul class="column">
          @for (slot of rightSlots(); track slot.id) {
            <li>
              <ng-container *ngTemplateOutlet="slotTemplate; context: { $implicit: slot }" />
            </li>
          }
        </ul>
      </div>

      <ng-template #slotTemplate let-slot>
        <button
          type="button"
          [class]="slotClass(slot)"
          [attr.tabindex]="readOnly() ? -1 : 0"
          [attr.aria-disabled]="readOnly()"
          [attr.aria-label]="
            slot.equipped !== null
              ? slot.label + ': ' + slot.equipped.name + '. ' + slot.equipped.description
              : slot.label +
                ': empty' +
                (selectedItemSlotId() === slot.id ? '. Compatible destination' : '')
          "
          [title]="slot.equipped !== null ? slot.equipped.name : slot.label"
          [attr.draggable]="slot.equipped !== null && !readOnly() ? true : null"
          (dragstart)="
            slot.equipped !== null && !readOnly() && onDragStart($event, slot.equipped)
          "
          (dragend)="itemDragEnded.emit()"
          (dragover)="onDragOver($event, slot.id)"
          (dragleave)="onDragLeave(slot.id)"
          (drop)="onDrop($event, slot.id)"
          (click)="!readOnly() && activateSlot(slot)"
        >
          @if (slot.equipped !== null) {
            <span class="slot__icon" aria-hidden="true">{{ slot.equipped.icon }}</span>
            <span class="slot__name">{{ slot.equipped.name }}</span>
          } @else {
            <span class="slot__icon" aria-hidden="true">＋</span>
          }
          <span class="slot__label">{{ slot.label }}</span>
          @if (selectedItemSlotId() === slot.id) {
            <span class="slot__compatibility">Compatible</span>
          }
        </button>
      </ng-template>
    </section>
  `,
})
export class EquipmentPanelComponent {
  readonly slots = input.required<readonly EquipmentSlotView[]>();
  readonly label = input<string>('Equipment');
  readonly instructions = input<string>('');
  readonly dragType = input<string>(DEFAULT_EQUIP_DRAG_TYPE);
  readonly readOnly = input<boolean>(false);
  readonly selectedItemSlotId = input<string | null>(null);

  /** Reports an item drop on a slot; the host decides what "equip" means. */
  readonly itemDropped = output<EquipmentDropEvent>();
  readonly itemDragStarted = output<EquippedItemView>();
  readonly itemDragEnded = output<void>();
  readonly slotActivated = output<EquipmentSlotView>();
  /** Reports a click on a filled slot (e.g. to unequip). */
  readonly slotSelected = output<EquipmentSlotView>();

  protected readonly dragOverSlotId = signal<string | null>(null);

  /** Slots are split around the silhouette: first half left, second half right. */
  protected readonly leftSlots = computed(() =>
    this.slots().slice(0, Math.ceil(this.slots().length / 2)),
  );
  protected readonly rightSlots = computed(() =>
    this.slots().slice(Math.ceil(this.slots().length / 2)),
  );

  protected slotClass(slot: EquipmentSlotView): string {
    const classes = ['slot'];
    if (slot.equipped !== null) {
      classes.push('slot--filled', `slot--${slot.equipped.rarity}`);
    }
    if (this.dragOverSlotId() === slot.id) {
      classes.push('slot--drop-target');
    }
    const selectedSlot = this.selectedItemSlotId();
    if (selectedSlot !== null) {
      classes.push(
        selectedSlot === slot.id ? 'slot--compatible' : 'slot--incompatible',
      );
    }
    return classes.join(' ');
  }

  protected onDragOver(event: DragEvent, slotId: string): void {
    if (
      this.readOnly() ||
      (this.selectedItemSlotId() !== null && this.selectedItemSlotId() !== slotId) ||
      event.dataTransfer === null ||
      !event.dataTransfer.types.includes(this.dragType())
    ) {
      return;
    }
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
    this.dragOverSlotId.set(slotId);
  }

  protected onDragLeave(slotId: string): void {
    if (this.dragOverSlotId() === slotId) {
      this.dragOverSlotId.set(null);
    }
  }

  protected onDrop(event: DragEvent, slotId: string): void {
    if (
      this.readOnly() ||
      (this.selectedItemSlotId() !== null && this.selectedItemSlotId() !== slotId)
    ) {
      return;
    }
    event.preventDefault();
    const itemId = event.dataTransfer?.getData(this.dragType()) ?? '';
    this.dragOverSlotId.set(null);
    if (itemId === '') {
      return;
    }
    this.itemDropped.emit({ slotId, itemId });
  }

  protected onDragStart(event: DragEvent, item: EquippedItemView): void {
    if (event.dataTransfer !== null) {
      event.dataTransfer.setData(this.dragType(), item.id);
      event.dataTransfer.effectAllowed = 'move';
    }
    this.itemDragStarted.emit(item);
  }

  protected activateSlot(slot: EquipmentSlotView): void {
    this.slotActivated.emit(slot);
    if (slot.equipped !== null) {
      this.slotSelected.emit(slot);
    }
  }
}
