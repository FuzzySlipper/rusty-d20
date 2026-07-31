import {
  afterNextRender,
  afterRenderEffect,
  ChangeDetectionStrategy,
  Component,
  type ElementRef,
  inject,
  Injector,
  input,
  viewChild,
} from "@angular/core";

/** View model for one log line. Local to the widget — no game types. */
export interface CombatLogEntryView {
  readonly id: number;
  readonly source: string;
  readonly text: string;
  readonly severity: "info" | "hit" | "miss" | "system";
  readonly details: readonly string[];
}

export interface CombatLogAutoFollowState {
  readonly latestEntryId: number | null;
  readonly shouldScroll: boolean;
}

export function resolveCombatLogAutoFollow(
  previousEntryId: number | null,
  entries: readonly CombatLogEntryView[],
): CombatLogAutoFollowState {
  const latestEntryId = entries.at(-1)?.id ?? null;
  return {
    latestEntryId,
    shouldScroll: latestEntryId !== null && latestEntryId !== previousEntryId,
  };
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

      .entries:focus-visible {
        outline: 2px solid var(--rusty-engine-accent);
        outline-offset: 3px;
      }

      .entry {
        font-size: 0.75rem;
        line-height: 1.35;
        overflow-wrap: anywhere;
        position: relative;
      }

      .entry__summary {
        background: transparent;
        border: 0;
        color: inherit;
        cursor: help;
        display: block;
        font: inherit;
        line-height: inherit;
        padding: 2px 0;
        text-align: left;
        width: 100%;
      }

      .entry__summary:focus-visible {
        border-radius: var(--rusty-engine-radius-sm);
        outline: 2px solid currentColor;
        outline-offset: 2px;
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

      .entry__details {
        background: rgb(6 11 14 / 0.96);
        border: 1px solid var(--rusty-engine-border);
        border-radius: var(--rusty-engine-radius-sm);
        color: var(--rusty-engine-text);
        display: grid;
        gap: 4px;
        margin: 2px 0 6px;
        max-height: 0;
        opacity: 0;
        overflow: hidden;
        padding: 0 8px;
      }

      .entry:hover .entry__details,
      .entry:focus-within .entry__details {
        max-height: 320px;
        opacity: 1;
        overflow-y: auto;
        padding: 8px;
      }

      .entry__details-title {
        color: var(--rusty-engine-accent);
        font-size: 0.68rem;
        font-weight: 700;
        letter-spacing: 0.06em;
        margin: 0;
        text-transform: uppercase;
      }

      .entry__details-list {
        display: grid;
        gap: 3px;
        margin: 0;
        padding-left: 18px;
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
        <ul
          #entriesList
          class="entries"
          aria-live="polite"
          aria-relevant="additions"
          tabindex="0"
        >
          @for (entry of entries(); track entry.id) {
            <li class="entry" [class]="'entry entry--' + entry.severity">
              @if (entry.details.length > 0) {
                <span
                  class="entry__summary"
                  role="group"
                  tabindex="0"
                  [attr.aria-describedby]="'combat-log-details-' + entry.id"
                  [attr.aria-label]="
                    entry.source +
                    ': ' +
                    entry.text +
                    '. Inspect rule resolution'
                  "
                  (focus)="revealEntryDetails(entry.id)"
                >
                  <span class="entry__source">[{{ entry.source }}]</span
                  >{{ entry.text }}
                </span>
                <section
                  class="entry__details"
                  [id]="'combat-log-details-' + entry.id"
                  [attr.aria-label]="entry.source + ' rule resolution'"
                >
                  <p class="entry__details-title">Rule resolution</p>
                  <ul class="entry__details-list">
                    @for (detail of entry.details; track detail) {
                      <li>{{ detail }}</li>
                    }
                  </ul>
                </section>
              } @else {
                <span class="entry__source">[{{ entry.source }}]</span
                >{{ entry.text }}
              }
            </li>
          }
        </ul>
      }
    </section>
  `,
})
export class CombatLogComponent {
  readonly entries = input.required<readonly CombatLogEntryView[]>();
  private readonly injector = inject(Injector);
  private readonly entriesList =
    viewChild<ElementRef<HTMLUListElement>>("entriesList");
  private latestEntryId: number | null = null;

  constructor() {
    afterRenderEffect(() => {
      const state = resolveCombatLogAutoFollow(
        this.latestEntryId,
        this.entries(),
      );
      this.latestEntryId = state.latestEntryId;
      const list = this.entriesList()?.nativeElement;
      if (state.shouldScroll && list !== undefined) {
        list.scrollTop = list.scrollHeight;
      }
    });
  }

  protected revealEntryDetails(entryId: number): void {
    afterNextRender(
      () => {
        const list = this.entriesList()?.nativeElement;
        const details = list?.querySelector<HTMLElement>(
          `#combat-log-details-${entryId}`,
        );
        const entry = details?.closest<HTMLElement>(".entry");
        if (list === undefined || entry === undefined || entry === null) {
          return;
        }
        list.scrollTop = entry.offsetTop;
      },
      { injector: this.injector },
    );
  }
}
