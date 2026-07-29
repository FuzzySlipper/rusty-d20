import { spawn, type ChildProcess } from "node:child_process";
import { mkdtemp, rm } from "node:fs/promises";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  expect,
  test,
  type APIRequestContext,
  type Page,
} from "@playwright/test";
import { workspaceRoot } from "@nx/devkit";

test.describe.serial("complete deterministic encounter outcomes", () => {
  test("condition-forbidden opposition actions are filtered without a browser deadlock", async ({
    page,
    request,
  }, testInfo) => {
    test.setTimeout(120_000);
    const host = await startIsolatedHost("legal-opposition");
    const browserErrors: string[] = [];
    page.on("pageerror", (error) => browserErrors.push(error.message));
    page.on("console", (message) => {
      if (message.type() === "error") {
        browserErrors.push(message.text());
      }
    });
    try {
      await page.goto(host.baseUrl);
      await startAdventure(page, "The Warden's Gate");
      await enterWardenEncounter(page);

      for (let activation = 0; activation < 96; activation += 1) {
        const current = await sessionSnapshot(request, host.baseUrl);
        const encounter = current.encounter;
        if (encounter === null || encounter.currentActorId === null) {
          throw new Error(
            "Encounter resolved before the Unsettled regression was exercised.",
          );
        }
        const actor = encounter.participants.find(
          (participant) =>
            participant.character.id === encounter.currentActorId,
        );
        if (actor === undefined) {
          throw new Error(
            "Current actor is absent from the authoritative roster.",
          );
        }
        const warden = encounter.participants.find(
          (participant) => participant.character.id === 102,
        );
        const wardenUnsettled =
          warden?.character.effects.some((effect) =>
            effect.startsWith("Unsettled"),
          ) ?? false;

        if (actor.faction === "party") {
          if (actor.character.id === 101) {
            const previewed = await postSnapshot(
              request,
              host.baseUrl,
              "/api/v1/session/preview",
              {
                expectedRevision: current.revision,
                actorId: 101,
                targetId: 102,
                actionId: "disrupt",
              },
            );
            const token = previewed.encounter?.pendingAction?.token;
            if (token === undefined) {
              throw new Error(
                "Disrupt did not produce an authoritative preview.",
              );
            }
            await postSnapshot(
              request,
              host.baseUrl,
              "/api/v1/session/action",
              {
                expectedRevision: previewed.revision,
                previewToken: token,
              },
            );
          } else {
            await postSnapshot(
              request,
              host.baseUrl,
              "/api/v1/session/activation/end",
              {
                expectedRevision: current.revision,
              },
            );
          }
          continue;
        }

        const selected = await postSnapshot(
          request,
          host.baseUrl,
          "/api/v1/session/opposition",
          { expectedRevision: current.revision },
        );
        const pending = selected.encounter?.pendingAction;
        if (actor.character.id === 102 && wardenUnsettled) {
          expect(pending?.actionId).toMatch(
            /^(longsword-strike|precise-shot)$/,
          );
          await page.reload();
          await page
            .getByRole("button", { name: "Continue Adventure" })
            .click();
          const preview = page.getByLabel("Authoritative action preview");
          await expect(preview).toContainText("Iron Warden");
          await expect(preview).toContainText(/Longsword Strike|Precise Shot/);
          await expect(preview).not.toContainText(/Pin In Place|Disrupt/);
          await testInfo.attach("legal-opposition-after-unsettled.png", {
            body: await page.screenshot({ fullPage: true }),
            contentType: "image/png",
          });
          expect(browserErrors).toEqual([]);
          return;
        }
        if (pending !== null && pending !== undefined) {
          await postSnapshot(request, host.baseUrl, "/api/v1/session/action", {
            expectedRevision: selected.revision,
            previewToken: pending.token,
          });
        }
      }

      throw new Error(
        "The deterministic browser sequence never applied Unsettled.",
      );
    } finally {
      await host.stop();
    }
  });

  test("victory reward and the next authored encounter survive complete campaign reopen", async ({
    page,
    request,
  }, testInfo) => {
    test.setTimeout(180_000);
    const host = await startIsolatedHost("victory");
    const browserErrors: string[] = [];
    page.on("pageerror", (error) => browserErrors.push(error.message));
    page.on("console", (message) => {
      if (message.type() === "error") {
        browserErrors.push(message.text());
      }
    });
    try {
      await page.goto(host.baseUrl);
      await startAdventure(page, "The Warden's Gate");
      await enterWardenEncounter(page);
      await playToOutcome(
        page,
        request,
        host.baseUrl,
        "Precise Shot",
        "Iron Warden",
        false,
        true,
      );

      await expect(page.getByLabel("Encounter victory")).toContainText(
        "The Iron Warden defeated",
      );
      await expect(page.getByLabel("Encounter victory")).toContainText(
        "Warden chain armor",
      );
      await testInfo.attach("warden-victory.png", {
        body: await page.screenshot({ fullPage: true }),
        contentType: "image/png",
      });
      await page.getByRole("button", { name: "Save", exact: true }).click();
      const outcome = await sessionSnapshot(request, host.baseUrl);
      expect(outcome.campaign.phase).toBe("outcome");
      expect(outcome.campaign.latestOutcome).toMatchObject({
        kind: "victory",
        rewardItemId: 201,
      });
      expect(outcome.campaign.party[0].loadout.stashItems).toContainEqual(
        expect.objectContaining({ entityId: 201, name: "Warden chain armor" }),
      );
      expect(outcome.encounter?.currentActorId).toBeNull();

      await host.restart();
      await page.goto(host.baseUrl);
      await page.getByRole("button", { name: "Continue Adventure" }).click();
      await expect(page.getByLabel("Encounter victory")).toBeVisible();
      await page.getByRole("button", { name: "Continue adventure" }).click();
      await expect(
        page
          .getByRole("region", { name: "Dungeon exploration" })
          .getByRole("heading", { name: "Warden's Gate Pass" })
          .first(),
      ).toBeVisible();
      await enterWardenReckoning(page);
      await expect(page.getByLabel("Latest outcome explanation")).toContainText(
        "bounded vitality track service",
      );
      await playToOutcome(
        page,
        request,
        host.baseUrl,
        "Precise Shot",
        "Iron Warden",
        false,
        true,
        undefined,
        true,
      );
      await expect(page.getByLabel("Encounter defeat")).toContainText(
        "Mara fell at the reckoning",
      );
      await expect(page.getByLabel("Encounter defeat")).toContainText(
        "without granting a reward",
      );
      await page.getByRole("button", { name: "Save", exact: true }).click();

      await host.restart();
      await page.goto(host.baseUrl);
      await page.getByRole("button", { name: "Continue Adventure" }).click();
      await expect(page.getByLabel("Encounter defeat")).toContainText(
        "Mara fell at the reckoning",
      );
      await page
        .getByRole("button", { name: "Return to The Warden's Gate Camp" })
        .click();
      await expect(page.getByLabel("Completed encounters")).toContainText(
        "The Warden's Reckoning",
      );
      await expect(page.getByRole("button", { name: /^Enter / })).toHaveCount(
        0,
      );
      await page.getByRole("button", { name: "Save", exact: true }).click();

      await host.restart();
      const reopened = await sessionSnapshot(request, host.baseUrl);
      expect(reopened.campaign.phase).toBe("camp");
      expect(reopened.campaign.completedEncounters).toEqual([
        expect.objectContaining({
          encounterId: "iron-warden",
          outcome: "victory",
        }),
        expect.objectContaining({
          encounterId: "wardens-reckoning",
          outcome: "defeat",
        }),
      ]);
      expect(
        reopened.campaign.party.every(
          (member) => member.character.healthCurrent === 12,
        ),
      ).toBe(true);
      expect(
        reopened.campaign.party[0].loadout.stashItems.filter(
          (item) => item.entityId === 201,
        ),
      ).toHaveLength(1);
      expect(reopened.encounter).toBeNull();
      expect(browserErrors).toEqual([]);
    } finally {
      await host.stop();
    }
  });

  test("defeat grants no reward and applies bounded camp recovery on mobile", async ({
    page,
    request,
  }, testInfo) => {
    test.setTimeout(180_000);
    const host = await startIsolatedHost("defeat");
    try {
      await page.setViewportSize({ width: 390, height: 844 });
      await page.goto(host.baseUrl);
      await startAdventure(page, "The Warden's Gate");
      await page
        .getByLabel("Equipment")
        .getByRole("button", { name: /Body: Mara's chain armor/ })
        .click();
      await page
        .getByLabel("Equipment")
        .getByRole("button", { name: /Off Hand: Mara's buckler/ })
        .click();
      await expect(page.getByLabel("Armor defense readout")).toContainText(
        "14",
      );
      await enterWardenEncounter(page);
      await playToOutcome(
        page,
        request,
        host.baseUrl,
        "Longsword Strike",
        "Iron Warden",
        true,
        false,
        undefined,
        true,
      );

      await expect(page.getByLabel("Encounter defeat")).toContainText(
        "The company was defeated",
      );
      await testInfo.attach("mobile-defeat.png", {
        body: await page.screenshot({ fullPage: true }),
        contentType: "image/png",
      });
      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= window.innerWidth,
        ),
      ).toBe(true);
      await page.getByRole("button", { name: "Save", exact: true }).click();
      const outcome = await sessionSnapshot(request, host.baseUrl);
      expect(outcome.campaign.latestOutcome).toMatchObject({
        kind: "defeat",
        rewardItemId: null,
      });
      expect(
        outcome.campaign.party[0].loadout.stashItems.some(
          (item) => item.entityId === 201,
        ),
      ).toBe(false);
      expect(
        outcome.campaign.party.every(
          (member) => member.character.healthCurrent === 0,
        ),
      ).toBe(true);

      await host.restart();
      await page.goto(host.baseUrl);
      await page.getByRole("button", { name: "Continue Adventure" }).click();
      await expect(page.getByLabel("Encounter defeat")).toBeVisible();
      await page
        .getByRole("button", { name: "Return to The Warden's Gate Camp" })
        .click();
      await page.getByRole("button", { name: "Save", exact: true }).click();
      const recovered = await sessionSnapshot(request, host.baseUrl);
      expect(recovered.campaign.phase).toBe("camp");
      expect(
        recovered.campaign.party.every(
          (member) => member.character.healthCurrent === 12,
        ),
      ).toBe(true);
      expect(recovered.campaign.latestOutcome?.kind).toBe("defeat");

      await host.restart();
      expect(
        (await sessionSnapshot(request, host.baseUrl)).campaign.party.every(
          (member) => member.character.healthCurrent === 12,
        ),
      ).toBe(true);
      await page.goto(host.baseUrl);
      await page.getByRole("button", { name: "Continue Adventure" }).click();
      await page.getByRole("button", { name: "Enter the dungeon" }).click();
      await stepForward(page, 8);

      const pastCompletedTrigger = await sessionSnapshot(request, host.baseUrl);
      expect(pastCompletedTrigger.campaign.phase).toBe("exploration");
      expect(pastCompletedTrigger.campaign.activeEncounterId).toBeNull();
      expect(pastCompletedTrigger.exploration).toEqual(
        expect.objectContaining({ x: 9, y: 1 }),
      );

      await enterWardenReckoning(page);
      await expect(page.getByLabel("Encounter identity")).toBeVisible();
      await expect(
        page.getByRole("button", { name: "Precise Shot" }),
      ).toBeVisible();
      const continued = await sessionSnapshot(request, host.baseUrl);
      expect(continued.campaign.phase).toBe("encounter");
      expect(continued.campaign.activeEncounterId).toBe("wardens-reckoning");
    } finally {
      await host.stop();
    }
  });

  test("Ember path exposes distinct authored mechanics and survives a complete fresh-process victory", async ({
    page,
    request,
  }) => {
    test.setTimeout(180_000);
    const host = await startIsolatedHost("ember-victory");
    try {
      await page.setViewportSize({ width: 390, height: 844 });
      await page.goto(host.baseUrl);
      await startAdventure(page, "Ember's Wake");
      await expect(
        page.getByRole("heading", { name: "Ember's Wake Camp" }),
      ).toBeVisible();
      await expect(
        page.getByLabel("Character status").getByText("Sera Vale", {
          exact: true,
        }),
      ).toBeVisible();
      await expect(page.getByLabel("Nerve defense readout")).toContainText(
        "16",
      );
      await expect(page.getByLabel("Nerve defense readout")).toContainText(
        "Equipped item 212",
      );
      await expect(page.getByLabel("Nerve defense readout")).toContainText(
        "Equipped item 213",
      );
      await expect(page.getByLabel("Nerve defense readout")).toContainText(
        "suppressed",
      );
      await expect(page.getByLabel("Camp stash")).toContainText(
        "Spare runed robe",
      );
      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= window.innerWidth,
        ),
      ).toBe(true);
      await page.getByRole("button", { name: "Save", exact: true }).click();

      const camp = await sessionSnapshot(request, host.baseUrl);
      expect(camp.campaign).toMatchObject({
        id: "embers-wake",
        title: "Ember's Wake",
        phase: "camp",
      });
      const emberFingerprint = camp.rulesetFingerprint;

      await host.restart();
      expect(await sessionSnapshot(request, host.baseUrl)).toEqual(camp);
      await page.goto(host.baseUrl);
      await page.getByRole("button", { name: "Continue Adventure" }).click();
      await expect(
        page.getByRole("heading", { name: "Ember's Wake Camp" }),
      ).toBeVisible();
      await enterAshSeerEncounter(page);
      await expect(
        page
          .locator("aui-character-status")
          .nth(1)
          .getByText("Ash Seer", { exact: true }),
      ).toBeVisible();
      await expect(page.getByText("Focus 3/3", { exact: true })).toHaveCount(2);
      await expect(
        page.getByRole("button", { name: "Fire Bolt", exact: true }),
      ).toBeVisible();
      await expect(
        page.getByRole("button", { name: "Mind Spike", exact: true }),
      ).toBeVisible();

      await playToOutcome(
        page,
        request,
        host.baseUrl,
        "Fire Bolt",
        "Ash Seer",
        false,
        true,
        /Fire Bolt|fire|Scorched/i,
      );
      await expect(page.getByLabel("Encounter victory")).toContainText(
        "The Ash Seer defeated",
      );
      await expect(page.getByLabel("Encounter victory")).toContainText(
        "Ash Seer's mindward charm",
      );
      await page.getByRole("button", { name: "Save", exact: true }).click();
      const outcome = await sessionSnapshot(request, host.baseUrl);
      expect(outcome.rulesetFingerprint).toBe(emberFingerprint);
      expect(outcome.campaign.latestOutcome).toMatchObject({
        kind: "victory",
        rewardItemId: 211,
      });

      await host.restart();
      expect(await sessionSnapshot(request, host.baseUrl)).toEqual(outcome);
      await page.goto(host.baseUrl);
      await page.getByRole("button", { name: "Continue Adventure" }).click();
      await expect(page.getByLabel("Encounter victory")).toBeVisible();
      await page.getByRole("button", { name: "Continue adventure" }).click();
      await expect(page.getByLabel("Camp stash")).toContainText(
        "Ash Seer's mindward charm",
      );
      await page.getByRole("button", { name: "Save", exact: true }).click();

      await host.restart();
      const reopened = await sessionSnapshot(request, host.baseUrl);
      expect(reopened.campaign.id).toBe("embers-wake");
      expect(reopened.rulesetFingerprint).toBe(emberFingerprint);
      expect(
        reopened.campaign.party[0].loadout.stashItems.filter(
          (item) => item.entityId === 211,
        ),
      ).toHaveLength(1);
    } finally {
      await host.stop();
    }
  });
});

async function playToOutcome(
  page: Page,
  request: APIRequestContext,
  baseUrl: string,
  playerAction: string,
  oppositionName: string,
  oppositionReacts: boolean,
  playerReacts: boolean,
  expectedPlayerReceipt?: RegExp,
  passParty = false,
): Promise<void> {
  let inspectedFirstPartyReceipt = false;
  for (let activation = 0; activation < 512; activation += 1) {
    const current = await sessionSnapshot(request, baseUrl);
    if (current.campaign.phase === "outcome") {
      await page.reload();
      await page.getByRole("button", { name: "Continue Adventure" }).click();
      return;
    }
    const encounter = current.encounter;
    if (encounter === null || encounter.currentActorId === null) {
      throw new Error("Active campaign has no current encounter actor.");
    }
    const actor = encounter.participants.find(
      (participant) => participant.character.id === encounter.currentActorId,
    );
    if (actor === undefined) {
      throw new Error("Current actor is absent from the encounter roster.");
    }

    if (actor.faction === "party") {
      if (passParty) {
        await postSnapshot(request, baseUrl, "/api/v1/session/activation/end", {
          expectedRevision: current.revision,
        });
        continue;
      }
      const action =
        encounter.actions.find(
          (candidate) => candidate.label === playerAction,
        ) ?? encounter.actions[0];
      if (action === undefined) {
        await postSnapshot(request, baseUrl, "/api/v1/session/activation/end", {
          expectedRevision: current.revision,
        });
        continue;
      }
      const legalTargets = encounter.legalTargets.find(
        (entry) => entry.actionId === action.id,
      )?.targetIds;
      const preferredTarget = encounter.participants.find(
        (participant) =>
          participant.character.name === oppositionName &&
          legalTargets?.includes(participant.character.id),
      )?.character.id;
      const targetId = preferredTarget ?? legalTargets?.[0];
      if (targetId === undefined) {
        throw new Error(`Action ${action.id} has no legal target.`);
      }
      let previewed = await postSnapshot(
        request,
        baseUrl,
        "/api/v1/session/preview",
        {
          expectedRevision: current.revision,
          actorId: actor.character.id,
          targetId,
          actionId: action.id,
        },
      );
      if (oppositionReacts) {
        previewed = await applyFirstApiReactionIfAvailable(
          request,
          baseUrl,
          previewed,
        );
      }
      const pending = previewed.encounter?.pendingAction;
      if (pending === null || pending === undefined) {
        throw new Error("Party action lost its authoritative preview.");
      }
      const resolved = await postSnapshot(
        request,
        baseUrl,
        "/api/v1/session/action",
        {
          expectedRevision: previewed.revision,
          previewToken: pending.token,
        },
      );
      if (!inspectedFirstPartyReceipt && expectedPlayerReceipt !== undefined) {
        expect(JSON.stringify(resolved.encounter?.log)).toMatch(
          expectedPlayerReceipt,
        );
        inspectedFirstPartyReceipt = true;
      }
      continue;
    }

    let selected = await postSnapshot(
      request,
      baseUrl,
      "/api/v1/session/opposition",
      { expectedRevision: current.revision },
    );
    if (playerReacts) {
      selected = await applyFirstApiReactionIfAvailable(
        request,
        baseUrl,
        selected,
      );
    }
    const pending = selected.encounter?.pendingAction;
    if (pending !== null && pending !== undefined) {
      await postSnapshot(request, baseUrl, "/api/v1/session/action", {
        expectedRevision: selected.revision,
        previewToken: pending.token,
      });
    }
  }
  throw new Error(
    `Deterministic encounter against ${oppositionName} did not reach an outcome within 512 activations.`,
  );
}

async function applyFirstApiReactionIfAvailable(
  request: APIRequestContext,
  baseUrl: string,
  snapshot: SessionSnapshot,
): Promise<SessionSnapshot> {
  const pending = snapshot.encounter?.pendingAction;
  const reaction = pending?.reactions[0];
  if (pending === null || pending === undefined || reaction === undefined) {
    return snapshot;
  }
  return postSnapshot(request, baseUrl, "/api/v1/session/reaction", {
    expectedRevision: snapshot.revision,
    previewToken: pending.token,
    reactionId: reaction.id,
  });
}

async function startAdventure(page: Page, title: string): Promise<void> {
  await page
    .getByRole("button", {
      name: `New Adventure · ${title}`,
      exact: true,
    })
    .click();
}

async function enterWardenEncounter(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Enter the dungeon" }).click();
  await stepForward(page, 8);
}

async function enterWardenReckoning(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Right ↷" }).click();
  await stepForward(page, 4);
  await page.getByRole("button", { name: "Right ↷" }).click();
  await stepForward(page, 8);
}

async function enterAshSeerEncounter(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Enter the dungeon" }).click();
  await stepForward(page, 6);
  await page.getByRole("button", { name: "Right ↷" }).click();
  await stepForward(page, 4);
}

async function stepForward(page: Page, count: number): Promise<void> {
  for (let step = 0; step < count; step += 1) {
    await page.getByRole("button", { name: "↑ Forward" }).click();
  }
}

interface SessionSnapshot {
  revision: number;
  rulesetFingerprint: string;
  campaign: {
    id: string;
    title: string;
    phase: "camp" | "exploration" | "encounter" | "outcome";
    activeEncounterId: string | null;
    party: [SessionPartyMember, ...SessionPartyMember[]];
    latestOutcome: {
      kind: "victory" | "defeat";
      rewardItemId: number | null;
    } | null;
    completedEncounters: Array<{
      encounterId: string;
      outcome: "victory" | "defeat";
    }>;
  };
  exploration: { x: number; y: number } | null;
  encounter: {
    currentActorId: number | null;
    participants: Array<{
      character: {
        id: number;
        name: string;
        effects: string[];
      };
      faction: "party" | "opposition";
    }>;
    actions: Array<{ id: string; label: string }>;
    legalTargets: Array<{ actionId: string; targetIds: number[] }>;
    log: Array<{ details: string[] }>;
    pendingAction: {
      token: string;
      actionId: string;
      reactions: Array<{ id: string }>;
    } | null;
  } | null;
}

interface SessionPartyMember {
  character: { healthCurrent: number };
  loadout: {
    stashItems: Array<{ entityId: number; name: string }>;
  };
}

async function sessionSnapshot(
  request: APIRequestContext,
  baseUrl: string,
): Promise<SessionSnapshot> {
  const response = await request.get(`${baseUrl}/api/v1/session`);
  expect(response.ok()).toBe(true);
  return response.json() as Promise<SessionSnapshot>;
}

async function postSnapshot(
  request: APIRequestContext,
  baseUrl: string,
  path: string,
  data: unknown,
): Promise<SessionSnapshot> {
  const response = await request.post(`${baseUrl}${path}`, { data });
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
  const savePath = join(directory, "save.json");
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
    "cargo",
    [
      "run",
      "-p",
      "rusty-d20",
      "--bin",
      "rusty-d20-host",
      "--",
      "--address",
      `127.0.0.1:${port}`,
      "--save-file",
      savePath,
    ],
    { cwd: workspaceRoot, stdio: ["ignore", "pipe", "pipe"] },
  );
}

async function stopHost(host: ChildProcess): Promise<void> {
  if (host.exitCode !== null) {
    return;
  }
  const exited = new Promise<void>((resolve) =>
    host.once("exit", () => resolve()),
  );
  host.kill("SIGINT");
  await exited;
}

async function waitForHealth(
  baseUrl: string,
  host: ChildProcess,
): Promise<void> {
  const started = Date.now();
  while (Date.now() - started < 90_000) {
    if (host.exitCode !== null) {
      throw new Error(
        `Rust host exited before becoming ready with code ${host.exitCode}.`,
      );
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
  throw new Error("Rust host did not become ready.");
}

function freePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      server.close(() => {
        if (address !== null && typeof address === "object") {
          resolve(address.port);
        } else {
          reject(new Error("Could not allocate a local port."));
        }
      });
    });
    server.on("error", reject);
  });
}
