import { spawn, type ChildProcess } from 'node:child_process';
import { access, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { expect, test } from '@playwright/test';
import { workspaceRoot } from '@nx/devkit';

test('visible save identity guards reset and malformed persistence has a usable recovery path', async ({
  page,
  request,
}, testInfo) => {
  test.setTimeout(180_000);
  const directory = await mkdtemp(join(tmpdir(), 'rusty-d20-reset-'));
  const savePath = join(directory, 'campaign.json');
  const port = await freePort();
  const baseUrl = `http://127.0.0.1:${port}`;
  const browserErrors: string[] = [];
  page.on('pageerror', (error) => browserErrors.push(error.message));
  page.on('console', (message) => {
    if (message.type() === 'error') {
      browserErrors.push(message.text());
    }
  });
  let host: ChildProcess | undefined;

  try {
    host = startHost(port, savePath);
    await waitForHealth(baseUrl, host);
    await page.goto(baseUrl);
    await expect(
      page.getByLabel('New adventure').getByText(savePath, { exact: true }),
    ).toBeVisible();
    await testInfo.attach('empty-save-identity.png', {
      body: await page.screenshot({ fullPage: true }),
      contentType: 'image/png',
    });
    await page
      .getByRole('button', {
        name: "New Adventure · The Warden's Gate",
        exact: true,
      })
      .click();
    await page.getByRole('button', { name: 'Save', exact: true }).click();

    const resetButton = page.getByRole('button', { name: 'Reset / New Adventure' });
    await resetButton.focus();
    await page.keyboard.press('Enter');
    const dialog = page.getByRole('alertdialog', { name: 'Discard this adventure?' });
    await expect(dialog).toContainText(savePath);
    await expect(dialog).toContainText("The Warden's Gate at revision");
    await expect(dialog).toBeVisible();
    await expect.poll(() => dialog.evaluate((element) => element.matches(':modal'))).toBe(true);
    await expect(dialog.getByRole('button', { name: 'Cancel' })).toBeFocused();
    await page.keyboard.press('Shift+Tab');
    await expect(
      dialog.getByRole('button', { name: 'Discard save and start over' }),
    ).toBeFocused();
    await page.keyboard.press('Tab');
    await expect(dialog.getByRole('button', { name: 'Cancel' })).toBeFocused();
    await page.keyboard.press('Escape');
    await expect(dialog).toBeHidden();
    await expect(resetButton).toBeFocused();
    await expect(page.getByRole('heading', { name: "The Warden's Gate Camp" })).toBeVisible();

    await page.keyboard.press('Enter');
    await expect(dialog.getByRole('button', { name: 'Cancel' })).toBeFocused();
    await page.keyboard.press('Tab');
    await page.keyboard.press('Enter');
    const newAdventureHeading = page.getByRole('heading', { name: 'Choose an adventure' });
    await expect(newAdventureHeading).toBeVisible();
    await expect(newAdventureHeading).toBeFocused();
    await expect(access(savePath)).rejects.toThrow();

    await page
      .getByRole('button', {
        name: "New Adventure · Ember's Wake",
        exact: true,
      })
      .click();
    await page.getByRole('button', { name: 'Save', exact: true }).click();
    await stopHost(host);
    host = undefined;
    host = startHost(port, savePath);
    await waitForHealth(baseUrl, host);
    await page.goto(baseUrl);
    await expect(page.getByRole('heading', { name: "Continue Ember's Wake" })).toBeVisible();
    await expect(
      page.getByLabel('Continue adventure').getByText(savePath, { exact: true }),
    ).toBeVisible();
    await expect(page.getByText(/Adventure embers-wake · revision/)).toBeVisible();

    await stopHost(host);
    host = undefined;
    await writeFile(savePath, '{"schemaVersion":6,"truncated":');
    const malformed = await readFile(savePath);
    host = startHost(port, savePath);
    await waitForHealth(baseUrl, host);
    await page.goto(baseUrl);
    const recovery = page.getByRole('alert');
    await expect(recovery).toContainText('persistence failure');
    await expect(recovery).toContainText('Recovery required');
    await expect(recovery).toContainText(savePath);
    await testInfo.attach('malformed-save-recovery.png', {
      body: await page.screenshot({ fullPage: true }),
      contentType: 'image/png',
    });
    expect(await readFile(savePath)).toEqual(malformed);

    const recoveryResetButton = page.getByRole('button', { name: 'Discard unreadable save' });
    await recoveryResetButton.focus();
    await page.keyboard.press('Enter');
    const recoveryDialog = page.getByRole('alertdialog', { name: 'Discard this adventure?' });
    await expect(recoveryDialog).toContainText('unreadable persisted session');
    await expect
      .poll(() => recoveryDialog.evaluate((element) => element.matches(':modal')))
      .toBe(true);
    await expect(recoveryDialog.getByRole('button', { name: 'Cancel' })).toBeFocused();
    await page.keyboard.press('Shift+Tab');
    await expect(
      recoveryDialog.getByRole('button', { name: 'Discard save and start over' }),
    ).toBeFocused();
    await page.keyboard.press('Tab');
    await expect(recoveryDialog.getByRole('button', { name: 'Cancel' })).toBeFocused();
    await page.keyboard.press('Escape');
    await expect(recoveryDialog).toBeHidden();
    await expect(recoveryResetButton).toBeFocused();

    await page.keyboard.press('Enter');
    await expect(recoveryDialog.getByRole('button', { name: 'Cancel' })).toBeFocused();
    await page.keyboard.press('Shift+Tab');
    await expect(
      recoveryDialog.getByRole('button', { name: 'Discard save and start over' }),
    ).toBeFocused();
    await page.keyboard.press('Enter');
    const recoveredHeading = page.getByRole('heading', { name: 'Choose an adventure' });
    await expect(recoveredHeading).toBeVisible();
    await expect(recoveredHeading).toBeFocused();
    await expect(access(savePath)).rejects.toThrow();
    const status = await request.get(`${baseUrl}/api/v1/session/save-status`);
    expect(status.ok()).toBe(true);
    await expect(status.json()).resolves.toMatchObject({
      saveIdentity: savePath,
      state: 'empty',
      campaignId: null,
      revision: 0,
      persistenceError: null,
    });
    expect(browserErrors).toEqual([]);
  } finally {
    if (host !== undefined) {
      await stopHost(host);
    }
    await rm(directory, { force: true, recursive: true });
  }
});

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
