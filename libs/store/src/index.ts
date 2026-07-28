import { Injectable, InjectionToken, signal } from '@angular/core';
import type { Provider, Signal } from '@angular/core';
import {
  projectGameSnapshot,
  projectRuntimeReadout,
  type GameSnapshotView,
  type RuntimeReadoutView,
} from '@rusty-d20/domain';
import { browserHttp } from '@rusty-d20/platform';
import type { ClassifiedError, GameSnapshotDto, Result } from '@rusty-d20/protocol';
import { createHttpRustyD20Transport, type RustyD20Transport } from '@rusty-d20/transport';

export type AsyncState<T> =
  | { readonly kind: 'idle' }
  | { readonly kind: 'loading' }
  | { readonly kind: 'data'; readonly value: T }
  | { readonly kind: 'error'; readonly error: ClassifiedError };

export const RUSTY_D20_TRANSPORT = new InjectionToken<RustyD20Transport>('RUSTY_D20_TRANSPORT', {
  factory: () => createHttpRustyD20Transport(browserHttp()),
});

@Injectable()
export class SessionStore {
  private readonly _readout = signal<AsyncState<RuntimeReadoutView>>({
    kind: 'idle',
  });
  private readonly _session = signal<AsyncState<GameSnapshotView>>({
    kind: 'idle',
  });
  private readonly _commandError = signal<ClassifiedError | null>(null);
  private readonly _busy = signal(false);
  private generation = 0;

  readonly readout: Signal<AsyncState<RuntimeReadoutView>> = this._readout.asReadonly();
  readonly session: Signal<AsyncState<GameSnapshotView>> = this._session.asReadonly();
  readonly commandError: Signal<ClassifiedError | null> = this._commandError.asReadonly();
  readonly busy: Signal<boolean> = this._busy.asReadonly();

  constructor(private readonly transport: RustyD20Transport) {}

  async loadReadout(): Promise<void> {
    this._readout.set({ kind: 'loading' });
    const result = await this.transport.loadReadout();
    this._readout.set(
      result.ok
        ? { kind: 'data', value: projectRuntimeReadout(result.value) }
        : { kind: 'error', error: result.error },
    );
  }

  async load(): Promise<void> {
    const generation = ++this.generation;
    if (this._session().kind !== 'data') {
      this._session.set({ kind: 'loading' });
    }
    this._busy.set(true);
    const result = await this.transport.loadSession();
    this.publish(generation, result, true);
  }

  async newAdventure(): Promise<void> {
    await this.mutate((revision) => this.transport.newAdventure(revision));
  }

  async enterEncounter(encounterId: string): Promise<void> {
    await this.mutate((expectedRevision) =>
      this.transport.enterEncounter({ expectedRevision, encounterId }),
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

  async transferItem(itemId: number, fromOwnerId: number, toOwnerId: number): Promise<void> {
    await this.mutate((expectedRevision) =>
      this.transport.transferItem({
        expectedRevision,
        itemId,
        fromOwnerId,
        toOwnerId,
      }),
    );
  }

  async previewAction(actionId: string, actorId: number, targetId: number): Promise<void> {
    await this.mutate((expectedRevision) =>
      this.transport.previewAction({
        expectedRevision,
        actorId,
        targetId,
        actionId,
      }),
    );
  }

  async applyReaction(previewToken: string, reactionId: string): Promise<void> {
    await this.mutate((expectedRevision) =>
      this.transport.applyReaction({
        expectedRevision,
        previewToken,
        reactionId,
      }),
    );
  }

  async applyAction(previewToken: string): Promise<void> {
    await this.mutate((expectedRevision) =>
      this.transport.applyAction({ expectedRevision, previewToken }),
    );
  }

  async beginOppositionTurn(): Promise<void> {
    await this.mutate((revision) => this.transport.beginOppositionTurn(revision));
  }

  async returnToCamp(): Promise<void> {
    await this.mutate((revision) => this.transport.returnToCamp(revision));
  }

  async save(): Promise<void> {
    await this.mutate((revision) => this.transport.save(revision));
  }

  clearCommandError(): void {
    this._commandError.set(null);
  }

  private async mutate(
    request: (revision: number) => Promise<Result<GameSnapshotDto>>,
  ): Promise<void> {
    if (this._busy()) {
      return;
    }
    const state = this._session();
    if (state.kind !== 'data') {
      return;
    }
    const generation = ++this.generation;
    this._busy.set(true);
    this._commandError.set(null);
    const result = await request(state.value.revision);
    this.publish(generation, result, false);
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
        kind: 'data',
        value: projectGameSnapshot(result.value),
      });
      return;
    }
    if (replaceOnError && this._session().kind !== 'data') {
      this._session.set({ kind: 'error', error: result.error });
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
