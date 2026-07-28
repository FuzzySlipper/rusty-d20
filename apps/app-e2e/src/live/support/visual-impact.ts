import type { Locator, Page } from '@playwright/test';
import { mkdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';

export interface VisualImpactResult {
  readonly name: string;
  readonly beforePath: string;
  readonly afterPath: string;
  readonly changedBytes: number;
  readonly minChangedBytes: number;
}

export async function captureVisualImpact(options: {
  readonly page: Page;
  readonly artifactDir: string;
  readonly name: string;
  readonly action: () => Promise<void>;
  readonly region?: Locator;
  readonly minChangedBytes?: number;
}): Promise<VisualImpactResult> {
  await mkdir(options.artifactDir, { recursive: true });
  const beforePath = join(options.artifactDir, `${options.name}-before.png`);
  const afterPath = join(options.artifactDir, `${options.name}-after.png`);
  const target = options.region ?? options.page;
  await target.screenshot({ path: beforePath });
  await options.action();
  await target.screenshot({ path: afterPath });
  const changedBytes = await byteDelta(beforePath, afterPath);
  const minChangedBytes = options.minChangedBytes ?? 1;
  if (changedBytes < minChangedBytes) {
    throw new Error(`Visual impact ${options.name} changed ${changedBytes} bytes, expected at least ${minChangedBytes}`);
  }
  return { name: options.name, beforePath, afterPath, changedBytes, minChangedBytes };
}

async function byteDelta(leftPath: string, rightPath: string): Promise<number> {
  const [left, right] = await Promise.all([readFile(leftPath), readFile(rightPath)]);
  const length = Math.max(left.length, right.length);
  let changed = 0;
  for (let index = 0; index < length; index += 1) {
    if (left[index] !== right[index]) {
      changed += 1;
    }
  }
  return changed;
}
