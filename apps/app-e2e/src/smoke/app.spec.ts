import { expect, test } from '@playwright/test';

test('real Rust host serves health, shell, and authoritative readout', async ({ page, request }) => {
  const health = await request.get('/healthz');
  expect(health.ok()).toBe(true);
  await expect(health.json()).resolves.toEqual({ status: 'ok', version: '0.1.0' });

  await page.goto('/');
  await expect(page.getByRole('heading', { level: 1, name: 'Rusty D20', exact: true })).toBeVisible();
  const readout = page.getByLabel('Rust runtime readout');
  await expect(readout).toContainText('Runtime ready');
  await expect(readout).toContainText('fb608e323a8b');
  await expect(readout).toContainText('Canonical entities');
  await expect(readout).toContainText('1');
});

test('runtime connection failure is visibly classified and retryable', async ({ page }) => {
  await page.route('**/api/v1/readout', (route) => route.abort('connectionrefused'));
  await page.goto('/');

  const alert = page.getByRole('alert');
  await expect(alert).toContainText('network failure');
  await expect(page.getByRole('button', { name: 'Retry connection' })).toBeVisible();
});

test('invalid runtime payload fails closed at the protocol border', async ({ page }) => {
  await page.route('**/api/v1/readout', (route) =>
    route.fulfill({ contentType: 'application/json', status: 200, body: '{"status":"ready"}' }),
  );
  await page.goto('/');

  await expect(page.getByRole('alert')).toContainText('unknown failure');
  await expect(page.getByText('Runtime readout has an unexpected shape.')).toBeVisible();
});
