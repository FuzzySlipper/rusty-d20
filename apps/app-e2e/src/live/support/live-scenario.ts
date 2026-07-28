import { test as base } from '@playwright/test';
import { ArtifactCollector } from './artifact-collector';
import { requireLiveRun, resolveLiveBaseUrl } from './live-gate';

export const liveScenario = base.extend<{ collector: ArtifactCollector; liveBaseUrl: string }>({
  liveBaseUrl: async ({ page }, use) => {
    void page;
    requireLiveRun();
    await use(resolveLiveBaseUrl().value);
  },
  collector: async ({ page }, use, testInfo) => {
    requireLiveRun();
    const collector = new ArtifactCollector(page, testInfo, resolveLiveBaseUrl());
    await collector.start();
    try {
      await use(collector);
    } finally {
      await collector.finish();
    }
  },
});

export { expect } from '@playwright/test';
