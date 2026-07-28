import { Component } from '@angular/core';
import { RouterOutlet } from '@angular/router';

@Component({
  imports: [RouterOutlet],
  selector: 'aui-root',
  styles: [
    `
      :host {
        display: block;
        min-height: 100vh;
      }
    `,
  ],
  template: `<router-outlet />`,
})
export class AppComponent {}
