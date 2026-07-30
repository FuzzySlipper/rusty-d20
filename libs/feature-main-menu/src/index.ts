import {
  ChangeDetectionStrategy,
  Component,
  Injector,
  afterNextRender,
  computed,
  inject,
  signal,
  viewChild,
} from "@angular/core";
import type { ElementRef, OnInit } from "@angular/core";
import { browserAnimationFrame, browserClock } from "@rusty-d20/platform";
import type {
  CharacterDto,
  ExplorationCommandKindDto,
  LoadoutItemDto,
} from "@rusty-d20/protocol";
import {
  GameViewportComponent,
  TacticalBoardComponent,
  type DungeonViewportView,
  type GameViewportView,
  type TacticalBoardSelection,
  type TacticalBoardView,
} from "@rusty-d20/renderer";
import { SessionStore } from "@rusty-d20/store";
import {
  CharacterStatusComponent,
  type CharacterStatusView,
} from "@rusty-d20/ui-character-status";
import {
  CombatLogComponent,
  type CombatLogEntryView,
} from "@rusty-d20/ui-combat-log";
import { HotbarComponent, type HotbarSlotView } from "@rusty-d20/ui-hotbar";
import {
  EquipmentPanelComponent,
  type EquipmentDropEvent,
  type EquipmentSlotView,
} from "@rusty-d20/ui-equipment";
import {
  InventoryGridComponent,
  type InventoryItemView,
} from "@rusty-d20/ui-inventory";
import {
  CompassComponent,
  type CompassMarkerView,
} from "@rusty-d20/ui-compass";
import {
  MinimapComponent,
  type MinimapMarkerView,
} from "@rusty-d20/ui-minimap";

@Component({
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: {
    "(window:keydown)": "handleExplorationKeydown($event)",
  },
  imports: [
    CharacterStatusComponent,
    CombatLogComponent,
    CompassComponent,
    EquipmentPanelComponent,
    GameViewportComponent,
    HotbarComponent,
    InventoryGridComponent,
    MinimapComponent,
    TacticalBoardComponent,
  ],
  selector: "aui-main-menu-screen",
  standalone: true,
  styles: [
    `
      :host {
        display: block;
        min-height: 100dvh;
      }

      .game-shell {
        background: var(--rusty-engine-bg);
        isolation: isolate;
        min-height: 100dvh;
        overflow: hidden;
        position: relative;
      }

      .game-viewport {
        inset: 0;
        position: fixed;
        z-index: 0;
      }

      .game-overlay {
        display: grid;
        gap: 18px;
        grid-template-rows: auto minmax(0, 1fr);
        height: 100dvh;
        margin: 0 auto;
        max-width: 1440px;
        overflow-x: hidden;
        overflow-y: auto;
        padding: clamp(16px, 3vw, 32px);
        pointer-events: none;
        position: relative;
        scrollbar-gutter: stable;
        z-index: 2;
      }

      .game-overlay > .topbar,
      .game-overlay > .reset-dialog,
      .game-overlay__stage .rusty-engine-panel,
      .game-overlay__stage .command-error,
      .game-overlay__stage button,
      .game-overlay__stage a,
      .game-overlay__stage input,
      .game-overlay__stage select,
      .game-overlay__stage summary,
      .game-overlay__stage textarea,
      .game-overlay__stage aui-character-status,
      .game-overlay__stage aui-combat-log,
      .game-overlay__stage aui-compass,
      .game-overlay__stage aui-equipment-panel,
      .game-overlay__stage aui-hotbar,
      .game-overlay__stage aui-inventory-grid,
      .game-overlay__stage aui-minimap,
      .game-overlay__stage aui-tactical-board {
        pointer-events: auto;
      }

      .game-overlay__stage {
        align-content: start;
        display: grid;
        min-height: 0;
        pointer-events: none;
        position: relative;
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
        backdrop-filter: blur(14px);
        background: rgb(6 11 14 / 0.72);
        border: 1px solid var(--rusty-engine-border);
        border-radius: var(--rusty-engine-radius);
        justify-content: space-between;
        padding: 10px 12px;
        position: sticky;
        top: 0;
        z-index: 8;
      }

      .mark {
        background: linear-gradient(
          145deg,
          var(--rusty-engine-accent),
          var(--rusty-engine-cool)
        );
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

      .identity-readout {
        border: 1px solid var(--rusty-engine-border);
        border-radius: var(--rusty-engine-radius-sm);
        display: grid;
        overflow-wrap: anywhere;
        padding: 10px 12px;
        text-align: left;
      }

      .danger {
        border-color: var(--rusty-engine-danger);
        color: var(--rusty-engine-danger);
      }

      .reset-dialog {
        background: var(--rusty-engine-surface-solid);
        border: 1px solid var(--rusty-engine-danger);
        border-radius: var(--rusty-engine-radius);
        color: var(--rusty-engine-text);
        gap: 14px;
        margin: auto;
        max-width: min(92vw, 620px);
        padding: 22px;
        position: fixed;
        z-index: 20;
      }

      .reset-dialog[open] {
        display: grid;
      }

      .reset-dialog::backdrop {
        background: rgba(3, 7, 12, 0.82);
      }

      .reset-dialog__actions {
        display: flex;
        flex-wrap: wrap;
        gap: 8px;
        justify-content: flex-end;
      }

      .primary {
        background: var(--rusty-engine-accent-strong);
        border-color: var(--rusty-engine-accent);
      }

      .empty,
      .fatal {
        align-self: center;
        backdrop-filter: blur(16px);
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

      .exploration {
        display: grid;
        gap: 14px;
        grid-template-columns: minmax(250px, 360px) minmax(190px, 230px);
        justify-content: space-between;
        min-height: calc(100dvh - 132px);
      }

      .exploration__main,
      .exploration__sidebar,
      .movement-pad,
      .landmark {
        align-content: start;
        display: grid;
        gap: 12px;
      }

      .exploration__main {
        grid-template-rows: auto minmax(32px, 1fr) auto;
      }

      .exploration__main > .landmark {
        align-self: start;
      }

      .exploration__sidebar aui-compass,
      .exploration__sidebar aui-minimap {
        width: 100%;
      }

      .movement-pad {
        grid-template-columns: repeat(3, minmax(0, 1fr));
      }

      .movement-pad button {
        min-height: 48px;
      }

      .movement-pad__forward,
      .movement-pad__back {
        grid-column: 2;
      }

      .movement-pad__left {
        grid-column: 1;
      }

      .movement-pad__right {
        grid-column: 3;
      }

      .exploration__status {
        display: grid;
        gap: 10px;
      }

      .camp {
        align-content: start;
        display: grid;
        gap: 16px;
        min-height: calc(100dvh - 132px);
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
      .adventure-complete,
      .outcome,
      .outcome-banner {
        align-content: start;
        display: grid;
        gap: 14px;
      }

      .action-workbench {
        min-width: 0;
      }

      .outcome-banner {
        border-color: var(--rusty-engine-accent);
      }

      .adventure-complete {
        align-self: center;
        border-color: var(--rusty-engine-accent);
        justify-self: center;
        max-width: 760px;
        padding: clamp(24px, 6vw, 56px);
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
        backdrop-filter: blur(14px);
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
        .exploration,
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

        .exploration {
          min-height: auto;
        }
      }

      @media (max-width: 420px) {
        .game-overlay {
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
    <main class="game-shell" [attr.data-scene-mode]="gameViewport().mode">
      <aui-game-viewport class="game-viewport" [view]="gameViewport()" />
      <div class="game-overlay">
        <header class="topbar" data-overlay-region="top">
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
                  {{ snapshot.saved ? "Saved" : "Unsaved changes" }}
                </span>
                <button
                  type="button"
                  [disabled]="
                    store.busy() ||
                    (snapshot.encounter !== null &&
                      snapshot.encounter.reactionPrompt !== null)
                  "
                  [attr.title]="
                    snapshot.encounter !== null &&
                    snapshot.encounter.reactionPrompt !== null
                      ? 'Choose or decline the reaction before saving'
                      : null
                  "
                  (click)="save()"
                >
                  Save
                </button>
                <button
                  class="danger"
                  type="button"
                  [disabled]="store.busy() || saveStatus() === null"
                  (click)="openResetDialog($event)"
                >
                  Reset / New Adventure
                </button>
                @if (
                  snapshot.encounter !== null &&
                  snapshot.encounter.reactionPrompt !== null
                ) {
                  <span class="save-hint" role="status">
                    Choose or decline the reaction before saving.
                  </span>
                }
                @if (
                  snapshot.campaign.phase === "encounter" &&
                  snapshot.encounter?.currentFaction === "opposition" &&
                  snapshot.encounter.reactionPrompt === null
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

        <dialog
          #resetDialog
          class="reset-dialog"
          role="alertdialog"
          aria-labelledby="reset-title"
          aria-describedby="reset-description"
          aria-modal="true"
          (cancel)="cancelReset($event)"
          (keydown)="handleResetDialogKeydown($event)"
        >
          <p class="eyebrow">Destructive save operation</p>
          <h2 id="reset-title">Discard this adventure?</h2>
          <p id="reset-description">
            This removes the save at
            <strong>{{ saveStatus()?.saveIdentity }}</strong>
            @if (game()?.campaign; as campaign) {
              and discards {{ campaign.title }} at revision
              {{ game()?.revision }}
            } @else {
              and discards the unreadable persisted session
            }
            . Unsaved changes and any pending reaction cannot be recovered.
          </p>
          <div class="reset-dialog__actions">
            <button
              #resetCancelButton
              type="button"
              autofocus
              [disabled]="store.busy()"
              (click)="cancelReset()"
            >
              Cancel
            </button>
            <button
              #resetConfirmButton
              class="danger"
              type="button"
              [disabled]="store.busy()"
              (click)="confirmReset()"
            >
              Discard save and start over
            </button>
          </div>
        </dialog>

        <div class="game-overlay__stage">
          @switch (store.session().kind) {
            @case ("idle") {
              <section class="rusty-engine-panel empty" aria-live="polite">
                <p>Preparing the authoritative session…</p>
              </section>
            }
            @case ("loading") {
              <section
                class="rusty-engine-panel empty"
                aria-live="polite"
                aria-busy="true"
              >
                <p>Loading authored rules and Rust state…</p>
              </section>
            }
            @case ("error") {
              <section class="rusty-engine-panel fatal" role="alert">
                <p class="eyebrow">{{ sessionError().kind }} failure</p>
                @if (saveStatus()?.state === "recovery-required") {
                  <h2>Saved adventure needs recovery</h2>
                  <p>
                    The runtime rejected the persisted session without changing
                    it.
                  </p>
                  <div class="identity-readout">
                    <strong>Recovery required</strong>
                    <span>{{ saveStatus()?.saveIdentity }}</span>
                    <span>{{ saveStatus()?.persistenceError }}</span>
                  </div>
                  @if (store.commandError(); as resetError) {
                    <p class="command-error">{{ resetError.message }}</p>
                  }
                  <button
                    class="danger"
                    type="button"
                    [disabled]="store.busy()"
                    (click)="openResetDialog($event)"
                  >
                    Discard unreadable save
                  </button>
                } @else {
                  <h2>Could not reach the game runtime</h2>
                  <p>{{ sessionError().message }}</p>
                  @if (sessionError().retryable) {
                    <button class="primary" type="button" (click)="reload()">
                      Retry connection
                    </button>
                  }
                }
              </section>
            }
            @case ("data") {
              @if (store.commandError(); as error) {
                <section
                  class="command-error"
                  data-overlay-region="status"
                  role="alert"
                >
                  <strong>{{ error.kind }} rejection</strong>
                  <span>{{ error.message }}</span>
                  <div class="command-error__actions">
                    @if (error.retryable) {
                      <button type="button" (click)="reload()">
                        Reload current state
                      </button>
                    }
                    <button type="button" (click)="dismissError()">
                      Dismiss
                    </button>
                  </div>
                </section>
              }

              @if (game()?.campaign === null) {
                <section
                  class="rusty-engine-panel empty"
                  data-overlay-region="modal"
                  aria-label="New adventure"
                >
                  <p class="eyebrow">Rust-compiled authored catalog</p>
                  <h2 #newAdventureHeading tabindex="-1">
                    Choose an adventure
                  </h2>
                  <p class="lede">
                    Each path has its own authored cast, loadout, actions,
                    defenses, effects, opposition, and reward. Selection becomes
                    immutable when the Rust campaign starts.
                  </p>
                  <div class="adventure-catalog">
                    @for (
                      choice of game()?.availableAdventures ?? [];
                      track choice.id
                    ) {
                      <article class="adventure-choice">
                        <div>
                          <p class="meta-label">
                            Authored path · {{ choice.id }}
                          </p>
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
                  @if (saveStatus(); as status) {
                    <div class="muted identity-readout">
                      <span>New adventures use save identity</span>
                      <strong>{{ status.saveIdentity }}</strong>
                    </div>
                  }
                  <p class="muted">
                    Engine {{ game()?.engineRevisionShort }} · exact checked
                    catalog
                  </p>
                </section>
              } @else if (!campaignEntered()) {
                <section
                  class="rusty-engine-panel empty"
                  data-overlay-region="modal"
                  aria-label="Continue adventure"
                >
                  <p class="eyebrow">Durable campaign found</p>
                  <h2>Continue {{ game()?.campaign?.title }}</h2>
                  <p class="lede">
                    Resume in
                    {{
                      game()?.campaign?.phase === "camp"
                        ? "camp"
                        : game()?.campaign?.phase === "exploration"
                          ? "the dungeon"
                          : game()?.campaign?.phase === "adventure-complete"
                            ? "the completed expedition"
                            : "the active encounter"
                    }}
                    at state revision {{ game()?.revision }}.
                  </p>
                  @if (saveStatus(); as status) {
                    <div class="identity-readout">
                      <strong>{{ status.saveIdentity }}</strong>
                      <span
                        >Adventure {{ status.campaignId }} · revision
                        {{ status.revision }}</span
                      >
                    </div>
                  }
                  <button
                    class="primary"
                    type="button"
                    (click)="continueCampaign()"
                  >
                    Continue Adventure
                  </button>
                  <button
                    class="danger"
                    type="button"
                    [disabled]="store.busy() || saveStatus() === null"
                    (click)="openResetDialog($event)"
                  >
                    Reset / New Adventure
                  </button>
                </section>
              } @else if (game()?.campaign?.phase === "camp") {
                @if (game()?.campaign; as campaign) {
                  <section class="camp" aria-label="Adventure camp">
                    <header class="rusty-engine-panel camp__header">
                      <div>
                        <p class="eyebrow">
                          Rust-owned camp · Engine-backed loadout
                        </p>
                        <h2>{{ campaign.title }} Camp</h2>
                        <p class="lede">
                          Equip every party member from canonical inventory
                          state, move spare gear through the camp stash, and
                          inspect every attributed defense before entering the
                          encounter.
                        </p>
                      </div>
                      <span class="capacity">
                        Carried
                        {{ activePartyMember()?.loadout?.capacity?.used }}/{{
                          activePartyMember()?.loadout?.capacity?.maximum
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
                        <nav
                          class="reaction-list"
                          aria-label="Party loadout selection"
                        >
                          @for (
                            member of campaign.party;
                            track member.character.id
                          ) {
                            <button
                              type="button"
                              [class.primary]="
                                activePartyMember()?.character?.id ===
                                member.character.id
                              "
                              (click)="selectPartyMember(member.character.id)"
                            >
                              {{ member.character.name }}
                            </button>
                          }
                        </nav>
                        <article class="character-card">
                          @if (activePartyMember(); as member) {
                            <aui-character-status
                              [status]="characterStatus(member.character)"
                            />
                          }
                        </article>

                        <section class="loadout" aria-label="Defense readout">
                          @for (
                            defense of activePartyMember()?.loadout?.defenses ??
                              [];
                            track defense.id
                          ) {
                            <article
                              class="defense-readout"
                              [attr.aria-label]="
                                defense.label + ' defense readout'
                              "
                            >
                              <div>
                                <p class="meta-label">
                                  Derived {{ defense.label }} defense
                                </p>
                                <strong>{{ defense.value }}</strong>
                              </div>
                              <details>
                                <summary>Attributed sources</summary>
                                <ul class="source-list">
                                  @for (
                                    source of defense.sources;
                                    track source
                                  ) {
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

                        <section
                          class="rusty-engine-panel"
                          aria-label="Inventory item actions"
                        >
                          <p class="meta-label">Carried gear</p>
                          <ul class="stash__items">
                            @for (item of carriedItems(); track item.entityId) {
                              <li class="stash__item">
                                <span class="stash__identity">
                                  <span
                                    class="stash__icon"
                                    aria-hidden="true"
                                    >{{ item.icon }}</span
                                  >
                                  <span>
                                    {{ item.name }}
                                    @if (item.equippedSlotId !== null) {
                                      · equipped {{ item.equippedSlotId }}
                                    }
                                  </span>
                                </span>
                                <button
                                  type="button"
                                  [disabled]="
                                    store.busy() || item.equippedSlotId !== null
                                  "
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

                      <aside
                        class="stash rusty-engine-panel"
                        data-overlay-region="right"
                        aria-label="Camp stash"
                      >
                        <div>
                          <p class="meta-label">Canonical storage</p>
                          <h2>Camp stash</h2>
                        </div>
                        @if (
                          (activePartyMember()?.loadout?.stashItems?.length ??
                            0) === 0
                        ) {
                          <p class="muted">The stash is empty.</p>
                        } @else {
                          <ul class="stash__items">
                            @for (
                              item of activePartyMember()?.loadout
                                ?.stashItems ?? [];
                              track item.entityId
                            ) {
                              <li class="stash__item">
                                <span class="stash__identity">
                                  <span
                                    class="stash__icon"
                                    aria-hidden="true"
                                    >{{ item.icon }}</span
                                  >
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
                          Capacity, containment, equipped-item, and stale-state
                          rejections come from the Rust owner without changing
                          the live loadout.
                        </p>

                        @if (campaign.availableEncounters.length > 0) {
                          <article class="action-note encounter-choice">
                            <strong>Begin the expedition</strong>
                            <span>
                              Enter the authored dungeon. Encounters begin only
                              when the party reaches their hidden Rust-owned
                              trigger.
                            </span>
                            <button
                              class="primary"
                              type="button"
                              [disabled]="store.busy()"
                              (click)="beginExploration()"
                            >
                              Enter the dungeon
                            </button>
                          </article>
                        }
                        @if (campaign.completedEncounters.length > 0) {
                          <section
                            class="identity-readout"
                            aria-label="Completed encounters"
                          >
                            <strong>Campaign progress</strong>
                            @for (
                              completed of campaign.completedEncounters;
                              track completed.encounterId
                            ) {
                              <span
                                >{{ completed.title }} ·
                                {{ completed.outcome }}</span
                              >
                            }
                          </section>
                        }
                      </aside>
                    </section>
                  </section>
                }
              } @else if (game()?.campaign?.phase === "exploration") {
                @if (game()?.exploration; as exploration) {
                  <section class="exploration" aria-label="Dungeon exploration">
                    <div class="exploration__main" data-overlay-region="left">
                      <header class="rusty-engine-panel">
                        <p class="eyebrow">Rust-owned dungeon exploration</p>
                        <h2>{{ exploration.dungeonTitle }}</h2>
                        <p class="lede">
                          Move one square at a time. Only visited cells reach
                          the automap, and authored encounters remain hidden
                          until the party steps onto them.
                        </p>
                      </header>
                      @if (exploration.landmark; as landmark) {
                        <section
                          class="rusty-engine-panel landmark"
                          aria-label="Dungeon landmark"
                        >
                          <p class="meta-label">
                            {{ landmark.inspected ? "Inspected" : "Landmark" }}
                          </p>
                          <h3>{{ landmark.title }}</h3>
                          <p>{{ landmark.text }}</p>
                          <button
                            type="button"
                            [disabled]="store.busy() || landmark.inspected"
                            (click)="explorationCommand('interact')"
                          >
                            {{
                              landmark.inspected
                                ? "Already inspected"
                                : "Inspect"
                            }}
                          </button>
                        </section>
                      }
                      @if (exploration.treasure; as treasure) {
                        <section
                          class="rusty-engine-panel landmark"
                          aria-label="Dungeon treasure"
                        >
                          <p class="meta-label">
                            {{
                              treasure.collected
                                ? "Claimed treasure"
                                : "Treasure"
                            }}
                          </p>
                          <h3>{{ treasure.title }}</h3>
                          <p>{{ treasure.text }}</p>
                          <button
                            class="primary"
                            type="button"
                            [disabled]="store.busy() || treasure.collected"
                            (click)="explorationCommand('interact')"
                          >
                            {{
                              treasure.collected
                                ? "Already claimed"
                                : "Claim treasure"
                            }}
                          </button>
                        </section>
                      }
                      @if (exploration.doorAhead; as door) {
                        <section
                          class="rusty-engine-panel landmark"
                          aria-label="Dungeon door"
                        >
                          <p class="meta-label">
                            {{
                              door.opened
                                ? "Opened passage"
                                : door.locked
                                  ? "Locked passage"
                                  : "Door"
                            }}
                          </p>
                          <h3>{{ door.title }}</h3>
                          <p>{{ door.text }}</p>
                          <button
                            class="primary"
                            type="button"
                            [disabled]="
                              store.busy() || door.opened || door.locked
                            "
                            (click)="explorationCommand('interact')"
                          >
                            {{
                              door.opened
                                ? "Door opened"
                                : door.locked
                                  ? "Requires its authored treasure"
                                  : "Open door"
                            }}
                          </button>
                        </section>
                      }
                      @if (exploration.checkpoint; as checkpoint) {
                        <section
                          class="rusty-engine-panel landmark"
                          aria-label="Dungeon checkpoint"
                        >
                          <p class="meta-label">
                            {{
                              checkpoint.active
                                ? "Active checkpoint"
                                : "Safe return"
                            }}
                          </p>
                          <h3>{{ checkpoint.title }}</h3>
                          <p>{{ checkpoint.text }}</p>
                          <button
                            type="button"
                            [disabled]="store.busy()"
                            (click)="explorationCommand('interact')"
                          >
                            Return safely to camp
                          </button>
                        </section>
                      }
                      <nav
                        class="rusty-engine-panel movement-pad"
                        data-overlay-region="bottom"
                        aria-label="Dungeon movement"
                      >
                        <button
                          class="movement-pad__forward"
                          type="button"
                          [disabled]="
                            store.busy() || !exploration.canStepForward
                          "
                          (click)="explorationCommand('step-forward')"
                        >
                          ↑ Forward
                        </button>
                        <button
                          class="movement-pad__left"
                          type="button"
                          [disabled]="store.busy()"
                          (click)="explorationCommand('turn-left')"
                        >
                          ↶ Left
                        </button>
                        <button
                          class="movement-pad__right"
                          type="button"
                          [disabled]="store.busy()"
                          (click)="explorationCommand('turn-right')"
                        >
                          Right ↷
                        </button>
                        <button
                          class="movement-pad__back"
                          type="button"
                          [disabled]="
                            store.busy() || !exploration.canStepBackward
                          "
                          (click)="explorationCommand('step-backward')"
                        >
                          ↓ Back
                        </button>
                      </nav>
                    </div>
                    <aside
                      class="exploration__sidebar"
                      data-overlay-region="right"
                    >
                      <aui-compass
                        [headingDegrees]="compassHeading()"
                        [markers]="compassMarkers"
                      />
                      <aui-minimap
                        [regionName]="exploration.dungeonTitle"
                        [markers]="minimapMarkers()"
                        [playerXPercent]="minimapPlayerX()"
                        [playerYPercent]="minimapPlayerY()"
                      />
                      <section
                        class="rusty-engine-panel exploration__status"
                        aria-label="Party status"
                      >
                        <p class="meta-label">Exploring party</p>
                        @for (
                          member of game()!.campaign!.party;
                          track member.character.id
                        ) {
                          <aui-character-status
                            [status]="characterStatus(member.character)"
                          />
                        }
                        <span class="muted">
                          Facing {{ exploration.facing }} · cell
                          {{ exploration.x }},{{ exploration.y }}
                        </span>
                        <span class="muted">
                          {{ exploration.discoveredCells.length }} cells
                          discovered
                        </span>
                      </section>
                    </aside>
                  </section>
                }
              } @else if (game()?.campaign?.phase === "adventure-complete") {
                @if (game()?.campaign?.completion; as completion) {
                  <section
                    class="rusty-engine-panel adventure-complete"
                    data-overlay-region="modal"
                    [attr.aria-label]="'Adventure complete: ' + completion.kind"
                  >
                    <p class="eyebrow">
                      Adventure complete · {{ completion.kind }}
                    </p>
                    <h2>{{ completion.title }}</h2>
                    <p class="lede">{{ completion.text }}</p>
                    <ul class="detail-list">
                      @for (detail of completion.details; track detail) {
                        <li>{{ detail }}</li>
                      }
                    </ul>
                    <section
                      class="identity-readout"
                      aria-label="Final encounter record"
                    >
                      <strong>{{ game()?.campaign?.title }}</strong>
                      @for (
                        completed of game()?.campaign?.completedEncounters ??
                          [];
                        track completed.encounterId
                      ) {
                        <span
                          >{{ completed.title }} · {{ completed.outcome }}</span
                        >
                      }
                    </section>
                    <p class="muted">
                      Save preserves the terminal ending, party state, treasure,
                      opened door, checkpoint, discoveries, and every encounter
                      outcome.
                    </p>
                    <button
                      class="danger"
                      type="button"
                      [disabled]="store.busy() || saveStatus() === null"
                      (click)="openResetDialog($event)"
                    >
                      Reset / New Adventure
                    </button>
                  </section>
                }
              } @else {
                <section class="encounter-meta" aria-label="Encounter identity">
                  <span>Round {{ encounter().round }}</span>
                  <span>
                    {{
                      encounter().currentFaction === "party"
                        ? encounter().currentActor?.name + " acting"
                        : encounter().currentFaction === "opposition"
                          ? encounter().currentActor?.name + " acting"
                          : "Encounter resolved"
                    }}
                  </span>
                  <span>State revision {{ game()?.revision }}</span>
                  @for (
                    defense of activePartyMember()?.loadout?.defenses ?? [];
                    track defense.id
                  ) {
                    <span>{{ defense.label }} defense {{ defense.value }}</span>
                  }
                  <span
                    >Engine <code>{{ game()?.engineRevisionShort }}</code></span
                  >
                  <span>
                    Rules
                    <code [title]="game()?.rulesetFingerprint">{{
                      game()?.campaign?.title
                    }}</code>
                  </span>
                </section>

                <section class="characters" aria-label="Character status">
                  @for (
                    participant of encounter().participants;
                    track participant.character.id
                  ) {
                    <article
                      class="character-card"
                      [class.defeated]="participant.defeated"
                      [attr.data-faction]="participant.faction"
                    >
                      <p class="meta-label">
                        {{ participant.faction }} · initiative
                        {{ participant.initiative }}
                        @if (
                          participant.character.id ===
                          encounter().currentActorId
                        ) {
                          · acting
                        }
                        @if (participant.defeated) {
                          · defeated
                        }
                      </p>
                      <aui-character-status
                        [status]="characterStatus(participant.character)"
                      />
                      <div
                        class="resources"
                        [attr.aria-label]="
                          participant.character.name + ' resources'
                        "
                      >
                        @for (
                          resource of participant.character.resources;
                          track resource.id
                        ) {
                          <span class="resource-chip">
                            {{ resource.label }} {{ resource.current }}/{{
                              resource.maximum
                            }}
                          </span>
                        }
                      </div>
                    </article>
                  }
                </section>

                @if (game()?.campaign?.phase === "outcome") {
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
                        } @else if (outcome.kind === "defeat") {
                          <p class="muted">
                            Returning to camp applies bounded recovery without
                            granting a reward.
                          </p>
                        } @else {
                          <p class="muted">
                            This victory advances the campaign without granting
                            another item.
                          </p>
                        }
                        <button
                          class="primary"
                          type="button"
                          [disabled]="store.busy()"
                          (click)="returnToCamp()"
                        >
                          {{
                            outcome.kind === "victory" &&
                            game()?.exploration !== null
                              ? "Continue adventure"
                              : "Return to " + game()?.campaign?.title + " Camp"
                          }}
                        </button>
                      </article>
                      <aside class="rusty-engine-panel outcome">
                        <aui-combat-log [entries]="combatLog()" />
                        @if (latestLog(); as latest) {
                          <section
                            class="latest"
                            aria-label="Latest outcome explanation"
                          >
                            <p class="meta-label">
                              Latest receipt · turn {{ latest.turn }}
                            </p>
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
                      <aui-tactical-board
                        [view]="tacticalBoard()"
                        (cellSelected)="selectTacticalCell($event)"
                      />
                      <section class="rusty-engine-panel">
                        <header class="actions__header">
                          <div>
                            <p class="meta-label">
                              {{
                                encounter().currentFaction === "party"
                                  ? "Authored party actions"
                                  : "Rust-owned opposition"
                              }}
                            </p>
                            <h2>
                              {{
                                encounter().currentFaction === "party"
                                  ? "Choose an action"
                                  : encounter().reactionPrompt === null
                                    ? opponentName() + " is ready"
                                    : "Respond to " + opponentName()
                              }}
                            </h2>
                          </div>
                          @if (encounter().currentFaction === "party") {
                            <div class="target-control">
                              <label for="target">Target</label>
                              <select
                                id="target"
                                [value]="targetId()"
                                (change)="selectTarget($event)"
                              >
                                @for (
                                  target of encounter().targets;
                                  track target.id
                                ) {
                                  <option [value]="target.id">
                                    {{ target.name }}
                                  </option>
                                }
                              </select>
                            </div>
                          }
                        </header>

                        @if (encounter().currentFaction === "party") {
                          <aui-hotbar
                            [slots]="hotbarSlots()"
                            (slotSelected)="chooseAction($event)"
                          />

                          <button
                            type="button"
                            [disabled]="
                              store.busy() ||
                              encounter().reactionPrompt !== null
                            "
                            (click)="endActivation()"
                          >
                            End {{ encounter().currentActor?.name }} activation
                          </button>

                          <div class="action-catalog">
                            @for (
                              action of encounter().actions;
                              track action.id
                            ) {
                              <div class="action-note">
                                <strong>{{ action.label }}</strong>
                                <span>
                                  {{ action.ability }} vs {{ action.defense }} ·
                                  {{ action.damage }}
                                  · range {{ action.range }} ·
                                  {{ action.activation.join(" + ") }} ·
                                  {{ action.target }}
                                  @if (action.implement !== null) {
                                    · {{ action.implement }}
                                  }
                                  @if (action.effect !== null) {
                                    · {{ action.effect }}
                                  }
                                  @if (action.forcedMovement > 0) {
                                    · pushes {{ action.forcedMovement }}
                                  }
                                </span>
                              </div>
                            }
                          </div>
                        } @else if (encounter().reactionPrompt === null) {
                          <p class="lede">
                            Begin the explicit opposition phase to let Rust
                            choose and resolve {{ opponentName() }}'s action
                            from admitted definitions. If a reaction is
                            available, Rust will pause for that gameplay choice
                            before resolving the roll.
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

                      @if (encounter().reactionPrompt; as prompt) {
                        <section
                          class="rusty-engine-panel preview"
                          aria-label="Available reaction"
                        >
                          <p class="meta-label">
                            Reaction window ·
                            {{ pendingActorName(prompt.actorId) }} ·
                            {{ prompt.actionLabel }}
                          </p>
                          <p class="preview__math">
                            Ability {{ prompt.abilityScore }} ({{
                              signed(prompt.abilityModifier)
                            }}) against defense {{ prompt.defense }}
                          </p>
                          <div>
                            <h3>Defense attribution</h3>
                            <ul class="source-list">
                              @for (
                                source of prompt.defenseSources;
                                track source
                              ) {
                                <li>{{ source }}</li>
                              }
                            </ul>
                          </div>

                          @if (prompt.reactions.length > 0) {
                            <div
                              class="reaction-list"
                              aria-label="Available reactions"
                            >
                              @for (
                                reaction of prompt.reactions;
                                track reaction.id
                              ) {
                                <button
                                  class="reaction"
                                  type="button"
                                  [disabled]="store.busy()"
                                  (click)="
                                    applyReaction(prompt.token, reaction.id)
                                  "
                                >
                                  {{ reaction.label }} · {{ reaction.cost }}
                                  {{ reaction.resource }} ·
                                  {{ signed(reaction.bonus) }} defense
                                </button>
                              }
                            </div>
                          }

                          <button
                            class="primary resolve"
                            type="button"
                            [disabled]="store.busy()"
                            (click)="declineReaction(prompt.token)"
                          >
                            Do not react
                          </button>
                        </section>
                      }
                    </div>

                    <aside class="rusty-engine-panel outcome">
                      <aui-combat-log [entries]="combatLog()" />
                      @if (latestLog(); as latest) {
                        <section
                          class="latest"
                          aria-label="Latest outcome explanation"
                        >
                          <p class="meta-label">
                            Latest receipt · turn {{ latest.turn }}
                          </p>
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
        </div>
      </div>
    </main>
  `,
})
export class MainMenuScreenComponent implements OnInit {
  protected readonly store = inject(SessionStore);
  private readonly selectedTarget = signal<number | null>(null);
  private readonly selectedLoadoutItem = signal<number | null>(null);
  private readonly selectedPartyMember = signal<number | null>(null);
  protected readonly campaignEntered = signal(false);
  private readonly resetDialog =
    viewChild.required<ElementRef<HTMLDialogElement>>("resetDialog");
  private readonly resetCancelButton =
    viewChild.required<ElementRef<HTMLButtonElement>>("resetCancelButton");
  private readonly resetConfirmButton =
    viewChild.required<ElementRef<HTMLButtonElement>>("resetConfirmButton");
  private readonly newAdventureHeading = viewChild<
    ElementRef<HTMLHeadingElement>
  >("newAdventureHeading");
  private readonly injector = inject(Injector);
  private readonly animationFrame = browserAnimationFrame;
  private readonly clock = browserClock;
  private resetDialogTrigger: HTMLElement | null = null;
  protected readonly compassMarkers: readonly CompassMarkerView[] = [];

  protected readonly game = computed(() => {
    const state = this.store.session();
    return state.kind === "data" ? state.value : null;
  });

  protected readonly saveStatus = computed(() => {
    const state = this.store.saveStatus();
    return state.kind === "data" ? state.value : null;
  });

  protected readonly activePartyMember = computed(() => {
    const party = this.game()?.campaign?.party ?? [];
    const selected = this.selectedPartyMember();
    return (
      party.find((member) => member.character.id === selected) ??
      party[0] ??
      null
    );
  });

  protected readonly dungeonViewport = computed<DungeonViewportView>(() => {
    const exploration = this.game()?.exploration;
    if (exploration === null || exploration === undefined) {
      throw new Error("Dungeon exploration is not available.");
    }
    return {
      title: exploration.dungeonTitle,
      wallStyle: exploration.wallStyle,
      facing: exploration.facing,
      x: exploration.x,
      y: exploration.y,
      depths: exploration.view,
    };
  });

  protected readonly gameViewport = computed<GameViewportView>(() => {
    const state = this.store.session();
    if (state.kind === "error") {
      return {
        mode: "error",
        label: "Runtime failure backdrop",
        dungeon: null,
      };
    }
    if (state.kind !== "data") {
      return {
        mode: "loading",
        label: "Loading the Rust-owned game session",
        dungeon: null,
      };
    }

    const snapshot = state.value;
    if (snapshot.campaign === null || !this.campaignEntered()) {
      return {
        mode: "catalog",
        label:
          snapshot.campaign === null
            ? "Choose a Rust-compiled adventure"
            : `Continue ${snapshot.campaign.title}`,
        dungeon: null,
      };
    }

    if (
      snapshot.campaign.phase === "exploration" &&
      snapshot.exploration !== null
    ) {
      const dungeon = this.dungeonViewport();
      return {
        mode: "exploration",
        label: `${dungeon.title}, facing ${dungeon.facing} at cell ${dungeon.x}, ${dungeon.y}`,
        dungeon,
      };
    }

    return {
      mode:
        snapshot.campaign.phase === "adventure-complete"
          ? "complete"
          : snapshot.campaign.phase,
      label:
        snapshot.campaign.phase === "camp"
          ? `${snapshot.campaign.title} camp`
          : snapshot.campaign.phase === "outcome"
            ? `${snapshot.campaign.title} encounter outcome`
            : `${snapshot.campaign.title} tactical encounter`,
      dungeon: null,
    };
  });

  protected readonly compassHeading = computed(() => {
    const facing = this.game()?.exploration?.facing;
    return facing === "east"
      ? 90
      : facing === "south"
        ? 180
        : facing === "west"
          ? 270
          : 0;
  });

  protected readonly minimapMarkers = computed<readonly MinimapMarkerView[]>(
    () => {
      const exploration = this.game()?.exploration;
      if (exploration === null || exploration === undefined) {
        return [];
      }
      const xDivisor = Math.max(1, exploration.width - 1);
      const yDivisor = Math.max(1, exploration.height - 1);
      return exploration.discoveredCells
        .filter((cell) => cell.x !== exploration.x || cell.y !== exploration.y)
        .map((cell) => ({
          id: `${cell.x}:${cell.y}`,
          label: `Discovered cell ${cell.x},${cell.y}`,
          kind: "poi" as const,
          x: (cell.x / xDivisor) * 100,
          y: (cell.y / yDivisor) * 100,
        }));
    },
  );

  protected readonly minimapPlayerX = computed(() => {
    const exploration = this.game()?.exploration;
    return exploration === null || exploration === undefined
      ? 50
      : (exploration.x / Math.max(1, exploration.width - 1)) * 100;
  });

  protected readonly minimapPlayerY = computed(() => {
    const exploration = this.game()?.exploration;
    return exploration === null || exploration === undefined
      ? 50
      : (exploration.y / Math.max(1, exploration.height - 1)) * 100;
  });

  protected readonly tacticalBoard = computed<TacticalBoardView>(() => {
    const encounter = this.game()?.encounter;
    if (encounter === null || encounter === undefined) {
      throw new Error("Tactical encounter is not available.");
    }
    const participants = new Map(
      encounter.participants.map((participant) => [
        `${participant.x}:${participant.y}`,
        participant,
      ]),
    );
    const legalMoves = new Map(
      encounter.board.legalMoves.map((move) => [
        `${move.x}:${move.y}`,
        move.cost,
      ]),
    );
    const cells = encounter.board.rows.flatMap((row, y) =>
      [...row].map((terrain, x) => {
        const participant = participants.get(`${x}:${y}`);
        return {
          id: `${x}:${y}`,
          x,
          y,
          terrain: terrain === "#" ? ("wall" as const) : ("floor" as const),
          participantId: participant?.character.id ?? null,
          participantName: participant?.character.name ?? null,
          faction: participant?.faction ?? null,
          defeated: participant?.defeated ?? false,
          current: participant?.character.id === encounter.currentActorId,
          selectedTarget: participant?.character.id === this.selectedTarget(),
          selectable:
            encounter.currentFaction === "party" &&
            participant?.faction === "opposition" &&
            participant.defeated === false,
          legalMoveCost: legalMoves.get(`${x}:${y}`) ?? null,
        };
      }),
    );
    return {
      width: encounter.board.width,
      height: encounter.board.height,
      cells,
    };
  });

  protected readonly inventorySlots = computed<
    readonly (InventoryItemView | null)[]
  >(() =>
    (this.activePartyMember()?.loadout.inventorySlots ?? []).map((item) =>
      item === null ? null : this.inventoryItem(item),
    ),
  );

  protected readonly selectedInventoryItemId = computed(() => {
    const selected = this.selectedLoadoutItem();
    return selected === null ? null : String(selected);
  });

  protected readonly equipmentSlots = computed<readonly EquipmentSlotView[]>(
    () =>
      (this.activePartyMember()?.loadout.equipmentSlots ?? []).map((slot) => ({
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
    (this.activePartyMember()?.loadout.inventorySlots ?? []).filter(
      (item): item is LoadoutItemDto => item !== null,
    ),
  );

  protected readonly hotbarSlots = computed<readonly HotbarSlotView[]>(() =>
    (this.game()?.encounter?.actions ?? []).map((action, index) => ({
      index,
      keybind: String(index + 1),
      label: action.label,
      icon: index === 0 ? "⚔" : "➶",
      empty: false,
    })),
  );

  protected readonly combatLog = computed<readonly CombatLogEntryView[]>(() =>
    (this.game()?.encounter?.log ?? []).map((entry) => ({
      id: entry.id,
      source: `T${entry.turn} ${entry.source}`,
      text: entry.text,
      severity:
        entry.kind === "hit"
          ? "hit"
          : entry.kind === "miss"
            ? "miss"
            : entry.kind === "system"
              ? "system"
              : "info",
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
      throw new Error("Encounter is not available.");
    }
    return encounter;
  }

  protected sessionError() {
    const state = this.store.session();
    if (state.kind !== "error") {
      throw new Error("Session error is not available.");
    }
    return state.error;
  }

  protected characterStatus(character: CharacterDto): CharacterStatusView {
    const resource = character.resources.find(
      (entry) => entry.id === "guard",
    ) ??
      character.resources[0] ?? { label: "Resource", current: 0, maximum: 0 };
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

  protected selectPartyMember(memberId: number): void {
    this.selectedPartyMember.set(memberId);
    this.selectedLoadoutItem.set(null);
  }

  protected selectTarget(event: Event): void {
    const target = event.target;
    if (target instanceof HTMLSelectElement) {
      this.selectedTarget.set(Number(target.value));
    }
  }

  protected selectTacticalCell(selection: TacticalBoardSelection): void {
    const encounter = this.encounter();
    if (selection.participantId !== null) {
      const participant = encounter.participants.find(
        (entry) => entry.character.id === selection.participantId,
      );
      if (participant?.faction === "opposition" && !participant.defeated) {
        this.selectedTarget.set(participant.character.id);
      }
      return;
    }
    const actor = encounter.currentActorId;
    if (
      actor !== null &&
      encounter.currentFaction === "party" &&
      encounter.board.legalMoves.some(
        (move) => move.x === selection.x && move.y === selection.y,
      )
    ) {
      void this.store.moveActor(actor, selection.x, selection.y);
    }
  }

  protected chooseAction(slot: HotbarSlotView): void {
    const action = this.encounter().actions[slot.index];
    if (action !== undefined) {
      const targets =
        this.encounter().legalTargets.find(
          (entry) => entry.actionId === action.id,
        )?.targetIds ?? [];
      const target = targets.includes(this.targetId())
        ? this.targetId()
        : (targets[0] ?? 0);
      const actor = this.encounter().currentActorId;
      if (actor !== null && target !== 0) {
        this.selectedTarget.set(target);
        void this.store.chooseAction(action.id, actor, target);
      }
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

  protected beginExploration(): void {
    void this.store.beginExploration();
  }

  protected explorationCommand(command: ExplorationCommandKindDto): void {
    void this.store.explorationCommand(command);
  }

  protected handleExplorationKeydown(event: KeyboardEvent): void {
    if (this.game()?.campaign?.phase !== "exploration" || this.store.busy()) {
      return;
    }
    const target = event.target;
    if (
      target instanceof HTMLInputElement ||
      target instanceof HTMLSelectElement ||
      target instanceof HTMLTextAreaElement ||
      target instanceof HTMLButtonElement
    ) {
      return;
    }
    const command: ExplorationCommandKindDto | undefined =
      event.key === "ArrowUp" || event.key.toLowerCase() === "w"
        ? "step-forward"
        : event.key === "ArrowDown" || event.key.toLowerCase() === "s"
          ? "step-backward"
          : event.key === "ArrowLeft" || event.key.toLowerCase() === "a"
            ? "turn-left"
            : event.key === "ArrowRight" || event.key.toLowerCase() === "d"
              ? "turn-right"
              : event.key.toLowerCase() === "e"
                ? "interact"
                : undefined;
    if (command !== undefined) {
      event.preventDefault();
      this.explorationCommand(command);
    }
  }

  protected activateInventoryItem(item: InventoryItemView): void {
    const authoritative = this.findCarriedItem(item.id);
    if (authoritative === undefined) {
      return;
    }
    this.selectedLoadoutItem.set(authoritative.entityId);
    if (authoritative.equippedSlotId === null) {
      void this.store.equipItem(
        authoritative.entityId,
        authoritative.equipmentSlotId,
      );
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
    const loadout = this.activePartyMember()?.loadout;
    if (loadout !== undefined && item.equippedSlotId === null) {
      void this.store.transferItem(
        item.entityId,
        loadout.ownerId,
        loadout.stashOwnerId,
      );
    }
  }

  protected takeItem(item: LoadoutItemDto): void {
    const loadout = this.activePartyMember()?.loadout;
    if (loadout !== undefined) {
      void this.store.transferItem(
        item.entityId,
        loadout.stashOwnerId,
        loadout.ownerId,
      );
    }
  }

  protected applyReaction(token: string, reactionId: string): void {
    void this.store.applyReaction(token, reactionId);
  }

  protected declineReaction(token: string): void {
    void this.store.declineReaction(token);
  }

  protected beginOppositionTurn(): void {
    void this.store.beginOppositionTurn();
  }

  protected endActivation(): void {
    void this.store.endActivation();
  }

  protected returnToCamp(): void {
    void this.store.returnToCamp();
  }

  protected save(): void {
    void this.store.save();
  }

  protected openResetDialog(event: Event): void {
    if (this.saveStatus() !== null) {
      this.resetDialogTrigger =
        event.currentTarget instanceof HTMLElement ? event.currentTarget : null;
      const dialog = this.resetDialog().nativeElement;
      if (!dialog.open) {
        dialog.showModal();
        this.resetCancelButton().nativeElement.focus();
      }
      this.animationFrame.request(() => {
        this.animationFrame.request(() => {
          if (dialog.open && !dialog.matches(":focus-within")) {
            this.resetCancelButton().nativeElement.focus();
          }
        });
      });
    }
  }

  protected cancelReset(event?: Event): void {
    event?.preventDefault();
    this.closeResetDialog(true);
  }

  protected handleResetDialogKeydown(event: KeyboardEvent): void {
    if (event.key !== "Tab") {
      return;
    }

    event.preventDefault();
    const cancelButton = this.resetCancelButton().nativeElement;
    const confirmButton = this.resetConfirmButton().nativeElement;
    if (event.shiftKey) {
      (event.target === cancelButton ? confirmButton : cancelButton).focus();
    } else {
      (event.target === confirmButton ? cancelButton : confirmButton).focus();
    }
  }

  protected async confirmReset(): Promise<void> {
    await this.store.resetSession();
    if (this.game()?.campaign === null) {
      this.campaignEntered.set(false);
      this.closeResetDialog(false);
      afterNextRender(
        () => {
          this.clock.setTimeout(
            () => this.newAdventureHeading()?.nativeElement.focus(),
            0,
          );
        },
        { injector: this.injector },
      );
    }
  }

  private closeResetDialog(restoreTrigger: boolean): void {
    const dialog = this.resetDialog().nativeElement;
    if (dialog.open) {
      dialog.close();
    }

    if (restoreTrigger && this.resetDialogTrigger?.isConnected) {
      this.resetDialogTrigger.focus();
    }
    this.resetDialogTrigger = null;
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
      this.encounter().participants.find(
        (entry) => entry.character.id === actorId,
      )?.character.name ?? `Entity ${actorId}`
    );
  }

  protected opponentName(): string {
    return this.encounter().currentFaction === "opposition"
      ? (this.encounter().currentActor?.name ?? "Opposition")
      : (this.encounter().targets[0]?.name ?? "Opposition");
  }

  private inventoryItem(item: LoadoutItemDto): InventoryItemView {
    return {
      id: String(item.entityId),
      name:
        item.equippedSlotId === null
          ? item.name
          : `${item.name} · equipped ${item.equippedSlotId}`,
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
