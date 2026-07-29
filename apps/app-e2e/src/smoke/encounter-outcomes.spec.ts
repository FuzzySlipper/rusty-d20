import { spawn, type ChildProcess } from 'node:child_process';
import { mkdtemp, rm } from 'node:fs/promises';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { expect, test, type APIRequestContext, type Page } from '@playwright/test';
import { workspaceRoot } from '@nx/devkit';

test.describe.serial('complete deterministic encounter outcomes', () => {
  test('condition-forbidden opposition actions are filtered without a browser deadlock', async ({
    page,
  }, testInfo) => {
    test.setTimeout(120_000);
    const host = await startIsolatedHost('legal-opposition');
    const browserErrors: string[] = [];
    page.on('pageerror', (error) => browserErrors.push(error.message));
    page.on('console', (message) => {
      if (message.type() === 'error') {
        browserErrors.push(message.text());
      }
    });
    try {
      await page.goto(host.baseUrl);
      await startAdventure(page, "The Warden's Gate");
      await enterWardenEncounter(page);

      for (let round = 0; round < 8; round += 1) {
        await page.getByRole('button', { name: 'Disrupt', exact: true }).click();
        await page.getByRole('button', { name: 'Resolve deterministic roll' }).click();
        await expect(
          page.getByRole('button', { name: 'Begin Iron Warden turn', exact: true }).first(),
        ).toBeVisible();

        const warden = page.locator('aui-character-status').nth(1);
        const unsettled = warden.getByLabel('Active buffs').getByText(/^Unsettled/);
        const isUnsettled = (await unsettled.count()) > 0 && (await unsettled.isVisible());

        await page
          .getByRole('button', { name: 'Begin Iron Warden turn', exact: true })
          .first()
          .click();
        const preview = page.getByLabel('Authoritative action preview');
        await expect(preview).toContainText('Iron Warden');
        if (isUnsettled) {
          await expect(preview).toContainText(/Longsword Strike|Precise Shot/);
          await expect(preview).not.toContainText(/Pin In Place|Disrupt/);
          await testInfo.attach('legal-opposition-after-unsettled.png', {
            body: await page.screenshot({ fullPage: true }),
            contentType: 'image/png',
          });
          await page.getByRole('button', { name: 'Resolve deterministic roll' }).click();
          await expect(page.getByLabel('Encounter identity')).toContainText('Mara Venn acting');
          expect(browserErrors).toEqual([]);
          return;
        }

        await page.getByRole('button', { name: 'Resolve deterministic roll' }).click();
        await expect(page.getByRole('button', { name: 'Disrupt', exact: true })).toBeVisible();
      }

      throw new Error('The deterministic browser sequence never applied Unsettled.');
    } finally {
      await host.stop();
    }
  });

  test('victory reward and the next authored encounter survive complete campaign reopen', async ({
    page,
    request,
  }, testInfo) => {
    test.setTimeout(180_000);
    const host = await startIsolatedHost('victory');
    const browserErrors: string[] = [];
    page.on('pageerror', (error) => browserErrors.push(error.message));
    page.on('console', (message) => {
      if (message.type() === 'error') {
        browserErrors.push(message.text());
      }
    });
    try {
      await page.goto(host.baseUrl);
      await startAdventure(page, "The Warden's Gate");
      await enterWardenEncounter(page);
      await playToOutcome(page, 'Precise Shot', 'Iron Warden', false, true);

      await expect(page.getByLabel('Encounter victory')).toContainText('The Iron Warden defeated');
      await expect(page.getByLabel('Encounter victory')).toContainText('Warden chain armor');
      await testInfo.attach('warden-victory.png', {
        body: await page.screenshot({ fullPage: true }),
        contentType: 'image/png',
      });
      await page.getByRole('button', { name: 'Save', exact: true }).click();
      const outcome = await sessionSnapshot(request, host.baseUrl);
      expect(outcome.campaign.phase).toBe('outcome');
      expect(outcome.campaign.latestOutcome).toMatchObject({
        kind: 'victory',
        rewardItemId: 201,
      });
      expect(outcome.campaign.loadout.stashItems).toContainEqual(
        expect.objectContaining({ entityId: 201, name: 'Warden chain armor' }),
      );
      expect(outcome.encounter?.turnOwner).toBeNull();

      await host.restart();
      await page.goto(host.baseUrl);
      await page.getByRole('button', { name: 'Continue Adventure' }).click();
      await expect(page.getByLabel('Encounter victory')).toBeVisible();
      await page.getByRole('button', { name: 'Continue adventure' }).click();
      await expect(
        page
          .getByRole('region', { name: 'Dungeon exploration' })
          .getByRole('heading', { name: "Warden's Gate Pass" })
          .first(),
      ).toBeVisible();
      await enterWardenReckoning(page);
      await expect(page.getByLabel('Latest outcome explanation')).toContainText(
        'bounded vitality track service',
      );
      await playToOutcome(page, 'Precise Shot', 'Iron Warden', false, true);
      await expect(page.getByLabel('Encounter defeat')).toContainText(
        'Mara fell at the reckoning',
      );
      await expect(page.getByLabel('Encounter defeat')).toContainText(
        'without granting a reward',
      );
      await page.getByRole('button', { name: 'Save', exact: true }).click();

      await host.restart();
      await page.goto(host.baseUrl);
      await page.getByRole('button', { name: 'Continue Adventure' }).click();
      await expect(page.getByLabel('Encounter defeat')).toContainText(
        'Mara fell at the reckoning',
      );
      await page.getByRole('button', { name: "Return to The Warden's Gate Camp" }).click();
      await expect(page.getByLabel('Completed encounters')).toContainText(
        "The Warden's Reckoning",
      );
      await expect(page.getByRole('button', { name: /^Enter / })).toHaveCount(0);
      await page.getByRole('button', { name: 'Save', exact: true }).click();

      await host.restart();
      const reopened = await sessionSnapshot(request, host.baseUrl);
      expect(reopened.campaign.phase).toBe('camp');
      expect(reopened.campaign.completedEncounters).toEqual([
        expect.objectContaining({ encounterId: 'iron-warden', outcome: 'victory' }),
        expect.objectContaining({ encounterId: 'wardens-reckoning', outcome: 'defeat' }),
      ]);
      expect(reopened.campaign.hero.healthCurrent).toBe(12);
      expect(
        reopened.campaign.loadout.stashItems.filter((item) => item.entityId === 201),
      ).toHaveLength(1);
      expect(reopened.encounter).toBeNull();
      expect(browserErrors).toEqual([]);
    } finally {
      await host.stop();
    }
  });

  test('defeat grants no reward and applies bounded camp recovery on mobile', async ({
    page,
    request,
  }, testInfo) => {
    test.setTimeout(180_000);
    const host = await startIsolatedHost('defeat');
    try {
      await page.setViewportSize({ width: 390, height: 844 });
      await page.goto(host.baseUrl);
      await startAdventure(page, "The Warden's Gate");
      await page
        .getByLabel('Equipment')
        .getByRole('button', { name: /Body: Mara's chain armor/ })
        .click();
      await page
        .getByLabel('Equipment')
        .getByRole('button', { name: /Off Hand: Mara's buckler/ })
        .click();
      await expect(page.getByLabel('Armor defense readout')).toContainText('12');
      await enterWardenEncounter(page);
      await playToOutcome(page, 'Longsword Strike', 'Iron Warden', true, false);

      await expect(page.getByLabel('Encounter defeat')).toContainText('Mara was defeated');
      await testInfo.attach('mobile-defeat.png', {
        body: await page.screenshot({ fullPage: true }),
        contentType: 'image/png',
      });
      expect(
        await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),
      ).toBe(true);
      await page.getByRole('button', { name: 'Save', exact: true }).click();
      const outcome = await sessionSnapshot(request, host.baseUrl);
      expect(outcome.campaign.latestOutcome).toMatchObject({
        kind: 'defeat',
        rewardItemId: null,
      });
      expect(outcome.campaign.loadout.stashItems.some((item) => item.entityId === 201)).toBe(false);
      expect(outcome.campaign.hero.healthCurrent).toBe(0);

      await host.restart();
      await page.goto(host.baseUrl);
      await page.getByRole('button', { name: 'Continue Adventure' }).click();
      await expect(page.getByLabel('Encounter defeat')).toBeVisible();
      await page.getByRole('button', { name: "Return to The Warden's Gate Camp" }).click();
      await page.getByRole('button', { name: 'Save', exact: true }).click();
      const recovered = await sessionSnapshot(request, host.baseUrl);
      expect(recovered.campaign.phase).toBe('camp');
      expect(recovered.campaign.hero.healthCurrent).toBe(12);
      expect(recovered.campaign.latestOutcome?.kind).toBe('defeat');

      await host.restart();
      expect((await sessionSnapshot(request, host.baseUrl)).campaign.hero.healthCurrent).toBe(12);
      await page.goto(host.baseUrl);
      await page.getByRole('button', { name: 'Continue Adventure' }).click();
      await page.getByRole('button', { name: 'Enter the dungeon' }).click();
      await stepForward(page, 8);

      const pastCompletedTrigger = await sessionSnapshot(request, host.baseUrl);
      expect(pastCompletedTrigger.campaign.phase).toBe('exploration');
      expect(pastCompletedTrigger.campaign.activeEncounterId).toBeNull();
      expect(pastCompletedTrigger.exploration).toEqual(
        expect.objectContaining({ x: 9, y: 1 }),
      );

      await enterWardenReckoning(page);
      await expect(page.getByLabel('Encounter identity')).toBeVisible();
      await expect(page.getByRole('button', { name: 'Precise Shot' })).toBeVisible();
      const continued = await sessionSnapshot(request, host.baseUrl);
      expect(continued.campaign.phase).toBe('encounter');
      expect(continued.campaign.activeEncounterId).toBe('wardens-reckoning');
    } finally {
      await host.stop();
    }
  });

  test('Ember path exposes distinct authored mechanics and survives a complete fresh-process victory', async ({
    page,
    request,
  }) => {
    test.setTimeout(180_000);
    const host = await startIsolatedHost('ember-victory');
    try {
      await page.setViewportSize({ width: 390, height: 844 });
      await page.goto(host.baseUrl);
      await startAdventure(page, "Ember's Wake");
      await expect(page.getByRole('heading', { name: "Ember's Wake Camp" })).toBeVisible();
      await expect(page.getByText('Sera Vale', { exact: true })).toBeVisible();
      await expect(page.getByLabel('Nerve defense readout')).toContainText('16');
      await expect(page.getByLabel('Nerve defense readout')).toContainText('Equipped item 212');
      await expect(page.getByLabel('Nerve defense readout')).toContainText('Equipped item 213');
      await expect(page.getByLabel('Nerve defense readout')).toContainText('suppressed');
      await expect(page.getByLabel('Camp stash')).toContainText('Spare runed robe');
      expect(
        await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),
      ).toBe(true);
      await page.getByRole('button', { name: 'Save', exact: true }).click();

      const camp = await sessionSnapshot(request, host.baseUrl);
      expect(camp.campaign).toMatchObject({
        id: 'embers-wake',
        title: "Ember's Wake",
        phase: 'camp',
      });
      const emberFingerprint = camp.rulesetFingerprint;

      await host.restart();
      expect(await sessionSnapshot(request, host.baseUrl)).toEqual(camp);
      await page.goto(host.baseUrl);
      await page.getByRole('button', { name: 'Continue Adventure' }).click();
      await expect(page.getByRole('heading', { name: "Ember's Wake Camp" })).toBeVisible();
      await enterAshSeerEncounter(page);
      await expect(
        page.locator('aui-character-status').nth(1).getByText('Ash Seer', { exact: true }),
      ).toBeVisible();
      await expect(page.getByText('Focus 3/3', { exact: true })).toHaveCount(2);
      await expect(page.getByRole('button', { name: 'Fire Bolt', exact: true })).toBeVisible();
      await expect(page.getByRole('button', { name: 'Mind Spike', exact: true })).toBeVisible();

      await playToOutcome(page, 'Fire Bolt', 'Ash Seer', false, true, /Fire Bolt|fire|Scorched/i);
      await expect(page.getByLabel('Encounter victory')).toContainText('The Ash Seer defeated');
      await expect(page.getByLabel('Encounter victory')).toContainText("Ash Seer's mindward charm");
      await page.getByRole('button', { name: 'Save', exact: true }).click();
      const outcome = await sessionSnapshot(request, host.baseUrl);
      expect(outcome.rulesetFingerprint).toBe(emberFingerprint);
      expect(outcome.campaign.latestOutcome).toMatchObject({
        kind: 'victory',
        rewardItemId: 211,
      });

      await host.restart();
      expect(await sessionSnapshot(request, host.baseUrl)).toEqual(outcome);
      await page.goto(host.baseUrl);
      await page.getByRole('button', { name: 'Continue Adventure' }).click();
      await expect(page.getByLabel('Encounter victory')).toBeVisible();
      await page.getByRole('button', { name: 'Continue adventure' }).click();
      await expect(page.getByLabel('Camp stash')).toContainText("Ash Seer's mindward charm");
      await page.getByRole('button', { name: 'Save', exact: true }).click();

      await host.restart();
      const reopened = await sessionSnapshot(request, host.baseUrl);
      expect(reopened.campaign.id).toBe('embers-wake');
      expect(reopened.rulesetFingerprint).toBe(emberFingerprint);
      expect(
        reopened.campaign.loadout.stashItems.filter((item) => item.entityId === 211),
      ).toHaveLength(1);
    } finally {
      await host.stop();
    }
  });
});

async function playToOutcome(
  page: Page,
  playerAction: string,
  oppositionName: string,
  oppositionReacts: boolean,
  playerReacts: boolean,
  expectedPlayerReceipt?: RegExp,
): Promise<void> {
  for (let round = 0; round < 64; round += 1) {
    await page.getByRole('button', { name: playerAction, exact: true }).click();
    if (oppositionReacts) {
      await applyFirstReactionIfAvailable(page);
    }
    await page.getByRole('button', { name: 'Resolve deterministic roll' }).click();
    if (round === 0 && expectedPlayerReceipt !== undefined) {
      await expect(page.getByLabel('Latest outcome explanation')).toContainText(
        expectedPlayerReceipt,
      );
    }
    if (await waitForOutcomeOr(page, `Begin ${oppositionName} turn`)) {
      return;
    }

    await page
      .getByRole('button', {
        name: `Begin ${oppositionName} turn`,
        exact: true,
      })
      .first()
      .click();
    if (playerReacts) {
      await applyFirstReactionIfAvailable(page);
    }
    await page.getByRole('button', { name: 'Resolve deterministic roll' }).click();
    if (await waitForOutcomeOr(page, playerAction)) {
      return;
    }
  }
  throw new Error('Deterministic encounter did not reach an outcome within 64 rounds.');
}

async function applyFirstReactionIfAvailable(page: Page): Promise<void> {
  const reaction = page.getByRole('button', { name: /· 1 (Guard|Focus) ·/ });
  if ((await reaction.count()) > 0 && (await reaction.first().isVisible())) {
    await reaction.first().click();
  }
}

async function waitForOutcomeOr(page: Page, nextControl: string): Promise<boolean> {
  const outcome = page.getByRole('button', {
    name: /^(Return to .+ Camp|Continue adventure)$/,
  });
  const next = page.getByRole('button', { name: nextControl, exact: true });
  await expect
    .poll(async () => (await outcome.isVisible()) || (await next.first().isVisible()), {
      timeout: 10_000,
    })
    .toBe(true);
  return outcome.isVisible();
}

async function startAdventure(page: Page, title: string): Promise<void> {
  await page
    .getByRole('button', {
      name: `New Adventure · ${title}`,
      exact: true,
    })
    .click();
}

async function enterWardenEncounter(page: Page): Promise<void> {
  await page.getByRole('button', { name: 'Enter the dungeon' }).click();
  await stepForward(page, 8);
}

async function enterWardenReckoning(page: Page): Promise<void> {
  await page.getByRole('button', { name: 'Right ↷' }).click();
  await stepForward(page, 4);
  await page.getByRole('button', { name: 'Right ↷' }).click();
  await stepForward(page, 8);
}

async function enterAshSeerEncounter(page: Page): Promise<void> {
  await page.getByRole('button', { name: 'Enter the dungeon' }).click();
  await stepForward(page, 6);
  await page.getByRole('button', { name: 'Right ↷' }).click();
  await stepForward(page, 4);
}

async function stepForward(page: Page, count: number): Promise<void> {
  for (let step = 0; step < count; step += 1) {
    await page.getByRole('button', { name: '↑ Forward' }).click();
  }
}

interface SessionSnapshot {
  rulesetFingerprint: string;
  campaign: {
    id: string;
    title: string;
    phase: 'camp' | 'exploration' | 'encounter' | 'outcome';
    activeEncounterId: string | null;
    hero: { healthCurrent: number };
    latestOutcome: {
      kind: 'victory' | 'defeat';
      rewardItemId: number | null;
    } | null;
    loadout: {
      stashItems: Array<{ entityId: number; name: string }>;
    };
    completedEncounters: Array<{
      encounterId: string;
      outcome: 'victory' | 'defeat';
    }>;
  };
  exploration: { x: number; y: number } | null;
  encounter: { turnOwner: 'player' | 'opposition' | null } | null;
}

async function sessionSnapshot(
  request: APIRequestContext,
  baseUrl: string,
): Promise<SessionSnapshot> {
  const response = await request.get(`${baseUrl}/api/v1/session`);
  expect(response.ok()).toBe(true);
  return response.json() as Promise<SessionSnapshot>;
}

interface IsolatedHost {
  readonly baseUrl: string;
  restart(): Promise<void>;
  stop(): Promise<void>;
}

async function startIsolatedHost(label: string): Promise<IsolatedHost> {
  const directory = await mkdtemp(join(tmpdir(), `rusty-d20-${label}-`));
  const savePath = join(directory, 'save.json');
  const port = await freePort();
  const baseUrl = `http://127.0.0.1:${port}`;
  let process = startHost(port, savePath);
  await waitForHealth(baseUrl, process);
  return {
    baseUrl,
    async restart() {
      await stopHost(process);
      process = startHost(port, savePath);
      await waitForHealth(baseUrl, process);
    },
    async stop() {
      await stopHost(process);
      await rm(directory, { force: true, recursive: true });
    },
  };
}

function startHost(port: number, savePath: string): ChildProcess {
  return spawn(
    'cargo',
    [
      'run',
      '-p',
      'rusty-d20',
      '--bin',
      'rusty-d20-host',
      '--',
      '--address',
      `127.0.0.1:${port}`,
      '--save-file',
      savePath,
    ],
    { cwd: workspaceRoot, stdio: ['ignore', 'pipe', 'pipe'] },
  );
}

async function stopHost(host: ChildProcess): Promise<void> {
  if (host.exitCode !== null) {
    return;
  }
  const exited = new Promise<void>((resolve) => host.once('exit', () => resolve()));
  host.kill('SIGINT');
  await exited;
}

async function waitForHealth(baseUrl: string, host: ChildProcess): Promise<void> {
  const started = Date.now();
  while (Date.now() - started < 90_000) {
    if (host.exitCode !== null) {
      throw new Error(`Rust host exited before becoming ready with code ${host.exitCode}.`);
    }
    try {
      const response = await fetch(`${baseUrl}/healthz`);
      if (response.ok) {
        return;
      }
    } catch {
      // The Rust host is still compiling or binding.
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error('Rust host did not become ready.');
}

function freePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      server.close(() => {
        if (address !== null && typeof address === 'object') {
          resolve(address.port);
        } else {
          reject(new Error('Could not allocate a local port.'));
        }
      });
    });
    server.on('error', reject);
  });
}
