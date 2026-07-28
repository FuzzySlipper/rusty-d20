import { ChangeDetectionStrategy, Component, computed, input, output, signal } from '@angular/core';

/**
 * View model for one inventory item. Local to the widget — no game types,
 * so a bootstrapped game feeds this from its own item state.
 */
export interface InventoryItemView {
  readonly id: string;
  readonly name: string;
  readonly icon: string;
  readonly rarity: 'common' | 'uncommon' | 'rare' | 'epic';
  readonly quantity: number;
  /** Display hint only (marks the slot visually); equip logic lives in the host. */
  readonly equippable: boolean;
}

/** Emitted when an item is dropped onto another grid slot. */
export interface InventoryMoveEvent {
  readonly itemId: string;
  readonly fromIndex: number;
  readonly toIndex: number;
}

/**
 * Default drag-and-drop contract for inventory items. Hosts that coordinate
 * drags between this grid and other widgets (e.g. an equipment panel) should
 * pass the same `dragType` to every participant instead of relying on this
 * default, keeping the widgets decoupled from each other.
 */
export const DEFAULT_ITEM_DRAG_TYPE = 'application/x-rusty-engine-inventory-item';

/**
 * Inventory widget: a grid of slots with HTML5 drag & drop between slots.
 * Purely presentational — it never mutates the item list; it only reports
 * drags, drops, and activations for the host screen to interpret.
 */
@Component({
  changeDetection: ChangeDetectionStrategy.OnPush,
  selector: 'aui-inventory-grid',
  standalone: true,
  styles: [
    `
      :host {
        display: block;
      }

      .grid {
        display: grid;
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
        display: grid;
        font-size: 1.3rem;
        justify-items: center;
        min-width: 52px;
        padding: 0;
        position: relative;
      }

      .slot--occupied {
        cursor: grab;
      }

      .slot--occupied:hover {
        background: var(--rusty-engine-hover-bg);
      }

      .slot--drop-target {
        border-color: var(--rusty-engine-accent);
        box-shadow: 0 0 0 1px var(--rusty-engine-accent);
      }

      .slot--selected {
        background: var(--rusty-engine-hover-bg);
        box-shadow: 0 0 0 2px var(--rusty-engine-accent);
      }

      .slot--common {
        border-color: var(--rusty-engine-border);
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

      .slot__quantity {
        bottom: 2px;
        color: var(--rusty-engine-muted);
        font-size: 0.62rem;
        position: absolute;
        right: 4px;
      }

      .slot__equippable {
        color: var(--rusty-engine-accent);
        font-size: 0.55rem;
        left: 4px;
        position: absolute;
        top: 2px;
      }
    `,
  ],
  template: `
    <section class="rusty-engine-panel" aria-label="Inventory">
      <h2 class="rusty-engine-panel__title">Inventory</h2>
      <ul class="grid" [style.grid-template-columns]="gridTemplate()">
        @for (slot of slots(); track $index) {
          <li>
            <div
              class="slot"
              [class.slot--occupied]="slot !== null"
              [class.slot--drop-target]="dragOverIndex() === $index"
              [class]="slotClass(slot, $index)"
              role="button"
              [attr.tabindex]="slot !== null ? 0 : null"
              [attr.aria-pressed]="slot !== null ? selectedItemId() === slot.id : null"
              [attr.aria-label]="slot !== null ? slot.name : 'Empty slot ' + $index"
              [title]="slot !== null ? slot.name : ''"
              [attr.draggable]="slot !== null ? true : null"
              (dragstart)="slot !== null && onDragStart($event, $index, slot)"
              (dragend)="onDragEnd()"
              (dragover)="onDragOver($event, $index)"
              (dragleave)="onDragLeave($index)"
              (drop)="onDrop($event, $index)"
              (click)="slot !== null && itemActivated.emit(slot)"
              (keydown.enter)="slot !== null && itemActivated.emit(slot)"
              (keydown.space)="slot !== null && activateFromSpace($event, slot)"
            >
              @if (slot !== null) {
                @if (slot.equippable) {
                  <span class="slot__equippable" title="Equippable" aria-hidden="true">E</span>
                }
                <span aria-hidden="true">{{ slot.icon }}</span>
                @if (slot.quantity > 1) {
                  <span class="slot__quantity">{{ slot.quantity }}</span>
                }
              }
            </div>
          </li>
        }
      </ul>
    </section>
  `,
})
export class InventoryGridComponent {
  readonly columns = input.required<number>();
  readonly slots = input.required<readonly (InventoryItemView | null)[]>();
  readonly dragType = input<string>(DEFAULT_ITEM_DRAG_TYPE);
  readonly selectedItemId = input<string | null>(null);

  /** Reports a completed drag between grid slots; the host decides what it means. */
  readonly itemMoved = output<InventoryMoveEvent>();
  /** Reports the start of any item drag (e.g. so the host can track cross-panel drags). */
  readonly itemDragStarted = output<InventoryItemView>();
  readonly itemDragEnded = output<void>();
  /** Reports click, Enter, or Space activation of an occupied slot. */
  readonly itemActivated = output<InventoryItemView>();

  protected readonly dragOverIndex = signal<number | null>(null);
  private dragSourceIndex: number | null = null;

  protected readonly gridTemplate = computed(() => `repeat(${this.columns()}, minmax(0, 1fr))`);

  protected slotClass(slot: InventoryItemView | null, index: number): string {
    const classes = ['slot'];
    if (slot !== null) {
      classes.push('slot--occupied', `slot--${slot.rarity}`);
      if (this.selectedItemId() === slot.id) {
        classes.push('slot--selected');
      }
    }
    if (this.dragOverIndex() === index) {
      classes.push('slot--drop-target');
    }
    return classes.join(' ');
  }

  protected onDragStart(event: DragEvent, index: number, item: InventoryItemView): void {
    this.dragSourceIndex = index;
    if (event.dataTransfer !== null) {
      event.dataTransfer.setData(this.dragType(), item.id);
      event.dataTransfer.effectAllowed = 'move';
    }
    this.itemDragStarted.emit(item);
  }

  protected onDragOver(event: DragEvent, index: number): void {
    if (event.dataTransfer === null || !event.dataTransfer.types.includes(this.dragType())) {
      return;
    }
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
    this.dragOverIndex.set(index);
  }

  protected onDragLeave(index: number): void {
    if (this.dragOverIndex() === index) {
      this.dragOverIndex.set(null);
    }
  }

  protected onDrop(event: DragEvent, toIndex: number): void {
    event.preventDefault();
    const itemId = event.dataTransfer?.getData(this.dragType()) ?? '';
    const fromIndex = this.dragSourceIndex;
    this.resetDrag();
    if (itemId === '' || fromIndex === null || fromIndex === toIndex) {
      return;
    }
    this.itemMoved.emit({ itemId, fromIndex, toIndex });
  }

  protected onDragEnd(): void {
    this.resetDrag();
    this.itemDragEnded.emit();
  }

  protected activateFromSpace(event: Event, item: InventoryItemView): void {
    event.preventDefault();
    this.itemActivated.emit(item);
  }

  private resetDrag(): void {
    this.dragSourceIndex = null;
    this.dragOverIndex.set(null);
  }
}
