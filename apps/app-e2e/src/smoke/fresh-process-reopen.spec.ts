import { spawn, type ChildProcess } from "node:child_process";
import { mkdtemp, readFile, rm } from "node:fs/promises";
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

test("pending saves reject atomically and completed state survives a fresh Rust host", async ({
  page,
  request,
}) => {
  test.setTimeout(180_000);
  const directory = await mkdtemp(join(tmpdir(), "rusty-d20-reopen-"));
  const savePath = join(directory, "save.json");
  const port = await freePort();
  const baseUrl = `http://127.0.0.1:${port}`;
  let host: ChildProcess | undefined;

  try {
    host = startHost(port, savePath);
    await waitForHealth(baseUrl, host);
    await page.goto(baseUrl);
    await page
      .getByRole("button", {
        name: "New Adventure · The Warden's Gate",
        exact: true,
      })
      .click();
    await page
      .getByLabel("Equipment")
      .getByRole("button", { name: "Off Hand: Mara's buckler" })
      .click();
    await page
      .getByLabel("Inventory item actions")
      .getByRole("listitem")
      .filter({ hasText: "Mara's buckler" })
      .getByRole("button", { name: "Store" })
      .click();
    await page
      .getByLabel("Camp stash")
      .getByRole("listitem")
      .filter({ hasText: "Spare buckler" })
      .getByRole("button", { name: "Take" })
      .click();
    await page.getByRole("button", { name: "Spare buckler" }).click();
    await expect(
      page
        .getByLabel("Equipment")
        .getByRole("button", { name: "Off Hand: Spare buckler" }),
    ).toBeVisible();
    await expect(page.getByLabel("Armor defense readout")).toContainText("18");
    await page.getByRole("button", { name: "Save", exact: true }).click();
    await expect(page.getByText("Saved", { exact: true })).toBeVisible();
    const baseline = await sessionSnapshot(request, baseUrl);
    expect(
      baseline.campaign.party[0].loadout.equipmentSlots.find(
        (slot) => slot.id === "off-hand",
      )?.equipped?.entityId,
    ).toBe(204);
    const baselineFile = await readFile(savePath);

    await enterWardenEncounter(page);
    await page.getByRole("button", { name: "Precise Shot" }).click();
    await expect(
      page.getByRole("button", { name: "Save", exact: true }),
    ).toBeDisabled();
    await expect(
      page.getByText("Resolve the pending action before saving."),
    ).toBeVisible();
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
    await page.getByRole("button", { name: "Continue Adventure" }).click();
    await expect(
      page
        .getByLabel("Equipment")
        .getByRole("button", { name: "Off Hand: Spare buckler" }),
    ).toBeVisible();
    await enterWardenEncounter(page);
    await expect(page.getByLabel("Encounter identity")).toContainText(
      "Armor defense 18",
    );
    await page.getByRole("button", { name: "Longsword Strike" }).click();
    await page.getByRole("button", { name: /Parry · 1 Guard/ }).click();
    await expect(
      page.getByRole("button", { name: "Save", exact: true }),
    ).toBeDisabled();
    const reacted = await sessionSnapshot(request, baseUrl);
    expect(reacted.encounter.pendingAction).not.toBeNull();
    const reactedOpponent = reacted.encounter.participants.find(
      (participant) =>
        participant.character.id ===
        reacted.encounter.pendingAction?.targetId,
    )?.character;
    expect(reactedOpponent?.resources).toContainEqual({
      current: 1,
      id: "guard",
      label: "Guard",
      maximum: 2,
    });
    expect(
      reactedOpponent?.effects.some((effect) =>
        effect.startsWith("Parry Stance"),
      ),
    ).toBe(true);
    await expectPendingSaveRejection(request, baseUrl, reacted.revision);
    expect(await sessionSnapshot(request, baseUrl)).toEqual(reacted);
    expect(await readFile(savePath)).toEqual(baselineFile);
    await stopHost(host);
    host = undefined;

    host = startHost(port, savePath);
    await waitForHealth(baseUrl, host);
    expect(await sessionSnapshot(request, baseUrl)).toEqual(baseline);
    await page.goto(baseUrl);
    await page.getByRole("button", { name: "Continue Adventure" }).click();
    await enterWardenEncounter(page);
    await page.getByRole("button", { name: "Precise Shot" }).click();
    await page
      .getByRole("button", { name: "Resolve deterministic roll" })
      .click();
    await page.getByRole("button", { name: "Save", exact: true }).click();
    await expect(page.getByText("Saved", { exact: true })).toBeVisible();
    const before = (await page.getByLabel("Encounter identity").innerText())
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => line.length > 0);
    await stopHost(host);
    host = undefined;

    host = startHost(port, savePath);
    await waitForHealth(baseUrl, host);
    await page.goto(baseUrl);
    await page.getByRole("button", { name: "Continue Adventure" }).click();
    await expect(page.getByText("Saved", { exact: true })).toBeVisible();
    for (const line of before) {
      await expect(page.getByLabel("Encounter identity")).toContainText(line);
    }
    expect((await sessionSnapshot(request, baseUrl)).encounter.nextRoll).toBe(
      1,
    );

    await advanceOneActivation(page, request, baseUrl);
    await advanceOneActivation(page, request, baseUrl);
    expect(
      (await sessionSnapshot(request, baseUrl)).encounter.nextRoll,
    ).toBeGreaterThanOrEqual(2);
    await page.reload();
    await page.getByRole("button", { name: "Continue Adventure" }).click();
    await expect(page.getByLabel("Encounter identity")).toContainText(
      "Iron Warden acting",
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
  name: string;
  resources: Array<{
    current: number;
    id: string;
    label: string;
    maximum: number;
  }>;
  effects: string[];
}

async function enterWardenEncounter(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Enter the dungeon" }).click();
  for (let step = 0; step < 8; step += 1) {
    await page.getByRole("button", { name: "↑ Forward" }).click();
  }
  await expect(page.getByLabel("Encounter identity")).toContainText(
    "Mara Venn acting",
  );
}

interface GameSnapshot {
  revision: number;
  campaign: {
    party: [
      {
        loadout: {
          equipmentSlots: Array<{
            id: string;
            equipped: { entityId: number } | null;
          }>;
        };
      },
      ...Array<{
        loadout: {
          equipmentSlots: Array<{
            id: string;
            equipped: { entityId: number } | null;
          }>;
        };
      }>,
    ];
  };
  encounter: {
    currentActorId: number;
    nextRoll: number;
    participants: Array<{
      character: GameCharacter;
      faction: "party" | "opposition";
    }>;
    actions: Array<{ id: string; label: string }>;
    legalTargets: Array<{ actionId: string; targetIds: number[] }>;
    pendingAction: { token: string; targetId: number } | null;
  };
}

async function advanceOneActivation(
  _page: Page,
  request: APIRequestContext,
  baseUrl: string,
): Promise<void> {
  const snapshot = await sessionSnapshot(request, baseUrl);
  const current = snapshot.encounter.participants.find(
    (participant) =>
      participant.character.id === snapshot.encounter.currentActorId,
  );
  expect(current).toBeDefined();
  if (current?.faction === "opposition") {
    const selected = await postSnapshot(
      request,
      baseUrl,
      "/api/v1/session/opposition",
      { expectedRevision: snapshot.revision },
    );
    const pending = selected.encounter.pendingAction;
    if (pending !== null) {
      await postSnapshot(request, baseUrl, "/api/v1/session/action", {
        expectedRevision: selected.revision,
        previewToken: pending.token,
      });
    }
  } else {
    const action = snapshot.encounter.actions[0];
    if (action === undefined) {
      await postSnapshot(request, baseUrl, "/api/v1/session/activation/end", {
        expectedRevision: snapshot.revision,
      });
      return;
    }
    const target = snapshot.encounter.legalTargets.find(
      (entry) => entry.actionId === action.id,
    )?.targetIds[0];
    if (target === undefined) {
      await postSnapshot(request, baseUrl, "/api/v1/session/activation/end", {
        expectedRevision: snapshot.revision,
      });
      return;
    }
    const previewed = await postSnapshot(
      request,
      baseUrl,
      "/api/v1/session/preview",
      {
        expectedRevision: snapshot.revision,
        actorId: snapshot.encounter.currentActorId,
        targetId: target,
        actionId: action.id,
      },
    );
    const pending = previewed.encounter.pendingAction;
    if (pending === null) {
      throw new Error("Party action did not produce an authoritative preview.");
    }
    await postSnapshot(request, baseUrl, "/api/v1/session/action", {
      expectedRevision: previewed.revision,
      previewToken: pending.token,
    });
  }
}

async function postSnapshot(
  request: APIRequestContext,
  baseUrl: string,
  path: string,
  data: unknown,
): Promise<GameSnapshot> {
  const response = await request.post(`${baseUrl}${path}`, { data });
  expect(response.ok()).toBe(true);
  return response.json() as Promise<GameSnapshot>;
}

async function sessionSnapshot(
  request: APIRequestContext,
  baseUrl: string,
): Promise<GameSnapshot> {
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
    kind: "invalid",
    message: "resolve the pending action before saving",
    retryable: false,
  });
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
      // Host is still compiling or binding.
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
