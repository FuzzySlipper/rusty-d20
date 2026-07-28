import { spawn, type ChildProcess } from 'node:child_process';
import { mkdtemp, rm } from 'node:fs/promises';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { expect, test, type APIRequestContext, type Page } from '@playwright/test';
import { workspaceRoot } from '@nx/devkit';

test.describe.serial('complete deterministic encounter outcomes', () => {
  test('victory grants one canonical reward and survives outcome and camp reopen', async ({
    page,
    request,
  }) => {
    test.setTimeout(180_000);
    const host = await startIsolatedHost('victory');
    try {
      await page.goto(host.baseUrl);
      await page.getByRole('button', { name: 'New Adventure' }).click();
      await page.getByRole('button', { name: 'Enter The Iron Warden' }).click();
      await playToOutcome(page, 'Precise Shot', false, true);

      await expect(page.getByLabel('Encounter victory')).toContainText(
        'The Iron Warden defeated',
      );
      await expect(page.getByLabel('Encounter victory')).toContainText(
        'Warden chain armor',
      );
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
      await page.getByRole('button', { name: "Return to Warden's Gate Camp" }).click();
      await expect(page.getByRole('heading', { name: "Warden's Gate Camp" })).toBeVisible();
      await expect(page.getByLabel('Latest encounter victory')).toContainText(
        'Warden chain armor',
      );
      await expect(page.getByLabel('Camp stash')).toContainText('Warden chain armor');
      await page.getByRole('button', { name: 'Save', exact: true }).click();

      await host.restart();
      const reopened = await sessionSnapshot(request, host.baseUrl);
      expect(reopened.campaign.phase).toBe('camp');
      expect(
        reopened.campaign.loadout.stashItems.filter((item) => item.entityId === 201),
      ).toHaveLength(1);
      expect(reopened.encounter).toBeNull();
    } finally {
      await host.stop();
    }
  });

  test('defeat grants no reward and applies bounded camp recovery on mobile', async ({
    page,
    request,
  }) => {
    test.setTimeout(180_000);
    const host = await startIsolatedHost('defeat');
    try {
      await page.setViewportSize({ width: 390, height: 844 });
      await page.goto(host.baseUrl);
      await page.getByRole('button', { name: 'New Adventure' }).click();
      await page
        .getByLabel('Equipment')
        .getByRole('button', { name: /Body: Mara's chain armor/ })
        .click();
      await page
        .getByLabel('Equipment')
        .getByRole('button', { name: /Off Hand: Mara's buckler/ })
        .click();
      await expect(page.getByLabel('Armor defense readout')).toContainText('12');
      await page.getByRole('button', { name: 'Enter The Iron Warden' }).click();
      await playToOutcome(page, 'Longsword Strike', true, false);

      await expect(page.getByLabel('Encounter defeat')).toContainText('Mara was defeated');
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
      await page.getByRole('button', { name: "Return to Warden's Gate Camp" }).click();
      await page.getByRole('button', { name: 'Save', exact: true }).click();
      const recovered = await sessionSnapshot(request, host.baseUrl);
      expect(recovered.campaign.phase).toBe('camp');
      expect(recovered.campaign.hero.healthCurrent).toBe(12);
      expect(recovered.campaign.latestOutcome?.kind).toBe('defeat');

      await host.restart();
      expect((await sessionSnapshot(request, host.baseUrl)).campaign.hero.healthCurrent).toBe(12);
    } finally {
      await host.stop();
    }
  });
});

async function playToOutcome(
  page: Page,
  playerAction: string,
  oppositionReacts: boolean,
  playerReacts: boolean,
): Promise<void> {
  for (let round = 0; round < 64; round += 1) {
    await page.getByRole('button', { name: playerAction, exact: true }).click();
    if (oppositionReacts) {
      await applyFirstReactionIfAvailable(page);
    }
    await page.getByRole('button', { name: 'Resolve deterministic roll' }).click();
    if (await waitForOutcomeOr(page, 'Begin Iron Warden turn')) {
      return;
    }

    await page.getByRole('button', { name: 'Begin Iron Warden turn' }).first().click();
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
  const reaction = page.getByRole('button', { name: /Parry · 1 Guard/ });
  if ((await reaction.count()) > 0 && (await reaction.first().isVisible())) {
    await reaction.first().click();
  }
}

async function waitForOutcomeOr(page: Page, nextControl: string): Promise<boolean> {
  const outcome = page.getByRole('button', { name: "Return to Warden's Gate Camp" });
  const next = page.getByRole('button', { name: nextControl, exact: true });
  await expect
    .poll(async () => (await outcome.isVisible()) || (await next.first().isVisible()), {
      timeout: 10_000,
    })
    .toBe(true);
  return outcome.isVisible();
}

interface SessionSnapshot {
  campaign: {
    phase: 'camp' | 'encounter' | 'outcome';
    hero: { healthCurrent: number };
    latestOutcome: { kind: 'victory' | 'defeat'; rewardItemId: number | null } | null;
    loadout: {
      stashItems: Array<{ entityId: number; name: string }>;
    };
  };
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
