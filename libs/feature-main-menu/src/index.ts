import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import type { OnInit } from '@angular/core';
import { SessionStore } from '@rusty-d20/store';

@Component({
  changeDetection: ChangeDetectionStrategy.OnPush,
  selector: 'aui-main-menu-screen',
  standalone: true,
  styles: [
    `
      :host {
        display: grid;
        min-height: 100vh;
        place-items: center;
        padding: 24px;
      }

      .shell {
        display: grid;
        gap: 20px;
        max-width: 720px;
        width: min(100%, 720px);
      }

      .eyebrow {
        color: var(--rusty-engine-accent);
        font-size: 0.75rem;
        font-weight: 700;
        letter-spacing: 0.12em;
        margin: 0;
        text-transform: uppercase;
      }

      h1 {
        font-size: clamp(2.4rem, 9vw, 5rem);
        line-height: 0.95;
        margin: 0;
      }

      .lede {
        color: var(--rusty-engine-muted);
        font-size: 1.05rem;
        margin: 0;
        max-width: 58ch;
      }

      .readout {
        background: var(--rusty-engine-surface);
        border: 1px solid var(--rusty-engine-border);
        border-radius: var(--rusty-engine-radius);
        display: grid;
        gap: 12px;
        padding: 20px;
      }

      .readout__header {
        align-items: center;
        display: flex;
        gap: 12px;
        justify-content: space-between;
      }

      .readout__header h2,
      .readout p {
        margin: 0;
      }

      .status {
        align-items: center;
        color: var(--rusty-engine-accent);
        display: inline-flex;
        font-size: 0.82rem;
        font-weight: 700;
        gap: 8px;
      }

      .status::before {
        background: currentColor;
        border-radius: 50%;
        content: '';
        height: 8px;
        width: 8px;
      }

      dl {
        display: grid;
        gap: 12px;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        margin: 0;
      }

      div {
        min-width: 0;
      }

      dt {
        color: var(--rusty-engine-muted);
        font-size: 0.72rem;
        letter-spacing: 0.08em;
        text-transform: uppercase;
      }

      dd {
        margin: 4px 0 0;
        overflow-wrap: anywhere;
      }

      code {
        color: var(--rusty-engine-cool);
      }

      .error {
        border-color: var(--rusty-engine-danger);
      }

      .error__kind {
        color: var(--rusty-engine-danger);
        font-weight: 700;
        text-transform: capitalize;
      }

      button {
        background: var(--rusty-engine-accent-strong);
        border: 1px solid var(--rusty-engine-accent);
        border-radius: var(--rusty-engine-radius-sm);
        color: var(--rusty-engine-text);
        cursor: pointer;
        justify-self: start;
        padding: 8px 14px;
      }

      @media (max-width: 560px) {
        dl {
          grid-template-columns: minmax(0, 1fr);
        }
      }
    `,
  ],
  template: `
    <main class="shell">
      <header>
        <p class="eyebrow">Rust-owned reference consumer</p>
        <h1>Rusty D20</h1>
        <p class="lede">
          A concrete d20 game downstream of Rusty Engine. The browser observes authoritative Rust
          state through one typed transport.
        </p>
      </header>

      @switch (store.readout().kind) {
        @case ('idle') {
          <section class="readout" aria-live="polite">
            <p>Preparing runtime connection…</p>
          </section>
        }
        @case ('loading') {
          <section class="readout" aria-live="polite" aria-busy="true">
            <p>Loading Rust runtime readout…</p>
          </section>
        }
        @case ('data') {
          <section class="readout" aria-label="Rust runtime readout">
            <header class="readout__header">
              <h2>{{ dataValue().product }}</h2>
              <span class="status">{{ dataValue().statusLabel }}</span>
            </header>
            <dl>
              <div>
                <dt>Game version</dt>
                <dd>{{ dataValue().version }}</dd>
              </div>
              <div>
                <dt>Canonical entities</dt>
                <dd>{{ dataValue().entityCount }}</dd>
              </div>
              <div>
                <dt>Rusty Engine revision</dt>
                <dd>
                  <code [title]="dataValue().engineRevision">{{ dataValue().engineRevisionShort }}</code>
                </dd>
              </div>
              <div>
                <dt>Transport</dt>
                <dd>Same-origin HTTP</dd>
              </div>
            </dl>
          </section>
        }
        @case ('error') {
          <section class="readout error" role="alert">
            <p class="error__kind">{{ errorValue().kind }} failure</p>
            <p>{{ errorValue().message }}</p>
            @if (errorValue().retryable) {
              <button type="button" (click)="reload()">Retry connection</button>
            }
          </section>
        }
      }
    </main>
  `,
})
export class MainMenuScreenComponent implements OnInit {
  protected readonly store = inject(SessionStore);

  ngOnInit(): void {
    void this.store.load();
  }

  protected dataValue() {
    const state = this.store.readout();
    if (state.kind !== 'data') {
      throw new Error('Runtime readout is not available.');
    }
    return state.value;
  }

  protected errorValue() {
    const state = this.store.readout();
    if (state.kind !== 'error') {
      throw new Error('Runtime error is not available.');
    }
    return state.error;
  }

  protected reload(): void {
    void this.store.load();
  }
}
