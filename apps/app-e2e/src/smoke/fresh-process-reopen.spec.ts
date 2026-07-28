import { spawn, type ChildProcess } from 'node:child_process';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { expect, test, type APIRequestContext } from '@playwright/test';
import { workspaceRoot } from '@nx/devkit';

test('pending saves reject atomically and completed state survives a fresh Rust host', async ({
  page,
  request,
}) => {
  test.setTimeout(180_000);
  const directory = await mkdtemp(join(tmpdir(), 'rusty-d20-reopen-'));
  const savePath = join(directory, 'save.json');
  const port = await freePort();
  const baseUrl = `http://127.0.0.1:${port}`;
  let host: ChildProcess | undefined;

  try {
    host = startHost(port, savePath);
    await waitForHealth(baseUrl, host);
    await page.goto(baseUrl);
    await page.getByRole('button', { name: 'New Adventure' }).click();
    await page
      .getByLabel('Equipment')
      .getByRole('button', { name: "Off Hand: Mara's buckler" })
      .click();
    await page
      .getByLabel('Inventory item actions')
      .getByRole('listitem')
      .filter({ hasText: "Mara's buckler" })
      .getByRole('button', { name: 'Store' })
      .click();
    await page
      .getByLabel('Camp stash')
      .getByRole('listitem')
      .filter({ hasText: 'Spare buckler' })
      .getByRole('button', { name: 'Take' })
      .click();
    await page.getByRole('button', { name: 'Spare buckler' }).click();
    await expect(
      page.getByLabel('Equipment').getByRole('button', { name: 'Off Hand: Spare buckler' }),
    ).toBeVisible();
    await expect(page.getByLabel('Armor defense readout')).toContainText('16');
    await page.getByRole('button', { name: 'Save', exact: true }).click();
    await expect(page.getByText('Saved', { exact: true })).toBeVisible();
    const baseline = await sessionSnapshot(request, baseUrl);
    expect(
      baseline.campaign.loadout.equipmentSlots.find((slot) => slot.id === 'off-hand')?.equipped
        ?.entityId,
    ).toBe(204);
    const baselineFile = await readFile(savePath);

    await page.getByRole('button', { name: 'Enter The Iron Warden' }).click();
    await page.getByRole('button', { name: 'Precise Shot' }).click();
    await expect(page.getByRole('button', { name: 'Save', exact: true })).toBeDisabled();
    await expect(page.getByText('Resolve the pending action before saving.')).toBeVisible();
    const previewOnly = await sessionSnapshot(request, baseUrl);
    expect(previewOnly.encounter.pendingAction).not.toBeNull();
    await expectPendingSaveRejection(request, baseUrl, previewOnly.revision);
    expect(await sessionSnapshot(request, baseUrl)).toEqual(previewOnly);
    expect(await readFile(savePath)).toEqual(baselineFile);
    await stopHost(host);
    host = undefined;

    host = startHost(port, savePath);
    await waitForHealth(baseUrl, host);
    expect(await sessionSnapshot(request, baseUrl)).toEqual(baseline);
    await page.goto(baseUrl);
    await page.getByRole('button', { name: 'Continue Adventure' }).click();
    await expect(
      page.getByLabel('Equipment').getByRole('button', { name: 'Off Hand: Spare buckler' }),
    ).toBeVisible();
    await page.getByRole('button', { name: 'Enter The Iron Warden' }).click();
    await expect(page.getByLabel('Encounter identity')).toContainText('Armor defense 16');
    await page.getByRole('button', { name: 'Longsword Strike' }).click();
    await page.getByRole('button', { name: /Parry · 1 Guard/ }).click();
    await expect(page.getByRole('button', { name: 'Save', exact: true })).toBeDisabled();
    const reacted = await sessionSnapshot(request, baseUrl);
    expect(reacted.encounter.pendingAction).not.toBeNull();
    const reactedOpponent = reacted.encounter.characters.find(
      (character: GameCharacter) => character.id !== reacted.encounter.playerId,
    );
    expect(reactedOpponent?.resources).toContainEqual({
      current: 1,
      id: 'guard',
      label: 'Guard',
      maximum: 2,
    });
    expect(reactedOpponent?.effects.some((effect) => effect.startsWith('Parry Stance'))).toBe(true);
    await expectPendingSaveRejection(request, baseUrl, reacted.revision);
    expect(await sessionSnapshot(request, baseUrl)).toEqual(reacted);
    expect(await readFile(savePath)).toEqual(baselineFile);
    await stopHost(host);
    host = undefined;

    host = startHost(port, savePath);
    await waitForHealth(baseUrl, host);
    expect(await sessionSnapshot(request, baseUrl)).toEqual(baseline);
    await page.goto(baseUrl);
    await page.getByRole('button', { name: 'Continue Adventure' }).click();
    await page.getByRole('button', { name: 'Enter The Iron Warden' }).click();
    await page.getByRole('button', { name: 'Precise Shot' }).click();
    await page.getByRole('button', { name: 'Resolve deterministic roll' }).click();
    await page.getByRole('button', { name: 'Save', exact: true }).click();
    await expect(page.getByText('Saved', { exact: true })).toBeVisible();
    const before = (await page.getByLabel('Encounter identity').innerText())
      .split('\n')
      .map((line) => line.trim())
      .filter((line) => line.length > 0);
    await stopHost(host);
    host = undefined;

    host = startHost(port, savePath);
    await waitForHealth(baseUrl, host);
    await page.goto(baseUrl);
    await page.getByRole('button', { name: 'Continue Adventure' }).click();
    await expect(page.getByText('Saved', { exact: true })).toBeVisible();
    for (const line of before) {
      await expect(page.getByLabel('Encounter identity')).toContainText(line);
    }
    await expect(page.getByLabel('Latest outcome explanation')).toContainText(
      'Deterministic roll index 0',
    );

    await page.getByRole('button', { name: 'Precise Shot' }).click();
    await page.getByRole('button', { name: 'Resolve deterministic roll' }).click();
    await expect(page.getByLabel('Latest outcome explanation')).toContainText(
      'Deterministic roll index 1',
    );
  } finally {
    if (host !== undefined) {
      await stopHost(host);
    }
    await rm(directory, { force: true, recursive: true });
  }
});

interface GameCharacter {
  id: number;
  resources: Array<{
    current: number;
    id: string;
    label: string;
    maximum: number;
  }>;
  effects: string[];
}

interface GameSnapshot {
  revision: number;
  campaign: {
    loadout: {
      equipmentSlots: Array<{
        id: string;
        equipped: { entityId: number } | null;
      }>;
    };
  };
  encounter: {
    playerId: number;
    characters: GameCharacter[];
    pendingAction: unknown | null;
  };
}

async function sessionSnapshot(request: APIRequestContext, baseUrl: string): Promise<GameSnapshot> {
  const response = await request.get(`${baseUrl}/api/v1/session`);
  expect(response.ok()).toBe(true);
  return response.json() as Promise<GameSnapshot>;
}

async function expectPendingSaveRejection(
  request: APIRequestContext,
  baseUrl: string,
  revision: number,
): Promise<void> {
  const response = await request.post(`${baseUrl}/api/v1/session/save`, {
    data: { expectedRevision: revision },
  });
  expect(response.status()).toBe(422);
  await expect(response.json()).resolves.toEqual({
    kind: 'invalid',
    message: 'resolve the pending action before saving',
    retryable: false,
  });
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
      // Host is still compiling or binding.
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
