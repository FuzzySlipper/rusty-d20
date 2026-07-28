import { expect, liveScenario } from './support/live-scenario';

liveScenario('Rust-owned bootstrap readout live evidence @live', async ({ page, collector, liveBaseUrl }) => {
  collector.addNonClaim('This scenario proves the durable shell reaches the Rust host readout. It does not prove d20 semantics, authored rules, saves, encounter workflows, or broader accessibility coverage.');

  await page.goto(liveBaseUrl);
  await expect(page.getByRole('heading', { level: 1, name: 'Rusty D20', exact: true })).toBeVisible();
  const readout = page.getByLabel('Rust runtime readout');
  await expect(readout).toContainText('Runtime ready');
  await expect(readout).toContainText('fb608e323a8b');
  const health = await page.request.get(`${liveBaseUrl}/healthz`);
  expect(health.ok()).toBe(true);
  await collector.milestone('Rust runtime readout rendered', {
    screenshot: true,
    layerSnapshot: {
      route: page.url(),
      visibleHeading: await page.getByRole('heading').first().innerText(),
      health: await health.json(),
    },
  });
});
