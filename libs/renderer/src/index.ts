import { Component, input } from '@angular/core';
import { StatusLineComponent } from '@rusty-d20/components';
import type { RuntimeReadoutView } from '@rusty-d20/domain';

@Component({
  imports: [StatusLineComponent],
  selector: 'aui-status-renderer',
  standalone: true,
  styles: [
    `
      :host {
        display: block;
      }
    `,
  ],
  template: `<aui-status-line [label]="status().statusLabel" />`,
})
export class StatusRendererComponent {
  readonly status = input.required<RuntimeReadoutView>();
}
