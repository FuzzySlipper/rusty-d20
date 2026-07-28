import type { Page, TestInfo } from '@playwright/test';
import { mkdir, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import type { LiveBaseUrl } from './live-gate';

interface ConsoleEntry {
  readonly type: string;
  readonly text: string;
  readonly location: string;
}

interface PageErrorEntry {
  readonly message: string;
  readonly stack?: string;
}

interface Milestone {
  readonly name: string;
  readonly atMs: number;
  readonly elapsedMs: number;
  readonly screenshot?: string;
  readonly visibleText?: string;
  readonly layerSnapshot?: Record<string, unknown>;
}

interface EvidencePacket {
  readonly title: string;
  readonly startedAt: string;
  readonly durationMs: number;
  readonly baseUrl: LiveBaseUrl;
  readonly milestones: readonly Milestone[];
  readonly console: readonly ConsoleEntry[];
  readonly pageErrors: readonly PageErrorEntry[];
  readonly nonClaims: readonly string[];
}

export class ArtifactCollector {
  private readonly startedAtMs = Date.now();
  private readonly consoleEntries: ConsoleEntry[] = [];
  private readonly pageErrors: PageErrorEntry[] = [];
  private readonly milestones: Milestone[] = [];
  private readonly nonClaims: string[] = [];

  constructor(
    private readonly page: Page,
    private readonly testInfo: TestInfo,
    private readonly baseUrl: LiveBaseUrl,
  ) {}

  get artifactDir(): string {
    return this.testInfo.outputPath('live-artifacts');
  }

  async start(): Promise<void> {
    await mkdir(this.artifactDir, { recursive: true });
    this.page.on('console', (message) => {
      const location = message.location();
      this.consoleEntries.push({
        type: message.type(),
        text: message.text(),
        location: `${location.url}:${location.lineNumber}:${location.columnNumber}`,
      });
    });
    this.page.on('pageerror', (error) => {
      const entry: PageErrorEntry = { message: error.message };
      if (error.stack !== undefined) {
        this.pageErrors.push({ ...entry, stack: error.stack });
        return;
      }
      this.pageErrors.push(entry);
    });
  }

  addNonClaim(nonClaim: string): void {
    this.nonClaims.push(nonClaim);
  }

  async milestone(name: string, options: { readonly screenshot?: boolean; readonly layerSnapshot?: Record<string, unknown> } = {}): Promise<void> {
    const visibleText = await this.page.locator('body').innerText().catch(() => '');
    let screenshot: string | undefined;
    if (options.screenshot === true) {
      screenshot = `${slugify(name)}.png`;
      await this.page.screenshot({ path: join(this.artifactDir, screenshot), fullPage: true });
    }

    const milestone: Milestone = {
      name,
      atMs: Date.now(),
      elapsedMs: Date.now() - this.startedAtMs,
      visibleText,
    };
    this.milestones.push(withOptionalFields(milestone, screenshot, options.layerSnapshot));
  }

  async finish(): Promise<void> {
    const packet: EvidencePacket = {
      title: this.testInfo.title,
      startedAt: new Date(this.startedAtMs).toISOString(),
      durationMs: Date.now() - this.startedAtMs,
      baseUrl: this.baseUrl,
      milestones: this.milestones,
      console: this.consoleEntries,
      pageErrors: this.pageErrors,
      nonClaims: this.nonClaims,
    };

    await writeFile(join(this.artifactDir, 'console.json'), `${JSON.stringify(this.consoleEntries, null, 2)}\n`);
    await writeFile(join(this.artifactDir, 'page-errors.json'), `${JSON.stringify(this.pageErrors, null, 2)}\n`);
    await writeFile(join(this.artifactDir, 'evidence-packet.json'), `${JSON.stringify(packet, null, 2)}\n`);
    await writeFile(join(this.artifactDir, 'scenario-summary.md'), scenarioSummary(packet));
  }
}

function withOptionalFields(milestone: Milestone, screenshot: string | undefined, layerSnapshot: Record<string, unknown> | undefined): Milestone {
  return {
    ...milestone,
    ...(screenshot === undefined ? {} : { screenshot }),
    ...(layerSnapshot === undefined ? {} : { layerSnapshot }),
  };
}

function scenarioSummary(packet: EvidencePacket): string {
  return `Live scenario: ${packet.title}
Command:
Backend/profile:
Artifacts: ${packet.milestones.length} milestones in live-artifacts
Screenshots inspected:
Rendered behavior observed:
Evidence packet: evidence-packet.json
Timeline notes:
Supporting checks:
Non-claims / residual risk:
${packet.nonClaims.map((entry) => `- ${entry}`).join('\n')}
`;
}

function slugify(value: string): string {
  return value.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/(^-|-$)/g, '') || 'milestone';
}
