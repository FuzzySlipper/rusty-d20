import { InjectionToken, Injectable, signal } from '@angular/core';
import type { Provider, Signal } from '@angular/core';
import { projectRuntimeReadout, type RuntimeReadoutView } from '@rusty-d20/domain';
import { browserHttp } from '@rusty-d20/platform';
import type { ClassifiedError } from '@rusty-d20/protocol';
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
  private readonly _readout = signal<AsyncState<RuntimeReadoutView>>({ kind: 'idle' });
  readonly readout: Signal<AsyncState<RuntimeReadoutView>> = this._readout.asReadonly();

  constructor(private readonly transport: RustyD20Transport) {}

  async load(): Promise<void> {
    this._readout.set({ kind: 'loading' });
    const result = await this.transport.loadReadout();
    this._readout.set(
      result.ok ? { kind: 'data', value: projectRuntimeReadout(result.value) } : { kind: 'error', error: result.error },
    );
  }
}

export function provideRustyD20StoreKernel(): Provider[] {
  return [
    { provide: RUSTY_D20_TRANSPORT, useFactory: () => createHttpRustyD20Transport(browserHttp()) },
    {
      provide: SessionStore,
      deps: [RUSTY_D20_TRANSPORT],
      useFactory: (transport: RustyD20Transport) => new SessionStore(transport),
    },
  ];
}
