import { expect, liveScenario } from './support/live-scenario';

liveScenario(
  'Rust-owned authored encounter live evidence @live',
  async ({ page, collector, liveBaseUrl }) => {
    collector.addNonClaim(
      'This certifies the D20G1B landing, Engine-backed camp loadout, and bounded encounter path, not later opposition, rewards, alternate content, or navigation.',
    );

    await page.goto(liveBaseUrl);
    if (await page.getByRole('button', { name: 'New Adventure' }).isVisible()) {
      await collector.milestone('empty game ready', { screenshot: true });
      await page.getByRole('button', { name: 'New Adventure' }).click();
      await expect(page.getByRole('heading', { name: "Warden's Gate Camp" })).toBeVisible();
      await expect(page.getByLabel('Armor defense readout')).toContainText('16');
      await collector.milestone('durable adventure camp loadout', {
        screenshot: true,
        layerSnapshot: {
          inventory: await page.getByRole('region', { name: 'Inventory', exact: true }).innerText(),
          equipment: await page.getByLabel('Equipment').innerText(),
        },
      });
      await page.setViewportSize({ width: 390, height: 844 });
      expect(
        await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),
      ).toBe(true);
      await collector.milestone('mobile camp loadout', { screenshot: true });
      await page.setViewportSize({ width: 1280, height: 720 });
    } else if (await page.getByRole('button', { name: 'Continue Adventure' }).isVisible()) {
      await page.getByRole('button', { name: 'Continue Adventure' }).click();
    }
    if (await page.getByRole('button', { name: 'Enter The Iron Warden' }).isVisible()) {
      await page.getByRole('button', { name: 'Enter The Iron Warden' }).click();
    }
    await expect(page.locator('aui-character-status')).toHaveCount(2);
    await page.getByRole('button', { name: 'Longsword Strike' }).click();
    await expect(page.getByLabel('Authoritative action preview')).toContainText(
      'Equipped item 201',
    );
    await collector.milestone('authored action preview with source attribution', {
      screenshot: true,
    });

    await page.getByRole('button', { name: /Parry · 1 Guard/ }).click();
    await expect(page.getByLabel('Authoritative action preview')).toContainText(
      'against defense 17',
    );
    await page.getByRole('button', { name: 'Resolve deterministic roll' }).click();
    await expect(page.getByLabel('Latest outcome explanation')).toContainText(
      'Deterministic roll index',
    );
    await collector.milestone('resolved action receipt with source decisions', {
      screenshot: true,
      layerSnapshot: {
        latest: await page.getByLabel('Latest outcome explanation').innerText(),
      },
    });
    await page.getByRole('button', { name: 'Advance turn' }).click();
    await page.getByRole('button', { name: 'Save', exact: true }).click();
    await expect(page.getByText('Saved', { exact: true })).toBeVisible();
    await collector.milestone('resolved receipt advanced turn and saved state', {
      screenshot: true,
      layerSnapshot: {
        route: page.url(),
        encounter: await page.getByLabel('Encounter identity').innerText(),
        latest: await page.getByLabel('Latest outcome explanation').innerText(),
      },
    });

    await page.setViewportSize({ width: 390, height: 844 });
    await expect(page.getByRole('button', { name: 'Advance turn' })).toBeVisible();
    await collector.milestone('mobile encounter shell', { screenshot: true });
  },
);
