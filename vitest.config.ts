import { defineConfig } from 'vitest/config';
import { fileURLToPath } from 'node:url';

export default defineConfig({
  resolve: {
    alias: {
      '@rusty-d20/domain': fileURLToPath(new URL('./libs/domain/src/index.ts', import.meta.url)),
      '@rusty-d20/platform': fileURLToPath(new URL('./libs/platform/src/index.ts', import.meta.url)),
      '@rusty-d20/protocol': fileURLToPath(new URL('./libs/protocol/src/index.ts', import.meta.url)),
      '@rusty-d20/transport': fileURLToPath(new URL('./libs/transport/src/index.ts', import.meta.url)),
    },
  },
  test: {
    include: ['libs/**/*.spec.ts'],
    exclude: ['**/node_modules/**', 'dist/**', 'coverage/**', 'apps/app-e2e/**'],
  },
});
