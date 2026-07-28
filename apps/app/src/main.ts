import { bootstrapApplication } from '@angular/platform-browser';
import { provideRouter } from '@angular/router';
import { provideRustyD20StoreKernel } from '@rusty-d20/store';
import { AppComponent } from './app.component';
import { appRoutes } from './app.routes';

bootstrapApplication(AppComponent, {
  providers: [provideRouter(appRoutes), provideRustyD20StoreKernel()],
}).catch((error: unknown) => {
  console.error(error);
});
