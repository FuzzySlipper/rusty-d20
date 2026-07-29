import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import type { OnInit } from '@angular/core';
import type { CharacterDto, LoadoutItemDto } from '@rusty-d20/protocol';
import { SessionStore } from '@rusty-d20/store';
import { CharacterStatusComponent, type CharacterStatusView } from '@rusty-d20/ui-character-status';
import { CombatLogComponent, type CombatLogEntryView } from '@rusty-d20/ui-combat-log';
import { HotbarComponent, type HotbarSlotView } from '@rusty-d20/ui-hotbar';
import {
  EquipmentPanelComponent,
  type EquipmentDropEvent,
  type EquipmentSlotView,
} from '@rusty-d20/ui-equipment';
import { InventoryGridComponent, type InventoryItemView } from '@rusty-d20/ui-inventory';

@Component({
  changeDetection: ChangeDetectionStrategy.OnPush,
  imports: [
    CharacterStatusComponent,
    CombatLogComponent,
    EquipmentPanelComponent,
    HotbarComponent,
    InventoryGridComponent,
  ],
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
        cursor: not-allowed;
        opacity: 0.5;
      }

      .save-hint {
        color: var(--rusty-engine-muted);
        font-size: 0.72rem;
        max-width: 19rem;
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

      .adventure-catalog {
        display: grid;
        gap: 12px;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        text-align: left;
      }

      .adventure-choice {
        align-content: start;
        border: 1px solid var(--rusty-engine-border);
        border-radius: var(--rusty-engine-radius);
        display: grid;
        gap: 10px;
        padding: 16px;
      }

      .adventure-choice button {
        justify-self: start;
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

      .camp {
        align-content: start;
        display: grid;
        gap: 16px;
      }

      .camp__header,
      .defense-readout,
      .stash__item {
        align-items: center;
        display: flex;
        flex-wrap: wrap;
        justify-content: space-between;
        gap: 10px;
      }

      .camp__layout {
        display: grid;
        gap: 14px;
        grid-template-columns: minmax(0, 1.35fr) minmax(280px, 0.65fr);
      }

      .loadout,
      .stash {
        align-content: start;
        display: grid;
        gap: 12px;
      }

      .loadout__widgets {
        display: grid;
        gap: 12px;
        grid-template-columns: minmax(220px, 0.8fr) minmax(320px, 1.2fr);
      }

      .defense-readout {
        background: rgba(255, 255, 255, 0.04);
        border: 1px solid var(--rusty-engine-border);
        border-radius: var(--rusty-engine-radius-sm);
        padding: 10px 12px;
      }

      .defense-readout strong {
        color: var(--rusty-engine-accent);
        font-size: 1.3rem;
      }

      .capacity {
        color: var(--rusty-engine-muted);
        font-size: 0.75rem;
      }

      .stash__items {
        display: grid;
        gap: 8px;
        list-style: none;
        margin: 0;
        padding: 0;
      }

      .stash__item {
        background: var(--rusty-engine-surface-solid);
        border: 1px solid var(--rusty-engine-border);
        border-radius: var(--rusty-engine-radius-sm);
        padding: 8px 10px;
      }

      .stash__identity {
        align-items: center;
        display: flex;
        gap: 8px;
      }

      .stash__icon {
        font-size: 1.35rem;
      }

      .encounter-choice {
        display: grid;
        gap: 8px;
      }

      .action-workbench,
      .outcome,
      .outcome-banner {
        align-content: start;
        display: grid;
        gap: 14px;
      }

      .outcome-banner {
        border-color: var(--rusty-engine-accent);
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
        .camp__layout,
        .adventure-catalog,
        .loadout__widgets,
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
            <p class="eyebrow">Rust-owned adventure</p>
            <h1>Rusty D20</h1>
          </div>
        </div>

        @if (game(); as snapshot) {
          @if (campaignEntered() && snapshot.campaign !== null) {
            <div class="topbar__controls">
              <span
                class="save-state"
                [class.save-state--saved]="snapshot.saved"
                aria-live="polite"
              >
                {{ snapshot.saved ? 'Saved' : 'Unsaved changes' }}
              </span>
              <button
                type="button"
                [disabled]="
                  store.busy() ||
                  (snapshot.encounter !== null && snapshot.encounter.pendingAction !== null)
                "
                [attr.title]="
                  snapshot.encounter !== null && snapshot.encounter.pendingAction !== null
                    ? 'Resolve the pending action before saving'
                    : null
                "
                (click)="save()"
              >
                Save
              </button>
              @if (snapshot.encounter !== null && snapshot.encounter.pendingAction !== null) {
                <span class="save-hint" role="status">
                  Resolve the pending action before saving.
                </span>
              }
              @if (
                snapshot.campaign.phase === 'encounter' &&
                snapshot.encounter?.turnOwner === 'opposition' &&
                snapshot.encounter.pendingAction === null
              ) {
                <button
                  class="primary"
                  type="button"
                  [disabled]="store.busy()"
                  (click)="beginOppositionTurn()"
                >
                  Begin {{ opponentName() }} turn
                </button>
              }
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

          @if (game()?.campaign === null) {
            <section class="rusty-engine-panel empty" aria-label="New adventure">
              <p class="eyebrow">Rust-compiled authored catalog</p>
              <h2>Choose an adventure</h2>
              <p class="lede">
                Each path has its own authored cast, loadout, actions, defenses, effects,
                opposition, and reward. Selection becomes immutable when the Rust campaign starts.
              </p>
              <div class="adventure-catalog">
                @for (choice of game()?.availableAdventures ?? []; track choice.id) {
                  <article class="adventure-choice">
                    <div>
                      <p class="meta-label">Authored path · {{ choice.id }}</p>
                      <h3>{{ choice.title }}</h3>
                    </div>
                    <p>{{ choice.summary }}</p>
                    <ul class="detail-list">
                      @for (detail of choice.details; track detail) {
                        <li>{{ detail }}</li>
                      }
                    </ul>
                    <button
                      class="primary"
                      type="button"
                      [disabled]="store.busy()"
                      (click)="newAdventure(choice.id)"
                    >
                      New Adventure · {{ choice.title }}
                    </button>
                  </article>
                }
              </div>
              <p class="muted">Engine {{ game()?.engineRevisionShort }} · exact checked catalog</p>
            </section>
          } @else if (!campaignEntered()) {
            <section class="rusty-engine-panel empty" aria-label="Continue adventure">
              <p class="eyebrow">Durable campaign found</p>
              <h2>Continue {{ game()?.campaign?.title }}</h2>
              <p class="lede">
                Resume in
                {{ game()?.campaign?.phase === 'camp' ? 'camp' : 'the active encounter' }}
                at state revision {{ game()?.revision }}.
              </p>
              <button class="primary" type="button" (click)="continueCampaign()">
                Continue Adventure
              </button>
            </section>
          } @else if (game()?.campaign?.phase === 'camp') {
            @if (game()?.campaign; as campaign) {
              <section class="camp" aria-label="Adventure camp">
                <header class="rusty-engine-panel camp__header">
                  <div>
                    <p class="eyebrow">Rust-owned camp · Engine-backed loadout</p>
                    <h2>{{ campaign.title }} Camp</h2>
                    <p class="lede">
                      Equip {{ campaign.hero.name }} from canonical inventory state, move spare gear
                      through the camp stash, and inspect every attributed defense before entering
                      the encounter.
                    </p>
                  </div>
                  <span class="capacity">
                    Carried {{ campaign.loadout.capacity.used }}/{{
                      campaign.loadout.capacity.maximum
                    }}
                  </span>
                </header>

                @if (campaign.latestOutcome; as outcome) {
                  <section
                    class="rusty-engine-panel outcome-banner"
                    [attr.aria-label]="'Latest encounter ' + outcome.kind"
                  >
                    <div>
                      <p class="eyebrow">{{ outcome.kind }}</p>
                      <h2>{{ outcome.title }}</h2>
                    </div>
                    <p>{{ outcome.summary }}</p>
                    @if (outcome.reward !== null) {
                      <p class="muted">
                        Reward: {{ outcome.reward }} · entity
                        {{ outcome.rewardItemId }}
                      </p>
                    }
                  </section>
                }

                <section class="camp__layout">
                  <div class="loadout">
                    <article class="character-card">
                      <aui-character-status [status]="characterStatus(campaign.hero)" />
                    </article>

                    <section class="loadout" aria-label="Defense readout">
                      @for (defense of campaign.loadout.defenses; track defense.id) {
                        <article
                          class="defense-readout"
                          [attr.aria-label]="defense.label + ' defense readout'"
                        >
                          <div>
                            <p class="meta-label">Derived {{ defense.label }} defense</p>
                            <strong>{{ defense.value }}</strong>
                          </div>
                          <details>
                            <summary>Attributed sources</summary>
                            <ul class="source-list">
                              @for (source of defense.sources; track source) {
                                <li>{{ source }}</li>
                              }
                            </ul>
                          </details>
                        </article>
                      }
                    </section>

                    <div class="loadout__widgets">
                      <aui-inventory-grid
                        [columns]="2"
                        [selectedItemId]="selectedInventoryItemId()"
                        [slots]="inventorySlots()"
                        (itemActivated)="activateInventoryItem($event)"
                      />
                      <aui-equipment-panel
                        [slots]="equipmentSlots()"
                        (itemDropped)="equipDroppedItem($event)"
                        (slotSelected)="unequipSlot($event)"
                      />
                    </div>

                    <section class="rusty-engine-panel" aria-label="Inventory item actions">
                      <p class="meta-label">Carried gear</p>
                      <ul class="stash__items">
                        @for (item of carriedItems(); track item.entityId) {
                          <li class="stash__item">
                            <span class="stash__identity">
                              <span class="stash__icon" aria-hidden="true">{{ item.icon }}</span>
                              <span>
                                {{ item.name }}
                                @if (item.equippedSlotId !== null) {
                                  · equipped {{ item.equippedSlotId }}
                                }
                              </span>
                            </span>
                            <button
                              type="button"
                              [disabled]="store.busy() || item.equippedSlotId !== null"
                              [attr.title]="
                                item.equippedSlotId !== null
                                  ? 'Unequip this item before moving it to the stash'
                                  : null
                              "
                              (click)="storeItem(item)"
                            >
                              Store
                            </button>
                          </li>
                        }
                      </ul>
                    </section>
                  </div>

                  <aside class="stash rusty-engine-panel" aria-label="Camp stash">
                    <div>
                      <p class="meta-label">Canonical storage</p>
                      <h2>Camp stash</h2>
                    </div>
                    @if (campaign.loadout.stashItems.length === 0) {
                      <p class="muted">The stash is empty.</p>
                    } @else {
                      <ul class="stash__items">
                        @for (item of campaign.loadout.stashItems; track item.entityId) {
                          <li class="stash__item">
                            <span class="stash__identity">
                              <span class="stash__icon" aria-hidden="true">{{ item.icon }}</span>
                              <span>{{ item.name }}</span>
                            </span>
                            <button
                              type="button"
                              [disabled]="store.busy()"
                              (click)="takeItem(item)"
                            >
                              Take
                            </button>
                          </li>
                        }
                      </ul>
                    }
                    <p class="muted">
                      Capacity, containment, equipped-item, and stale-state rejections come from the
                      Rust owner without changing the live loadout.
                    </p>

                    @for (choice of campaign.availableEncounters; track choice.id) {
                      <article class="action-note encounter-choice">
                        <strong>{{ choice.title }}</strong>
                        <span>{{ choice.summary }}</span>
                        <button
                          class="primary"
                          type="button"
                          [disabled]="store.busy()"
                          (click)="enterEncounter(choice.id)"
                        >
                          Enter {{ choice.title }}
                        </button>
                      </article>
                    }
                  </aside>
                </section>
              </section>
            }
          } @else {
            <section class="encounter-meta" aria-label="Encounter identity">
              <span>Turn {{ encounter().turn }}</span>
              <span>
                {{
                  encounter().turnOwner === 'player'
                    ? encounter().player.name + ' acting'
                    : encounter().turnOwner === 'opposition'
                      ? opponentName() + ' acting'
                      : 'Encounter resolved'
                }}
              </span>
              <span>Next deterministic roll {{ encounter().nextRoll }}</span>
              <span>State revision {{ game()?.revision }}</span>
              @for (defense of game()?.campaign?.loadout?.defenses ?? []; track defense.id) {
                <span>{{ defense.label }} defense {{ defense.value }}</span>
              }
              <span
                >Engine <code>{{ game()?.engineRevisionShort }}</code></span
              >
              <span>
                Rules
                <code [title]="game()?.rulesetFingerprint">{{ game()?.campaign?.title }}</code>
              </span>
            </section>

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

            @if (game()?.campaign?.phase === 'outcome') {
              @if (game()?.campaign?.latestOutcome; as outcome) {
                <section class="workspace">
                  <article
                    class="rusty-engine-panel outcome-banner"
                    [attr.aria-label]="'Encounter ' + outcome.kind"
                  >
                    <p class="eyebrow">{{ outcome.kind }}</p>
                    <h2>{{ outcome.title }}</h2>
                    <p class="lede">{{ outcome.summary }}</p>
                    @if (outcome.reward !== null) {
                      <p>
                        <strong>Reward admitted:</strong>
                        {{ outcome.reward }} · canonical entity
                        {{ outcome.rewardItemId }}
                      </p>
                    } @else {
                      <p class="muted">
                        Returning to camp applies bounded recovery without granting a reward.
                      </p>
                    }
                    <button
                      class="primary"
                      type="button"
                      [disabled]="store.busy()"
                      (click)="returnToCamp()"
                    >
                      Return to {{ game()?.campaign?.title }} Camp
                    </button>
                  </article>
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
            } @else {
              <section class="workspace">
              <div class="action-workbench">
                <section class="rusty-engine-panel">
                  <header class="actions__header">
                    <div>
                      <p class="meta-label">
                        {{
                          encounter().turnOwner === 'player'
                            ? 'Authored player actions'
                            : 'Deterministic opposition'
                        }}
                      </p>
                      <h2>
                        {{
                          encounter().turnOwner === 'player'
                            ? 'Choose an action'
                            : encounter().pendingAction === null
                              ? opponentName() + ' is ready'
                              : 'Respond to ' + opponentName()
                        }}
                      </h2>
                    </div>
                    @if (encounter().turnOwner === 'player') {
                      <div class="target-control">
                      <label for="target">Target</label>
                      <select id="target" [value]="targetId()" (change)="selectTarget($event)">
                        @for (target of encounter().targets; track target.id) {
                          <option [value]="target.id">{{ target.name }}</option>
                        }
                      </select>
                      </div>
                    }
                  </header>

                  @if (encounter().turnOwner === 'player') {
                    <aui-hotbar [slots]="hotbarSlots()" (slotSelected)="chooseAction($event)" />

                    <div class="action-catalog">
                      @for (action of encounter().actions; track action.id) {
                        <div class="action-note">
                          <strong>{{ action.label }}</strong>
                          <span>
                            {{ action.ability }} vs {{ action.defense }} ·
                            {{ action.damage }}
                            @if (action.effect !== null) {
                              · {{ action.effect }}
                            }
                          </span>
                        </div>
                      }
                    </div>
                  } @else if (encounter().pendingAction === null) {
                    <p class="lede">
                      Begin the explicit opposition phase to let Rust choose
                      {{ opponentName() }}'s action from admitted definitions. You can inspect and
                      answer its preview before the roll.
                    </p>
                    <button
                      class="primary resolve"
                      type="button"
                      [disabled]="store.busy()"
                      (click)="beginOppositionTurn()"
                    >
                      Begin {{ opponentName() }} turn
                    </button>
                  }
                </section>

                @if (encounter().pendingAction; as pending) {
                  <section
                    class="rusty-engine-panel preview"
                    aria-label="Authoritative action preview"
                  >
                    <p class="meta-label">
                      Rust preview · {{ pendingActorName(pending.actorId) }} ·
                      {{ pending.actionLabel }}
                    </p>
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
      }
    </main>
  `,
})
export class MainMenuScreenComponent implements OnInit {
  protected readonly store = inject(SessionStore);
  private readonly selectedTarget = signal<number | null>(null);
  private readonly selectedLoadoutItem = signal<number | null>(null);
  protected readonly campaignEntered = signal(false);

  protected readonly game = computed(() => {
    const state = this.store.session();
    return state.kind === 'data' ? state.value : null;
  });

  protected readonly inventorySlots = computed<readonly (InventoryItemView | null)[]>(() =>
    (this.game()?.campaign?.loadout.inventorySlots ?? []).map((item) =>
      item === null ? null : this.inventoryItem(item),
    ),
  );

  protected readonly selectedInventoryItemId = computed(() => {
    const selected = this.selectedLoadoutItem();
    return selected === null ? null : String(selected);
  });

  protected readonly equipmentSlots = computed<readonly EquipmentSlotView[]>(() =>
    (this.game()?.campaign?.loadout.equipmentSlots ?? []).map((slot) => ({
      id: slot.id,
      label: slot.label,
      equipped:
        slot.equipped === null
          ? null
          : {
              id: String(slot.equipped.entityId),
              name: slot.equipped.name,
              icon: slot.equipped.icon,
              rarity: slot.equipped.rarity,
            },
    })),
  );

  protected readonly carriedItems = computed<readonly LoadoutItemDto[]>(() =>
    (this.game()?.campaign?.loadout.inventorySlots ?? []).filter(
      (item): item is LoadoutItemDto => item !== null,
    ),
  );

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
      health: {
        current: character.healthCurrent,
        max: character.healthMaximum,
      },
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

  protected async newAdventure(adventureId: string): Promise<void> {
    await this.store.newAdventure(adventureId);
    if (this.game()?.campaign !== null) {
      this.campaignEntered.set(true);
    }
  }

  protected continueCampaign(): void {
    this.campaignEntered.set(true);
  }

  protected enterEncounter(encounterId: string): void {
    void this.store.enterEncounter(encounterId);
  }

  protected activateInventoryItem(item: InventoryItemView): void {
    const authoritative = this.findCarriedItem(item.id);
    if (authoritative === undefined) {
      return;
    }
    this.selectedLoadoutItem.set(authoritative.entityId);
    if (authoritative.equippedSlotId === null) {
      void this.store.equipItem(authoritative.entityId, authoritative.equipmentSlotId);
    } else {
      void this.store.unequipItem(authoritative.entityId);
    }
  }

  protected equipDroppedItem(event: EquipmentDropEvent): void {
    const authoritative = this.findCarriedItem(event.itemId);
    if (authoritative !== undefined) {
      void this.store.equipItem(authoritative.entityId, event.slotId);
    }
  }

  protected unequipSlot(slot: EquipmentSlotView): void {
    if (slot.equipped !== null) {
      void this.store.unequipItem(Number(slot.equipped.id));
    }
  }

  protected storeItem(item: LoadoutItemDto): void {
    const loadout = this.game()?.campaign?.loadout;
    if (loadout !== undefined && item.equippedSlotId === null) {
      void this.store.transferItem(item.entityId, loadout.ownerId, loadout.stashOwnerId);
    }
  }

  protected takeItem(item: LoadoutItemDto): void {
    const loadout = this.game()?.campaign?.loadout;
    if (loadout !== undefined) {
      void this.store.transferItem(item.entityId, loadout.stashOwnerId, loadout.ownerId);
    }
  }

  protected applyReaction(token: string, reactionId: string): void {
    void this.store.applyReaction(token, reactionId);
  }

  protected resolveAction(token: string): void {
    void this.store.applyAction(token);
  }

  protected beginOppositionTurn(): void {
    void this.store.beginOppositionTurn();
  }

  protected returnToCamp(): void {
    void this.store.returnToCamp();
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

  protected pendingActorName(actorId: number): string {
    return (
      this.encounter().characters.find((character) => character.id === actorId)?.name ??
      `Entity ${actorId}`
    );
  }

  protected opponentName(): string {
    return this.encounter().targets[0]?.name ?? 'Opposition';
  }

  private inventoryItem(item: LoadoutItemDto): InventoryItemView {
    return {
      id: String(item.entityId),
      name:
        item.equippedSlotId === null ? item.name : `${item.name} · equipped ${item.equippedSlotId}`,
      icon: item.icon,
      rarity: item.rarity,
      quantity: item.quantity,
      equippable: item.equippedSlotId === null,
    };
  }

  private findCarriedItem(itemId: string): LoadoutItemDto | undefined {
    const entityId = Number(itemId);
    return this.carriedItems().find((item) => item.entityId === entityId);
  }
}
