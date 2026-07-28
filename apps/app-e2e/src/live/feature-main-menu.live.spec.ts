import { expect, liveScenario } from './support/live-scenario';

liveScenario('runtime failure presentation live evidence @live', async ({ page, collector, liveBaseUrl }) => {
  collector.addNonClaim('This scenario proves the production shell presents a classified transport failure. It does not simulate a Rust process crash or certify retry timing.');
  await page.route('**/api/v1/session', (route) => route.abort('connectionrefused'));

  await page.goto(liveBaseUrl);
  await expect(page.getByRole('alert')).toContainText('network failure');
  await expect(page.getByRole('button', { name: 'Retry connection' })).toBeVisible();
  await collector.milestone('classified network failure rendered', {
    screenshot: true,
    layerSnapshot: { route: page.url(), failureKind: 'network' },
  });
});
