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

test("reaction prompts reject saves and automatically resolved rolls survive a fresh Rust host", async ({
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
    await page.setViewportSize({ width: 1280, height: 1000 });
    const maraEquipment = page.getByRole("region", {
      name: "Mara Venn equipment",
      exact: true,
    });
    const equippedBuckler = maraEquipment.getByRole("button", {
      name: /Off Hand: Mara's buckler/,
    });
    await equippedBuckler.click();
    await page.getByRole("button", { name: "Store" }).click();
    await expect(
      maraEquipment.getByRole("button", {
        name: /Off Hand: empty/,
      }),
    ).toBeVisible();
    await expect(page.getByLabel("Camp stash")).toContainText(
      "Shared slots 5/24",
    );
    await page
      .getByLabel("Shared camp inventory")
      .getByRole("button", { name: /Spare buckler/ })
      .click();
    await maraEquipment
      .getByRole("button", {
        name: /Off Hand: empty. Compatible destination/,
      })
      .click();
    await expect(
      maraEquipment.getByRole("button", { name: /Off Hand: Spare buckler/ }),
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
    await chooseActionOnGrid(page, "Precise Shot");
    const automatic = await sessionSnapshot(request, baseUrl);
    expect(automatic.encounter.reactionPrompt).toBeNull();
    expect(
      automatic.encounter.log.some((entry) =>
        entry.details.some((detail) =>
          detail.includes("Roll-source position 0"),
        ),
      ),
    ).toBe(true);
    expect(
      await page
        .getByRole("button", { name: "Resolve deterministic roll" })
        .count(),
    ).toBe(0);

    const prompted = await advanceToReactionPrompt(request, baseUrl);
    await page.reload();
    await page.getByRole("button", { name: "Continue Adventure" }).click();
    await expect(
      page.getByRole("button", { name: "Save", exact: true }),
    ).toBeDisabled();
    await expect(
      page.getByText("Choose or decline the reaction before saving."),
    ).toBeVisible();
    expect(prompted.encounter.reactionPrompt).not.toBeNull();
    await expectReactionPromptSaveRejection(
      request,
      baseUrl,
      prompted.revision,
    );
    expect(await sessionSnapshot(request, baseUrl)).toEqual(prompted);
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
        .getByRole("region", {
          name: "Mara Venn equipment",
          exact: true,
        })
        .getByRole("button", { name: /Off Hand: Spare buckler/ }),
    ).toBeVisible();
    await enterWardenEncounter(page);
    await expect(page.getByLabel("Encounter identity")).toContainText(
      "Armor defense 18",
    );
    await chooseActionOnGrid(page, "Precise Shot");
    await page.getByRole("button", { name: "Save", exact: true }).click();
    await expect(page.getByText("Saved", { exact: true })).toBeVisible();
    const saved = await sessionSnapshot(request, baseUrl);
    expect(saved.encounter.reactionPrompt).toBeNull();
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
    expect(await sessionSnapshot(request, baseUrl)).toEqual(saved);

    await advanceOneActivation(page, request, baseUrl);
    await advanceOneActivation(page, request, baseUrl);
    const advanced = await sessionSnapshot(request, baseUrl);
    expect(
      advanced.encounter.log.filter((entry) =>
        entry.details.some((detail) =>
          detail.startsWith("Roll-source position "),
        ),
      ).length,
    ).toBeGreaterThan(
      saved.encounter.log.filter((entry) =>
        entry.details.some((detail) =>
          detail.startsWith("Roll-source position "),
        ),
      ).length,
    );
    await page.reload();
    await page.getByRole("button", { name: "Continue Adventure" }).click();
    await expect(page.getByLabel("Encounter identity")).toBeVisible();
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
    participants: Array<{
      character: GameCharacter;
      faction: "party" | "opposition";
    }>;
    actions: Array<{ id: string; label: string }>;
    legalTargets: Array<{ actionId: string; targetIds: number[] }>;
    reactionPrompt: {
      token: string;
      targetId: number;
      reactions: Array<{ id: string }>;
    } | null;
    log: Array<{ details: string[] }>;
  };
}

async function advanceOneActivation(
  _page: Page,
  request: APIRequestContext,
  baseUrl: string,
): Promise<void> {
  const snapshot = await sessionSnapshot(request, baseUrl);
  if (snapshot.encounter.reactionPrompt !== null) {
    await postSnapshot(request, baseUrl, "/api/v1/session/reaction/decline", {
      expectedRevision: snapshot.revision,
      promptToken: snapshot.encounter.reactionPrompt.token,
    });
    return;
  }
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
    const prompt = selected.encounter.reactionPrompt;
    if (prompt !== null) {
      await postSnapshot(request, baseUrl, "/api/v1/session/reaction/decline", {
        expectedRevision: selected.revision,
        promptToken: prompt.token,
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
    await postSnapshot(request, baseUrl, "/api/v1/session/action", {
      expectedRevision: snapshot.revision,
      actorId: snapshot.encounter.currentActorId,
      targetId: target,
      actionId: action.id,
    });
  }
}

async function advanceToReactionPrompt(
  request: APIRequestContext,
  baseUrl: string,
): Promise<GameSnapshot> {
  for (let activation = 0; activation < 32; activation += 1) {
    const snapshot = await sessionSnapshot(request, baseUrl);
    if (snapshot.encounter.reactionPrompt !== null) {
      return snapshot;
    }
    const current = snapshot.encounter.participants.find(
      (participant) =>
        participant.character.id === snapshot.encounter.currentActorId,
    );
    if (current === undefined) {
      throw new Error("Current actor is absent from the encounter roster.");
    }
    const next = await postSnapshot(
      request,
      baseUrl,
      current.faction === "party"
        ? "/api/v1/session/activation/end"
        : "/api/v1/session/opposition",
      { expectedRevision: snapshot.revision },
    );
    if (next.encounter.reactionPrompt !== null) {
      return next;
    }
  }
  throw new Error("The authored encounter did not expose a reaction prompt.");
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

async function expectReactionPromptSaveRejection(
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
    message: "choose or decline the reaction before saving",
    retryable: false,
  });
}

async function chooseActionOnGrid(
  page: Page,
  actionLabel: string,
): Promise<void> {
  await page.getByRole("button", { name: actionLabel, exact: true }).click();
  const board = page.getByRole("application", {
    name: new RegExp(`Targeting ${actionLabel}`),
  });
  await expect(board).toHaveAttribute("data-interaction-mode", "targeting");
  await board.focus();
  await page.keyboard.press("Enter");
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
