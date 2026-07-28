import { expect, test, type Page } from '@playwright/test';

test.describe.serial('real Rust encounter shell', () => {
  test('empty game starts and resolves authored action, reaction, turn, and save', async ({
    page,
    request,
  }) => {
    const health = await request.get('/healthz');
    expect(health.ok()).toBe(true);
    await expect(health.json()).resolves.toEqual({
      status: 'ok',
      version: '0.1.0',
    });

    await page.goto('/');
    await expect(
      page.getByRole('heading', { level: 1, name: 'Rusty D20', exact: true }),
    ).toBeVisible();
    await expect(page.getByRole('heading', { name: "The Warden's Gate" })).toBeVisible();
    await page.getByRole('button', { name: 'New Adventure' }).click();
    await expect(page.getByRole('heading', { name: "Warden's Gate Camp" })).toBeVisible();
    await expect(page.getByLabel('Armor defense readout')).toContainText('16');
    await expect(page.getByRole('region', { name: 'Inventory', exact: true })).toBeVisible();
    await expect(page.getByLabel('Equipment')).toBeVisible();
    await expect(page.getByLabel('Camp stash')).toContainText('Spare buckler');

    await page.getByRole('button', { name: 'Take' }).click();
    await expect(page.getByRole('alert')).toContainText('capacity rejection');
    await expect(page.getByRole('alert')).toContainText('maximum: 2');
    await page.getByRole('button', { name: 'Dismiss' }).click();
    await expect(page.getByLabel('Armor defense readout')).toContainText('16');
    await expect(page.getByText('Carried 2/2')).toBeVisible();

    const chainInventory = page.getByRole('button', {
      name: /Mara's chain armor · equipped body/,
    });
    await chainInventory.focus();
    await chainInventory.press('Enter');
    await expect(page.getByLabel('Armor defense readout')).toContainText('14');
    const unequippedChain = page.getByRole('button', {
      name: "Mara's chain armor",
    });
    await unequippedChain.focus();
    await unequippedChain.press('Space');
    await expect(page.getByLabel('Armor defense readout')).toContainText('16');

    await page.setViewportSize({ width: 390, height: 844 });
    await expect(page.locator('aui-character-status')).toHaveCount(1);
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),
    ).toBe(true);
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.getByRole('button', { name: 'Enter The Iron Warden' }).click();

    await expect(page.locator('aui-character-status')).toHaveCount(2);
    await expect(page.getByText('Mara Venn', { exact: true })).toBeVisible();
    await expect(
      page.locator('aui-character-status').nth(1).getByText('Iron Warden', { exact: true }),
    ).toBeVisible();
    await expect(page.getByLabel('Encounter identity')).toContainText('Engine fb608e323a8b');

    await page.getByLabel('Target').selectOption({ label: 'Iron Warden' });
    await page.getByRole('button', { name: 'Longsword Strike' }).click();
    const preview = page.getByLabel('Authoritative action preview');
    await expect(preview).toContainText('against defense 15');
    await expect(preview).toContainText('Equipped item 201');

    await page.getByRole('button', { name: /Parry · 1 Guard/ }).click();
    await expect(preview).toContainText('against defense 17');
    await expect(page.getByLabel('Combat log')).toContainText('raised a reaction');

    await page.getByRole('button', { name: 'Resolve deterministic roll' }).click();
    const explanation = page.getByLabel('Latest outcome explanation');
    await expect(explanation).toContainText(/d20 \d+ \+ modifier/);
    await expect(explanation).toContainText('Deterministic roll index 0');
    await expect(explanation).toContainText(/Intrinsic|Equipped item|missed/);

    await page.getByRole('button', { name: 'Begin Iron Warden turn' }).first().click();
    await expect(preview).toContainText('Iron Warden');
    await page.getByRole('button', { name: /Parry · 1 Guard/ }).click();
    await expect(page.getByLabel('Combat log')).toContainText('Mara Venn raised a reaction');
    await page.getByRole('button', { name: 'Resolve deterministic roll' }).click();
    await expect(page.getByLabel('Encounter identity')).toContainText('Turn 1');
    await expect(page.getByLabel('Encounter identity')).toContainText('Mara acting');
    await page.getByRole('button', { name: 'Save', exact: true }).click();
    await expect(page.getByText('Saved', { exact: true })).toBeVisible();
  });

  test('a normal control presents a stale rejection after another client advances state', async ({
    browser,
    page,
  }) => {
    await page.goto('/');
    const second = await browser.newPage();
    await second.goto('/');
    await continueIfNeeded(page);
    await continueIfNeeded(second);

    await second.getByRole('button', { name: 'Precise Shot' }).click();
    await second.getByRole('button', { name: 'Resolve deterministic roll' }).click();
    await expect(second.getByLabel('Encounter identity')).toContainText('Iron Warden acting');

    await page.getByRole('button', { name: 'Longsword Strike' }).click();
    const alert = page.getByRole('alert');
    await expect(alert).toContainText('stale rejection');
    await expect(alert).toContainText('current revision');
    await expect(page.getByRole('button', { name: 'Reload current state' })).toBeVisible();
    await second.close();
  });

  test('mobile game shell remains usable without horizontal overflow', async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto('/');
    await continueIfNeeded(page);
    await expect(page.getByRole('button', { name: 'Begin Iron Warden turn' }).first()).toBeVisible();
    await expect(page.locator('aui-character-status')).toHaveCount(2);
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),
    ).toBe(true);
  });

  test('runtime connection failure is visibly classified and retryable', async ({ page }) => {
    await page.route('**/api/v1/session', (route) => route.abort('connectionrefused'));
    await page.goto('/');

    const alert = page.getByRole('alert');
    await expect(alert).toContainText('network failure');
    await expect(page.getByRole('button', { name: 'Retry connection' })).toBeVisible();
  });

  test('invalid runtime payload fails closed at the protocol border', async ({ page }) => {
    await page.route('**/api/v1/session', (route) =>
      route.fulfill({
        contentType: 'application/json',
        status: 200,
        body: '{"product":"Rusty D20"}',
      }),
    );
    await page.goto('/');

    await expect(page.getByRole('alert')).toContainText('unknown failure');
    await expect(page.getByText('Game snapshot has an unexpected or invalid shape.')).toBeVisible();
  });
});

async function continueIfNeeded(page: Page): Promise<void> {
  const button = page.getByRole('button', { name: 'Continue Adventure' });
  if (await button.isVisible()) {
    await button.click();
  }
}
