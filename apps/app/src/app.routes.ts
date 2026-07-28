import type { Routes } from '@angular/router';
import { MainMenuScreenComponent } from '@rusty-d20/feature-main-menu';
import { SHELL_PATHS } from '@rusty-d20/shell';

/**
 * Application routes: binds the shell-owned paths to the feature screens.
 * Route wiring lives in the app because only `type:app` projects may depend
 * on `type:feature` libraries.
 */
export const appRoutes: Routes = [
  { path: SHELL_PATHS.mainMenu, component: MainMenuScreenComponent, title: 'Main Menu' },
  { path: '**', redirectTo: SHELL_PATHS.mainMenu },
];
