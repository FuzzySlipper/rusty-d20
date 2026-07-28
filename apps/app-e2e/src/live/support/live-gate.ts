import { test } from '@playwright/test';

export interface LiveBaseUrl {
  readonly value: string;
  readonly provenance: 'env';
}

export function isLiveRun(): boolean {
  return process.env['LIVE_RUN'] === '1';
}

export function requireLiveRun(): void {
  test.skip(!isLiveRun(), 'Set LIVE_RUN=1 and BASE_URL to run live scenarios.');
}

export function resolveLiveBaseUrl(): LiveBaseUrl {
  const baseUrl = process.env['BASE_URL'];
  if (baseUrl !== undefined && baseUrl.length > 0) {
    return { value: baseUrl, provenance: 'env' };
  }

  throw new Error(
    'BASE_URL is required for live scenarios. Run `pnpm run serve:local`, then re-run with BASE_URL=<printed-url> LIVE_RUN=1.',
  );
}
