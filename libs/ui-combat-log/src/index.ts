import { ChangeDetectionStrategy, Component, input } from "@angular/core";

/** View model for one log line. Local to the widget — no game types. */
export interface CombatLogEntryView {
  readonly id: number;
  readonly source: string;
  readonly text: string;
  readonly severity: "info" | "hit" | "miss" | "system";
}

/**
 * Combat log widget: a scrolling list of log lines, newest at the bottom.
 * Purely presentational — the host screen owns the entry list.
 */
@Component({
  changeDetection: ChangeDetectionStrategy.OnPush,
  selector: "aui-combat-log",
  standalone: true,
  styles: [
    `
      :host {
        display: block;
        min-width: 0;
        width: min(320px, 100%);
      }

      .entries {
        display: flex;
        flex-direction: column;
        gap: 2px;
        list-style: none;
        margin: 0;
        max-height: 160px;
        overflow-y: auto;
        padding: 0;
      }

      .entry {
        font-size: 0.75rem;
        line-height: 1.35;
        overflow-wrap: anywhere;
      }

      .entry__source {
        font-weight: 700;
        margin-right: 0.35rem;
      }

      .entry--info {
        color: var(--rusty-engine-muted);
      }

      .entry--hit {
        color: var(--rusty-engine-accent);
      }

      .entry--miss {
        color: var(--rusty-engine-danger);
      }

      .entry--system {
        color: var(--rusty-engine-warn);
      }

      .empty {
        color: var(--rusty-engine-muted);
        font-size: 0.75rem;
        margin: 0;
      }
    `,
  ],
  template: `
    <section class="rusty-engine-panel" aria-label="Combat log">
      <h2 class="rusty-engine-panel__title">Combat Log</h2>
      @if (entries().length === 0) {
        <p class="empty">Nothing yet.</p>
      } @else {
        <ul class="entries">
          @for (entry of entries(); track entry.id) {
            <li class="entry" [class]="'entry entry--' + entry.severity">
              <span class="entry__source">[{{ entry.source }}]</span
              >{{ entry.text }}
            </li>
          }
        </ul>
      }
    </section>
  `,
})
export class CombatLogComponent {
  readonly entries = input.required<readonly CombatLogEntryView[]>();
}
