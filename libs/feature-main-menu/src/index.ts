import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
  signal,
} from '@angular/core';
import type { OnInit } from '@angular/core';
import type { CharacterDto } from '@rusty-d20/protocol';
import { SessionStore } from '@rusty-d20/store';
import {
  CharacterStatusComponent,
  type CharacterStatusView,
} from '@rusty-d20/ui-character-status';
import {
  CombatLogComponent,
  type CombatLogEntryView,
} from '@rusty-d20/ui-combat-log';
import { HotbarComponent, type HotbarSlotView } from '@rusty-d20/ui-hotbar';

@Component({
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [CharacterStatusComponent, CombatLogComponent, HotbarComponent],
  selector: 'aui-main-menu-screen',
  standalone: true,
  styles: [
    `
      :host {
        display: block;
        min-height: 100vh;
      }

      .game-shell {
        display: grid;
        gap: 18px;
        margin: 0 auto;
        max-width: 1180px;
        min-height: 100vh;
        padding: clamp(16px, 3vw, 32px);
      }

      .topbar,
      .topbar__identity,
      .topbar__controls,
      .encounter-meta,
      .actions__header,
      .reaction-list {
        align-items: center;
        display: flex;
        flex-wrap: wrap;
        gap: 10px;
      }

      .topbar {
        justify-content: space-between;
      }

      .mark {
        background: linear-gradient(145deg, var(--rusty-engine-accent), var(--rusty-engine-cool));
        border-radius: 10px;
        color: var(--rusty-engine-bg);
        display: grid;
        font-size: 1.3rem;
        font-weight: 900;
        height: 44px;
        place-items: center;
        width: 44px;
      }

      .eyebrow,
      .meta-label {
        color: var(--rusty-engine-accent);
        font-size: 0.7rem;
        font-weight: 700;
        letter-spacing: 0.1em;
        margin: 0;
        text-transform: uppercase;
      }

      h1,
      h2,
      h3,
      p {
        margin: 0;
      }

      h1 {
        font-size: 1.45rem;
      }

      button,
      select {
        background: var(--rusty-engine-surface-solid);
        border: 1px solid var(--rusty-engine-border);
        border-radius: var(--rusty-engine-radius-sm);
        color: var(--rusty-engine-text);
        min-height: 38px;
        padding: 7px 12px;
      }

      button {
        cursor: pointer;
      }

      button:hover:not(:disabled) {
        background: var(--rusty-engine-hover-bg);
        border-color: var(--rusty-engine-accent);
      }

      button:disabled {
        cursor: wait;
        opacity: 0.5;
      }

      .primary {
        background: var(--rusty-engine-accent-strong);
        border-color: var(--rusty-engine-accent);
      }

      .empty,
      .fatal {
        align-self: center;
        justify-self: center;
        max-width: 680px;
        padding: clamp(24px, 7vw, 64px);
        text-align: center;
      }

      .empty {
        gap: 18px;
      }

      .empty h2 {
        font-size: clamp(2rem, 8vw, 4.5rem);
        line-height: 0.95;
      }

      .lede,
      .muted {
        color: var(--rusty-engine-muted);
      }

      .empty button {
        justify-self: center;
      }

      .encounter-meta {
        color: var(--rusty-engine-muted);
        font-size: 0.78rem;
      }

      .encounter-meta code {
        color: var(--rusty-engine-cool);
      }

      .characters {
        display: grid;
        gap: 14px;
        grid-template-columns: repeat(2, minmax(0, 1fr));
      }

      .character-card {
        display: grid;
        gap: 8px;
      }

      .resources {
        display: flex;
        flex-wrap: wrap;
        gap: 6px;
      }

      .resource-chip {
        background: rgba(255, 255, 255, 0.05);
        border: 1px solid var(--rusty-engine-border);
        border-radius: 999px;
        color: var(--rusty-engine-muted);
        font-size: 0.7rem;
        padding: 3px 8px;
      }

      .workspace {
        display: grid;
        gap: 14px;
        grid-template-columns: minmax(0, 1.45fr) minmax(280px, 0.75fr);
      }

      .action-workbench,
      .outcome {
        align-content: start;
        display: grid;
        gap: 14px;
      }

      .actions__header {
        justify-content: space-between;
      }

      .target-control {
        align-items: center;
        display: flex;
        gap: 8px;
      }

      .target-control label {
        color: var(--rusty-engine-muted);
        font-size: 0.75rem;
        font-weight: 700;
      }

      .action-catalog {
        display: grid;
        gap: 8px;
        grid-template-columns: repeat(2, minmax(0, 1fr));
      }

      .action-note {
        border-left: 2px solid var(--rusty-engine-border);
        padding-left: 10px;
      }

      .action-note strong {
        display: block;
        font-size: 0.82rem;
      }

      .action-note span {
        color: var(--rusty-engine-muted);
        font-size: 0.72rem;
      }

      .preview {
        border-color: var(--rusty-engine-accent);
      }

      .preview__math {
        color: var(--rusty-engine-accent);
        font-size: 1.05rem;
        font-weight: 700;
      }

      .source-list,
      .detail-list {
        color: var(--rusty-engine-muted);
        font-size: 0.75rem;
        margin: 0;
        padding-left: 18px;
      }

      .reaction {
        border-color: var(--rusty-engine-warn);
      }

      .resolve {
        justify-self: start;
      }

      .command-error {
        background: var(--rusty-engine-danger-bg);
        border: 1px solid var(--rusty-engine-danger);
        border-radius: var(--rusty-engine-radius);
        display: grid;
        gap: 8px;
        padding: 12px 14px;
      }

      .command-error strong {
        color: var(--rusty-engine-danger);
        text-transform: capitalize;
      }

      .command-error__actions {
        display: flex;
        gap: 8px;
      }

      .save-state {
        color: var(--rusty-engine-muted);
        font-size: 0.72rem;
      }

      .save-state--saved {
        color: var(--rusty-engine-accent);
      }

      .latest {
        border-top: 1px solid var(--rusty-engine-border);
        display: grid;
        gap: 8px;
        padding-top: 12px;
      }

      @media (max-width: 760px) {
        .characters,
        .workspace,
        .action-catalog {
          grid-template-columns: minmax(0, 1fr);
        }

        .topbar {
          align-items: flex-start;
        }

        .topbar__controls {
          width: 100%;
        }

        .topbar__controls button {
          flex: 1 1 auto;
        }
      }

      @media (max-width: 420px) {
        .game-shell {
          padding: 12px;
        }

        .actions__header {
          align-items: stretch;
        }

        .target-control,
        .target-control select {
          width: 100%;
        }
      }
    `,
  ],
  template: `
    <main class="game-shell">
      <header class="topbar">
        <div class="topbar__identity">
          <div class="mark" aria-hidden="true">D20</div>
          <div>
            <p class="eyebrow">Rust-owned encounter</p>
            <h1>Rusty D20</h1>
          </div>
        </div>

        @if (game(); as snapshot) {
          @if (snapshot.encounter !== null) {
            <div class="topbar__controls">
              <span
                class="save-state"
                [class.save-state--saved]="snapshot.saved"
                aria-live="polite"
              >
                {{ snapshot.saved ? 'Saved' : 'Unsaved changes' }}
              </span>
              <button type="button" [disabled]="store.busy()" (click)="save()">Save</button>
              <button type="button" [disabled]="store.busy()" (click)="advanceTurn()">
                Advance turn
              </button>
            </div>
          }
        }
      </header>

      @switch (store.session().kind) {
        @case ('idle') {
          <section class="rusty-engine-panel empty" aria-live="polite">
            <p>Preparing the authoritative session…</p>
          </section>
        }
        @case ('loading') {
          <section class="rusty-engine-panel empty" aria-live="polite" aria-busy="true">
            <p>Loading authored rules and Rust state…</p>
          </section>
        }
        @case ('error') {
          <section class="rusty-engine-panel fatal" role="alert">
            <p class="eyebrow">{{ sessionError().kind }} failure</p>
            <h2>Could not reach the game runtime</h2>
            <p>{{ sessionError().message }}</p>
            @if (sessionError().retryable) {
              <button class="primary" type="button" (click)="reload()">Retry connection</button>
            }
          </section>
        }
        @case ('data') {
          @if (game()?.encounter === null) {
            <section class="rusty-engine-panel empty" aria-label="New encounter">
              <p class="eyebrow">Starter Core · Steel Guard</p>
              <h2>The Warden's Gate</h2>
              <p class="lede">
                Begin a compact authored encounter. Rust compiles the rules package, owns every
                roll and mutation, and records the Engine sources that shaped the outcome.
              </p>
              <button
                class="primary"
                type="button"
                [disabled]="store.busy()"
                (click)="startEncounter()"
              >
                Start encounter
              </button>
              <p class="muted">
                Engine {{ game()?.engineRevisionShort }} · rules
                <span [title]="game()?.rulesetFingerprint">Starter + Steel</span>
              </p>
            </section>
          } @else {
            <section class="encounter-meta" aria-label="Encounter identity">
              <span>Turn {{ encounter().turn }}</span>
              <span>Next deterministic roll {{ encounter().nextRoll }}</span>
              <span>State revision {{ game()?.revision }}</span>
              <span>Engine <code>{{ game()?.engineRevisionShort }}</code></span>
              <span>
                Rules
                <code [title]="game()?.rulesetFingerprint">Starter + Steel</code>
              </span>
            </section>

            @if (store.commandError(); as error) {
              <section class="command-error" role="alert">
                <strong>{{ error.kind }} rejection</strong>
                <span>{{ error.message }}</span>
                <div class="command-error__actions">
                  @if (error.retryable) {
                    <button type="button" (click)="reload()">Reload current state</button>
                  }
                  <button type="button" (click)="dismissError()">Dismiss</button>
                </div>
              </section>
            }

            <section class="characters" aria-label="Character status">
              @for (character of encounter().characters; track character.id) {
                <article class="character-card">
                  <aui-character-status [status]="characterStatus(character)" />
                  <div class="resources" [attr.aria-label]="character.name + ' resources'">
                    @for (resource of character.resources; track resource.id) {
                      <span class="resource-chip">
                        {{ resource.label }} {{ resource.current }}/{{ resource.maximum }}
                      </span>
                    }
                  </div>
                </article>
              }
            </section>

            <section class="workspace">
              <div class="action-workbench">
                <section class="rusty-engine-panel">
                  <header class="actions__header">
                    <div>
                      <p class="meta-label">Authored actions</p>
                      <h2>Choose an action</h2>
                    </div>
                    <div class="target-control">
                      <label for="target">Target</label>
                      <select id="target" [value]="targetId()" (change)="selectTarget($event)">
                        @for (target of encounter().targets; track target.id) {
                          <option [value]="target.id">{{ target.name }}</option>
                        }
                      </select>
                    </div>
                  </header>

                  <aui-hotbar
                    [slots]="hotbarSlots()"
                    (slotSelected)="chooseAction($event)"
                  />

                  <div class="action-catalog">
                    @for (action of encounter().actions; track action.id) {
                      <div class="action-note">
                        <strong>{{ action.label }}</strong>
                        <span>
                          {{ action.ability }} vs {{ action.defense }} · {{ action.damage }}
                          @if (action.effect !== null) {
                            · {{ action.effect }}
                          }
                        </span>
                      </div>
                    }
                  </div>
                </section>

                @if (encounter().pendingAction; as pending) {
                  <section class="rusty-engine-panel preview" aria-label="Authoritative action preview">
                    <p class="meta-label">Rust preview · {{ pending.actionLabel }}</p>
                    <p class="preview__math">
                      Ability {{ pending.abilityScore }} ({{ signed(pending.abilityModifier) }})
                      against defense {{ pending.defense }}
                    </p>
                    <div>
                      <h3>Defense attribution</h3>
                      <ul class="source-list">
                        @for (source of pending.defenseSources; track source) {
                          <li>{{ source }}</li>
                        }
                      </ul>
                    </div>

                    @if (pending.reactions.length > 0) {
                      <div class="reaction-list" aria-label="Available reactions">
                        @for (reaction of pending.reactions; track reaction.id) {
                          <button
                            class="reaction"
                            type="button"
                            [disabled]="store.busy()"
                            (click)="applyReaction(pending.token, reaction.id)"
                          >
                            {{ reaction.label }} · {{ reaction.cost }} {{ reaction.resource }} ·
                            {{ signed(reaction.bonus) }} defense
                          </button>
                        }
                      </div>
                    }

                    <button
                      class="primary resolve"
                      type="button"
                      [disabled]="store.busy()"
                      (click)="resolveAction(pending.token)"
                    >
                      Resolve deterministic roll
                    </button>
                  </section>
                }
              </div>

              <aside class="rusty-engine-panel outcome">
                <aui-combat-log [entries]="combatLog()" />
                @if (latestLog(); as latest) {
                  <section class="latest" aria-label="Latest outcome explanation">
                    <p class="meta-label">Latest receipt · turn {{ latest.turn }}</p>
                    <strong>{{ latest.source }}</strong>
                    <p>{{ latest.text }}</p>
                    <ul class="detail-list">
                      @for (detail of latest.details; track detail) {
                        <li>{{ detail }}</li>
                      }
                    </ul>
                  </section>
                }
              </aside>
            </section>
          }
        }
      }
    </main>
  `,
})
export class MainMenuScreenComponent implements OnInit {
  protected readonly store = inject(SessionStore);
  private readonly selectedTarget = signal<number | null>(null);

  protected readonly game = computed(() => {
    const state = this.store.session();
    return state.kind === 'data' ? state.value : null;
  });

  protected readonly hotbarSlots = computed<readonly HotbarSlotView[]>(() =>
    (this.game()?.encounter?.actions ?? []).map((action, index) => ({
      index,
      keybind: String(index + 1),
      label: action.label,
      icon: index === 0 ? '⚔' : '➶',
      empty: false,
    })),
  );

  protected readonly combatLog = computed<readonly CombatLogEntryView[]>(() =>
    (this.game()?.encounter?.log ?? []).map((entry) => ({
      id: entry.id,
      source: `T${entry.turn} ${entry.source}`,
      text: entry.text,
      severity:
        entry.kind === 'hit'
          ? 'hit'
          : entry.kind === 'miss'
            ? 'miss'
            : entry.kind === 'system'
              ? 'system'
              : 'info',
    })),
  );

  protected readonly latestLog = computed(() => {
    const log = this.game()?.encounter?.log ?? [];
    return log.at(-1) ?? null;
  });

  ngOnInit(): void {
    void this.store.load();
  }

  protected encounter() {
    const encounter = this.game()?.encounter;
    if (encounter === null || encounter === undefined) {
      throw new Error('Encounter is not available.');
    }
    return encounter;
  }

  protected sessionError() {
    const state = this.store.session();
    if (state.kind !== 'error') {
      throw new Error('Session error is not available.');
    }
    return state.error;
  }

  protected characterStatus(character: CharacterDto): CharacterStatusView {
    const resource = character.resources.find((entry) => entry.id === 'guard') ??
      character.resources[0] ?? { label: 'Resource', current: 0, maximum: 0 };
    return {
      name: character.name,
      level: character.level,
      title: character.title,
      health: { current: character.healthCurrent, max: character.healthMaximum },
      resource: {
        label: resource.label,
        current: resource.current,
        max: resource.maximum,
      },
      buffs: character.effects,
    };
  }

  protected targetId(): number {
    return this.selectedTarget() ?? this.encounter().targets[0]?.id ?? 0;
  }

  protected selectTarget(event: Event): void {
    const target = event.target;
    if (target instanceof HTMLSelectElement) {
      this.selectedTarget.set(Number(target.value));
    }
  }

  protected chooseAction(slot: HotbarSlotView): void {
    const action = this.encounter().actions[slot.index];
    if (action !== undefined) {
      void this.store.previewAction(action.id, this.encounter().playerId, this.targetId());
    }
  }

  protected startEncounter(): void {
    void this.store.startEncounter();
  }

  protected applyReaction(token: string, reactionId: string): void {
    void this.store.applyReaction(token, reactionId);
  }

  protected resolveAction(token: string): void {
    void this.store.applyAction(token);
  }

  protected advanceTurn(): void {
    void this.store.advanceTurn();
  }

  protected save(): void {
    void this.store.save();
  }

  protected reload(): void {
    void this.store.load();
  }

  protected dismissError(): void {
    this.store.clearCommandError();
  }

  protected signed(value: number): string {
    return value >= 0 ? `+${value}` : String(value);
  }
}
