import { spawn, type ChildProcess } from 'node:child_process';
import { mkdtemp, rm } from 'node:fs/promises';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { expect, test } from '@playwright/test';
import { workspaceRoot } from '@nx/devkit';

test('saved authoritative state survives a fresh Rust host process', async ({ page }) => {
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
    await page.getByRole('button', { name: 'Start encounter' }).click();
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
