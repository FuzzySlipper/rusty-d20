import { Injectable, InjectionToken, signal } from "@angular/core";
import type { Provider, Signal } from "@angular/core";
import {
  projectGameSnapshot,
  projectRuntimeReadout,
  type GameSnapshotView,
  type RuntimeReadoutView,
} from "@rusty-d20/domain";
import { browserHttp } from "@rusty-d20/platform";
import type {
  ClassifiedError,
  ExplorationCommandKindDto,
  GameSnapshotDto,
  Result,
  SaveStatusDto,
} from "@rusty-d20/protocol";
import {
  createHttpRustyD20Transport,
  type RustyD20Transport,
} from "@rusty-d20/transport";

export type AsyncState<T> =
  | { readonly kind: "idle" }
  | { readonly kind: "loading" }
  | { readonly kind: "data"; readonly value: T }
  | { readonly kind: "error"; readonly error: ClassifiedError };

export const RUSTY_D20_TRANSPORT = new InjectionToken<RustyD20Transport>(
  "RUSTY_D20_TRANSPORT",
  {
    factory: () => createHttpRustyD20Transport(browserHttp()),
  },
);

@Injectable()
export class SessionStore {
  private readonly _readout = signal<AsyncState<RuntimeReadoutView>>({
    kind: "idle",
  });
  private readonly _session = signal<AsyncState<GameSnapshotView>>({
    kind: "idle",
  });
  private readonly _saveStatus = signal<AsyncState<SaveStatusDto>>({
    kind: "idle",
  });
  private readonly _commandError = signal<ClassifiedError | null>(null);
  private readonly _busy = signal(false);
  private generation = 0;

  readonly readout: Signal<AsyncState<RuntimeReadoutView>> =
    this._readout.asReadonly();
  readonly session: Signal<AsyncState<GameSnapshotView>> =
    this._session.asReadonly();
  readonly saveStatus: Signal<AsyncState<SaveStatusDto>> =
    this._saveStatus.asReadonly();
  readonly commandError: Signal<ClassifiedError | null> =
    this._commandError.asReadonly();
  readonly busy: Signal<boolean> = this._busy.asReadonly();

  constructor(private readonly transport: RustyD20Transport) {}

  async loadReadout(): Promise<void> {
    this._readout.set({ kind: "loading" });
    const result = await this.transport.loadReadout();
    this._readout.set(
      result.ok
        ? { kind: "data", value: projectRuntimeReadout(result.value) }
        : { kind: "error", error: result.error },
    );
  }

  async load(): Promise<void> {
    const generation = ++this.generation;
    if (this._session().kind !== "data") {
      this._session.set({ kind: "loading" });
    }
    this._saveStatus.set({ kind: "loading" });
    this._busy.set(true);
    const saveStatus = await this.transport.loadSaveStatus();
    if (generation === this.generation) {
      this._saveStatus.set(
        saveStatus.ok
          ? { kind: "data", value: saveStatus.value }
          : { kind: "error", error: saveStatus.error },
      );
    }
    if (generation !== this.generation) {
      return;
    }
    if (saveStatus.ok && saveStatus.value.state === "recovery-required") {
      this.publish(
        generation,
        {
          ok: false,
          error: {
            kind: "persistence",
            message:
              saveStatus.value.persistenceError ?? "Save recovery is required.",
            retryable: false,
          },
        },
        true,
      );
      return;
    }
    const result = await this.transport.loadSession();
    this.publish(generation, result, true);
  }

  async newAdventure(adventureId: string): Promise<void> {
    await this.mutate((expectedRevision) =>
      this.transport.newAdventure({ expectedRevision, adventureId }),
    );
  }

  async beginExploration(): Promise<void> {
    await this.mutate((expectedRevision) =>
      this.transport.beginExploration(expectedRevision),
    );
  }

  async explorationCommand(command: ExplorationCommandKindDto): Promise<void> {
    await this.mutate((expectedRevision) =>
      this.transport.explorationCommand({ expectedRevision, command }),
    );
  }

  async equipItem(itemId: number, slotId: string): Promise<void> {
    await this.mutate((expectedRevision) =>
      this.transport.equipItem({ expectedRevision, itemId, slotId }),
    );
  }

  async unequipItem(itemId: number): Promise<void> {
    await this.mutate((expectedRevision) =>
      this.transport.unequipItem({ expectedRevision, itemId }),
    );
  }

  async transferItem(
    itemId: number,
    fromOwnerId: number,
    toOwnerId: number,
  ): Promise<void> {
    await this.mutate((expectedRevision) =>
      this.transport.transferItem({
        expectedRevision,
        itemId,
        fromOwnerId,
        toOwnerId,
      }),
    );
  }

  async moveLoadoutItem(
    itemId: number,
    fromOwnerId: number,
    toOwnerId: number,
    destinationSlotId: string | null,
  ): Promise<boolean> {
    return this.mutate((expectedRevision) =>
      this.transport.moveLoadoutItem({
        expectedRevision,
        itemId,
        fromOwnerId,
        toOwnerId,
        destinationSlotId,
      }),
    );
  }

  async chooseAction(
    actionId: string,
    actorId: number,
    targetId: number,
  ): Promise<boolean> {
    return this.mutate((expectedRevision) =>
      this.transport.chooseAction({
        expectedRevision,
        actorId,
        targetId,
        actionId,
      }),
    );
  }

  async moveActor(actorId: number, x: number, y: number): Promise<boolean> {
    return this.mutate((expectedRevision) =>
      this.transport.moveActor({ expectedRevision, actorId, x, y }),
    );
  }

  async applyReaction(promptToken: string, reactionId: string): Promise<void> {
    await this.mutate((expectedRevision) =>
      this.transport.applyReaction({
        expectedRevision,
        promptToken,
        reactionId,
      }),
    );
  }

  async declineReaction(promptToken: string): Promise<void> {
    await this.mutate((expectedRevision) =>
      this.transport.declineReaction({ expectedRevision, promptToken }),
    );
  }

  async beginOppositionTurn(): Promise<void> {
    await this.mutate((revision) =>
      this.transport.beginOppositionTurn(revision),
    );
  }

  async endActivation(): Promise<void> {
    await this.mutate((revision) => this.transport.endActivation(revision));
  }

  async returnToCamp(): Promise<void> {
    await this.mutate((revision) => this.transport.returnToCamp(revision));
  }

  async save(): Promise<void> {
    await this.mutate((revision) => this.transport.save(revision));
  }

  async resetSession(): Promise<void> {
    if (this._busy()) {
      return;
    }
    const status = this._saveStatus();
    if (status.kind !== "data") {
      return;
    }
    const session = this._session();
    const recovery = status.value.state === "recovery-required";
    if (!recovery && session.kind !== "data") {
      return;
    }
    const generation = ++this.generation;
    this._busy.set(true);
    this._commandError.set(null);
    const result = await this.transport.resetSession({
      expectedSaveIdentity: status.value.saveIdentity,
      expectedRevision: recovery
        ? null
        : session.kind === "data"
          ? session.value.revision
          : null,
      expectedAdventureId:
        recovery || session.kind !== "data"
          ? null
          : (session.value.campaign?.id ?? null),
    });
    if (generation !== this.generation) {
      return;
    }
    if (!result.ok) {
      this._busy.set(false);
      this._commandError.set(result.error);
      return;
    }
    const refreshedStatus = await this.transport.loadSaveStatus();
    if (generation !== this.generation) {
      return;
    }
    this._saveStatus.set(
      refreshedStatus.ok
        ? { kind: "data", value: refreshedStatus.value }
        : { kind: "error", error: refreshedStatus.error },
    );
    this._busy.set(false);
    this._commandError.set(null);
    this._session.set({
      kind: "data",
      value: projectGameSnapshot(result.value),
    });
  }

  clearCommandError(): void {
    this._commandError.set(null);
  }

  private async mutate(
    request: (revision: number) => Promise<Result<GameSnapshotDto>>,
  ): Promise<boolean> {
    if (this._busy()) {
      return false;
    }
    const state = this._session();
    if (state.kind !== "data") {
      return false;
    }
    const generation = ++this.generation;
    this._busy.set(true);
    this._commandError.set(null);
    const result = await request(state.value.revision);
    const isCurrent = generation === this.generation;
    this.publish(generation, result, false);
    return isCurrent;
  }

  private publish(
    generation: number,
    result: Result<GameSnapshotDto>,
    replaceOnError: boolean,
  ): void {
    if (generation !== this.generation) {
      return;
    }
    this._busy.set(false);
    if (result.ok) {
      this._commandError.set(null);
      this._session.set({
        kind: "data",
        value: projectGameSnapshot(result.value),
      });
      return;
    }
    if (replaceOnError && this._session().kind !== "data") {
      this._session.set({ kind: "error", error: result.error });
    } else {
      this._commandError.set(result.error);
    }
  }
}

export function provideRustyD20StoreKernel(): Provider[] {
  return [
    {
      provide: RUSTY_D20_TRANSPORT,
      useFactory: () => createHttpRustyD20Transport(browserHttp()),
    },
    {
      provide: SessionStore,
      deps: [RUSTY_D20_TRANSPORT],
      useFactory: (transport: RustyD20Transport) => new SessionStore(transport),
    },
  ];
}
