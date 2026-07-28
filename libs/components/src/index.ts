import { Component, input } from '@angular/core';

@Component({
  selector: 'aui-status-line',
  standalone: true,
  styles: [
    `
      :host {
        display: block;
        color: var(--rusty-engine-muted);
      }
    `,
  ],
  template: `<p>{{ label() }}</p>`,
})
export class StatusLineComponent {
  readonly label = input.required<string>();
}
