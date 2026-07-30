import { expect, test, type Page } from "@playwright/test";

async function expectRendererCanvasAtPoint(
  page: Page,
  x: number,
  y: number,
): Promise<void> {
  const hit = await page.evaluate(
    ({ pointX, pointY }) => {
      const target = document.elementFromPoint(pointX, pointY);
      return {
        insideRenderer: target?.closest("aui-game-viewport") !== null,
        tagName: target?.tagName ?? null,
      };
    },
    { pointX: x, pointY: y },
  );

  expect(hit).toEqual({ insideRenderer: true, tagName: "CANVAS" });
}

async function clickRenderedTacticalCell(
  page: Page,
  x: number,
  y: number,
  boardWidth: number,
  boardHeight: number,
): Promise<void> {
  await page.evaluate(
    () =>
      new Promise<void>((resolve) => {
        requestAnimationFrame(() => requestAnimationFrame(() => resolve()));
      }),
  );
  const canvas = page.getByRole("application", {
    name: /Rendered tactical combat board/,
  });
  const point = await canvas.evaluate(
    (element, cell) => {
      const bounds = element.getBoundingClientRect();
      const cellSize = 0.84;
      const fitWidth = cell.boardWidth * cellSize;
      const fitHeight = cell.boardHeight * cellSize;
      const aspect = bounds.width / bounds.height;
      const halfFovRadians = (58 * Math.PI) / 360;
      const distanceForHeight = fitHeight / (2 * Math.tan(halfFovRadians));
      const distanceForWidth =
        fitWidth / (2 * Math.tan(halfFovRadians) * aspect);
      const distance =
        Math.max(distanceForHeight, distanceForWidth) * 1.12 + 0.8;
      const visibleHeight = 2 * distance * Math.tan(halfFovRadians);
      const worldX = (cell.x - (cell.boardWidth - 1) / 2) * cellSize;
      const worldZ = (cell.y - (cell.boardHeight - 1) / 2) * cellSize;
      return {
        x:
          bounds.left +
          bounds.width / 2 +
          (worldX / visibleHeight) * bounds.height,
        y:
          bounds.top +
          bounds.height / 2 +
          (worldZ / visibleHeight) * bounds.height,
      };
    },
    { x, y, boardWidth, boardHeight },
  );
  await page.mouse.click(point.x, point.y);
}

test.describe.serial("real Rust encounter shell", () => {
  test("loading projection is visible while Rust save status is pending", async ({
    page,
  }, testInfo) => {
    await page.route("**/api/v1/session/save-status", async (route) => {
      await new Promise((resolve) => setTimeout(resolve, 750));
      await route.continue();
    });
    await page.goto("/");
    await expect(
      page.getByText("Loading authored rules and Rust state…"),
    ).toBeVisible();
    await testInfo.attach("loading-state.png", {
      body: await page.screenshot({ fullPage: true }),
      contentType: "image/png",
    });
    await page.unroute("**/api/v1/session/save-status");
  });

  test("empty game starts and resolves authored action, reaction, turn, and save", async ({
    page,
    request,
  }, testInfo) => {
    const health = await request.get("/healthz");
    expect(health.ok()).toBe(true);
    await expect(health.json()).resolves.toEqual({
      status: "ok",
      version: "0.1.0",
    });

    await page.goto("/");
    await expect(page.locator("aui-game-viewport canvas")).toHaveCount(1);
    await expect(
      page.locator("aui-game-viewport [data-scene-mode]"),
    ).toHaveAttribute("data-scene-mode", "catalog");
    await page.locator("aui-game-viewport canvas").evaluate((canvas) => {
      canvas.setAttribute("data-lifecycle-witness", "persistent");
    });
    await expect(
      page.getByRole("heading", { level: 1, name: "Rusty D20", exact: true }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "The Warden's Gate" }),
    ).toBeVisible();
    await testInfo.attach("empty-adventure-catalog.png", {
      body: await page.screenshot({ fullPage: true }),
      contentType: "image/png",
    });
    await page
      .getByRole("button", {
        name: "New Adventure · The Warden's Gate",
        exact: true,
      })
      .click();
    await expect(
      page.getByRole("heading", { name: "The Warden's Gate Camp" }),
    ).toBeVisible();
    await expect(page.locator("aui-game-viewport canvas")).toHaveCount(1);
    await expect(
      page.locator("aui-game-viewport [data-scene-mode]"),
    ).toHaveAttribute("data-scene-mode", "camp");
    await expect(page.locator("aui-game-viewport canvas")).toHaveAttribute(
      "data-lifecycle-witness",
      "persistent",
    );
    await expect(page.getByLabel("Armor defense readout")).toContainText("18");
    await expect(
      page.getByRole("region", { name: "Mara Venn pack", exact: true }),
    ).toBeVisible();
    const maraEquipment = page.getByRole("region", {
      name: "Mara Venn equipment",
      exact: true,
    });
    await expect(maraEquipment).toBeVisible();
    await expect(maraEquipment).toContainText("Mara's training blade");
    await expect(maraEquipment).toContainText("Mara's field bow");
    await expect(page.getByLabel("Camp stash")).toContainText(
      "Shared slots 4/24",
    );
    await expect(
      page
        .getByLabel("Shared camp inventory")
        .getByRole("button", { name: /Spare chain armor/ }),
    ).toBeVisible();
    await expect(
      page
        .getByLabel("Shared camp inventory")
        .getByRole("button", { name: /Spare training blade/ }),
    ).toBeVisible();
    await testInfo.attach("renderer-root-camp-desktop.png", {
      body: await page.screenshot(),
      contentType: "image/png",
    });

    await page
      .getByLabel("Shared camp inventory")
      .getByRole("button", { name: /Spare buckler/ })
      .click();
    await page.getByRole("button", { name: "Move to pack" }).click();
    await expect(page.getByRole("alert")).toContainText("capacity rejection");
    await expect(page.getByRole("alert")).toContainText("maximum: 4");
    await testInfo.attach("capacity-rejection.png", {
      body: await page.screenshot({ fullPage: true }),
      contentType: "image/png",
    });
    await page.getByRole("button", { name: "Dismiss" }).click();
    await expect(page.getByLabel("Armor defense readout")).toContainText("18");
    await expect(page.getByText("Carried 4/4")).toBeVisible();

    await page.setViewportSize({ width: 1280, height: 1000 });
    const equippedBuckler = maraEquipment.getByRole("button", {
      name: /Off Hand: Mara's buckler/,
    });
    const campEmptySlot = page
      .getByLabel("Shared camp inventory")
      .getByRole("button", { name: "Empty slot 5" });
    await equippedBuckler.dragTo(campEmptySlot);
    await expect(page.getByLabel("Camp stash")).toContainText(
      "Shared slots 5/24",
    );
    const spareBuckler = page
      .getByLabel("Shared camp inventory")
      .getByRole("button", { name: /Spare buckler/ });
    const emptyOffHand = maraEquipment.getByRole("button", {
      name: /Off Hand: empty. Compatible destination/,
    });
    await spareBuckler.dragTo(emptyOffHand);
    await expect(maraEquipment).toContainText("Spare buckler");
    await expect(page.getByLabel("Camp stash")).toContainText(
      "Shared slots 4/24",
    );

    let releasePendingMove: (() => void) | undefined;
    const pendingMoveRelease = new Promise<void>((resolve) => {
      releasePendingMove = resolve;
    });
    let markPendingMoveStarted: (() => void) | undefined;
    const pendingMoveStarted = new Promise<void>((resolve) => {
      markPendingMoveStarted = resolve;
    });
    let delayedMoveRequests = 0;
    await page.route("**/api/v1/session/loadout/move", async (route) => {
      delayedMoveRequests += 1;
      if (delayedMoveRequests === 1) {
        markPendingMoveStarted?.();
        await pendingMoveRelease;
      }
      await route.continue();
    });
    await maraEquipment
      .getByRole("button", { name: /Off Hand: Spare buckler/ })
      .click();
    await page.getByRole("button", { name: "Store" }).click();
    await pendingMoveStarted;
    const spareChain = page
      .getByLabel("Shared camp inventory")
      .getByRole("button", { name: /Spare chain armor/ });
    const occupiedBody = maraEquipment.getByRole("button", {
      name: /Body: Mara's chain armor/,
    });
    await expect(spareChain).toHaveAttribute("aria-disabled", "true");
    await expect(spareChain).not.toHaveAttribute("draggable", "true");
    await expect(occupiedBody).toHaveAttribute("aria-disabled", "true");
    await spareChain.dragTo(occupiedBody);
    expect(delayedMoveRequests).toBe(1);
    await expect(page.getByRole("status")).toContainText(
      "Moving Spare buckler",
    );
    await expect(page.getByRole("status")).not.toContainText(
      "Spare chain armor moved",
    );

    releasePendingMove?.();
    await expect(
      maraEquipment.getByRole("button", { name: /Off Hand: empty/ }),
    ).toBeVisible();
    await expect(page.getByLabel("Camp stash")).toContainText(
      "Shared slots 5/24",
    );
    await page.unroute("**/api/v1/session/loadout/move");
    await page
      .getByLabel("Shared camp inventory")
      .getByRole("button", { name: /Spare buckler/ })
      .dragTo(
        maraEquipment.getByRole("button", {
          name: /Off Hand: empty. Compatible destination/,
        }),
      );
    await expect(maraEquipment).toContainText("Spare buckler");
    await expect(page.getByLabel("Camp stash")).toContainText(
      "Shared slots 4/24",
    );

    const chainInventory = page.getByRole("button", {
      name: /Mara's chain armor · equipped body/,
    });
    await chainInventory.focus();
    await chainInventory.press("Enter");
    await page.getByRole("button", { name: "Move to pack" }).click();
    await expect(page.getByLabel("Armor defense readout")).toContainText("16");
    const unequippedChain = page.getByRole("button", {
      name: /Mara's chain armor. Fits the body equipment slot/,
    });
    await unequippedChain.focus();
    await unequippedChain.press("Space");
    await maraEquipment
      .getByRole("button", {
        name: /Body: empty. Compatible destination/,
      })
      .press("Enter");
    await expect(page.getByLabel("Armor defense readout")).toContainText("18");
    await testInfo.attach("drag-loadout-preparation.png", {
      body: await page.screenshot({ fullPage: true }),
      contentType: "image/png",
    });

    await page.setViewportSize({ width: 390, height: 844 });
    await expect(page.locator("aui-character-status")).toHaveCount(1);
    await expect(
      page
        .getByRole("navigation", { name: "Party loadout selection" })
        .getByRole("button"),
    ).toHaveCount(4);
    await page
      .getByRole("navigation", { name: "Party loadout selection" })
      .getByRole("button", { name: "Ilyra Fen" })
      .click();
    await expect(
      page.getByLabel("Character status").getByText("Ilyra Fen", {
        exact: true,
      }),
    ).toBeVisible();
    await expect(
      page.getByRole("region", {
        name: "Ilyra Fen equipment",
        exact: true,
      }),
    ).toContainText("Ilyra's chain armor");
    await page
      .getByRole("navigation", { name: "Party loadout selection" })
      .getByRole("button", { name: "Mara Venn" })
      .click();
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth,
      ),
    ).toBe(true);
    await page.setViewportSize({ width: 1280, height: 720 });
    await page.getByRole("button", { name: "Enter the dungeon" }).click();
    await expect(
      page
        .getByRole("region", { name: "Dungeon exploration" })
        .getByRole("heading", { name: "Warden's Gate Pass" })
        .first(),
    ).toBeVisible();
    const dungeonViewport = page.getByRole("img", {
      name: /Warden's Gate Pass, facing east at cell 1, 1/,
    });
    await expect(dungeonViewport).toBeVisible();
    await expect(dungeonViewport).toHaveAttribute(
      "data-renderer-backend",
      "rusty-engine-three",
    );
    await expect(dungeonViewport.locator("canvas")).toBeVisible();
    await expect(dungeonViewport.getByRole("alert")).toHaveCount(0);
    await expectRendererCanvasAtPoint(page, 640, 400);
    const beforeExplorationInventory = await (
      await request.get("/api/v1/session")
    ).json();
    const inventoryTrigger = page.getByRole("button", { name: "Inventory" });
    await inventoryTrigger.click();
    const explorationInventory = page.getByRole("dialog", {
      name: "Party loadout",
    });
    await expect(explorationInventory).toBeVisible();
    await expect(
      explorationInventory.getByRole("button", { name: "Close" }),
    ).toBeFocused();
    await expect(
      explorationInventory
        .getByRole("region", { name: "Mara Venn pack", exact: true })
        .getByRole("button", {
          name: /Mara's chain armor/,
        }),
    ).toHaveAttribute("aria-disabled", "true");
    await testInfo.attach("exploration-inventory-overlay.png", {
      body: await page.screenshot({ fullPage: true }),
      contentType: "image/png",
    });
    const rejectedExplorationMove = await request.post(
      "/api/v1/session/loadout/move",
      {
        data: {
          expectedRevision: beforeExplorationInventory.revision,
          itemId: 202,
          fromOwnerId: 101,
          toOwnerId: 103,
          destinationSlotId: null,
        },
      },
    );
    expect(rejectedExplorationMove.status()).toBe(422);
    await expect(rejectedExplorationMove.json()).resolves.toMatchObject({
      kind: "phase",
    });
    await expect(
      (await request.get("/api/v1/session")).json(),
    ).resolves.toEqual(beforeExplorationInventory);
    await page.keyboard.press("Escape");
    await expect(explorationInventory).toHaveCount(0);
    await expect(inventoryTrigger).toBeFocused();
    await testInfo.attach("engine-dungeon-corridor.png", {
      body: await dungeonViewport.screenshot(),
      contentType: "image/png",
    });
    await page.setViewportSize({ width: 390, height: 844 });
    await expect(page.locator("aui-game-viewport canvas")).toBeVisible();
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth,
      ),
    ).toBe(true);
    const narrowPanels = page.locator(
      ".exploration__main > .rusty-engine-panel",
    );
    const narrowUpperPanel = await narrowPanels.nth(0).boundingBox();
    const narrowLowerPanel = await narrowPanels.nth(1).boundingBox();
    expect(narrowUpperPanel).not.toBeNull();
    expect(narrowLowerPanel).not.toBeNull();
    await expectRendererCanvasAtPoint(
      page,
      195,
      Math.floor(
        ((narrowUpperPanel?.y ?? 0) +
          (narrowUpperPanel?.height ?? 0) +
          (narrowLowerPanel?.y ?? 0)) /
          2,
      ),
    );
    const narrowForward = page.getByRole("button", { name: "↑ Forward" });
    await narrowForward.focus();
    await expect(narrowForward).toBeFocused();
    await testInfo.attach("engine-dungeon-corridor-mobile.png", {
      body: await page.locator("aui-game-viewport").screenshot(),
      contentType: "image/png",
    });
    await page.setViewportSize({ width: 1280, height: 720 });
    for (const facing of ["north", "west", "south", "east"]) {
      await page.getByRole("button", { name: "↶ Left" }).click();
      const rotatedViewport = page.getByRole("img", {
        name: new RegExp(`Warden's Gate Pass, facing ${facing} at cell 1, 1`),
      });
      await expect(rotatedViewport).toBeVisible();
      await expect(page.locator("aui-game-viewport canvas")).toBeVisible();
      if (facing === "north" || facing === "south") {
        await testInfo.attach(`engine-dungeon-facing-${facing}.png`, {
          body: await rotatedViewport.screenshot(),
          contentType: "image/png",
        });
      }
    }
    for (let step = 0; step < 4; step += 1) {
      await page.getByRole("button", { name: "↑ Forward" }).click();
    }
    const movedDungeonViewport = page.locator("aui-game-viewport");
    await expect(movedDungeonViewport.locator("canvas")).toBeVisible();
    await expect(movedDungeonViewport.getByRole("alert")).toHaveCount(0);
    await expect(
      page.getByRole("heading", { name: "Silent murder holes" }),
    ).toBeVisible();
    await page.getByRole("button", { name: "Inspect" }).click();
    await expect(page.getByText("Inspected", { exact: true })).toBeVisible();
    await testInfo.attach("first-person-dungeon-exploration.png", {
      body: await page.screenshot({ fullPage: true }),
      contentType: "image/png",
    });
    for (let step = 0; step < 4; step += 1) {
      await page.getByRole("button", { name: "↑ Forward" }).click();
    }

    await expect(page.locator("aui-game-viewport canvas")).toHaveCount(1);
    await expect(
      page.locator("aui-game-viewport [data-scene-mode]"),
    ).toHaveAttribute("data-scene-mode", "encounter");
    await expect(page.locator("aui-game-viewport canvas")).toHaveAttribute(
      "data-lifecycle-witness",
      "persistent",
    );
    await expect(page.locator("aui-character-status")).toHaveCount(6);
    await expect(page.getByText("Mara Venn", { exact: true })).toBeVisible();
    await expect(
      page
        .locator("aui-character-status")
        .filter({ hasText: "Iron Warden" })
        .getByText("Iron Warden", { exact: true }),
    ).toBeVisible();
    await expect(page.getByLabel("Encounter identity")).toContainText(
      "Engine fb608e323a8b",
    );
    await expect(
      page.getByRole("button", { name: "Pin In Place" }),
    ).toBeVisible();
    await expect(page.getByRole("button", { name: "Disrupt" })).toBeVisible();
    const tacticalBoard = page.getByRole("application", {
      name: /Rendered tactical combat board/,
    });
    await expect(tacticalBoard).toBeVisible();
    await expect(page.locator("aui-tactical-board")).toHaveCount(0);
    await expect(page.getByLabel("Combat actions")).toBeVisible();
    await expect(page.getByLabel("Combat log")).toBeVisible();
    await testInfo.attach("renderer-root-encounter-desktop.png", {
      body: await page.screenshot(),
      contentType: "image/png",
    });
    await page.setViewportSize({ width: 390, height: 844 });
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth,
      ),
    ).toBe(true);
    await expect(tacticalBoard).toBeVisible();
    await expect(page.getByLabel("Combat actions")).toBeVisible();
    await expect(page.getByLabel("Combat log")).toBeVisible();
    await testInfo.attach("renderer-root-encounter-mobile.png", {
      body: await page.screenshot(),
      contentType: "image/png",
    });
    await page.setViewportSize({ width: 1280, height: 720 });
    await clickRenderedTacticalCell(page, 7, 4, 12, 8);
    await expect(tacticalBoard).toHaveAttribute(
      "data-last-pick-identity",
      "entity:101",
    );
    await tacticalBoard.focus();
    await page.keyboard.press("ArrowUp");
    await expect(page.getByText(/Move to 7, 3, cost 1/)).toBeVisible();
    await page.keyboard.press("Enter");
    const afterMovement = (await (
      await request.get("/api/v1/session")
    ).json()) as {
      encounter: {
        participants: Array<{
          character: { id: number };
          x: number;
          y: number;
        }>;
        log: Array<{ details: string[] }>;
      };
    };
    expect(
      afterMovement.encounter.participants.find(
        (participant) => participant.character.id === 101,
      ),
    ).toMatchObject({ x: 7, y: 3 });
    expect(
      afterMovement.encounter.log.some((entry) =>
        entry.details.some((detail) =>
          detail.includes("Engine pathfinding admitted a 1-square route"),
        ),
      ),
    ).toBe(true);
    const translatedStrike = page
      .locator(".action-note")
      .filter({ hasText: "Longsword Strike" });
    await expect(translatedStrike).toContainText("Might vs Armor");
    await expect(translatedStrike).toContainText("1 Standard Action");
    await expect(translatedStrike).toContainText("range 1");
    await expect(translatedStrike).toContainText("Training Blade");

    await page.getByLabel("Target").selectOption({ label: "Iron Warden" });
    await page.getByRole("button", { name: "Longsword Strike" }).click();
    await expect(page.getByLabel("Available reaction")).toHaveCount(0);
    await expect(
      page.getByRole("button", { name: "Resolve deterministic roll" }),
    ).toHaveCount(0);
    const afterPartyAction = (await (
      await request.get("/api/v1/session")
    ).json()) as {
      encounter: {
        log: Array<{ details: string[] }>;
      };
    };
    expect(
      afterPartyAction.encounter.log.some((entry) =>
        entry.details.some((detail) =>
          detail.includes("Roll-source position 0"),
        ),
      ),
    ).toBe(true);

    await page
      .getByRole("button", { name: "Begin Gate Skirmisher turn" })
      .first()
      .click();
    const reaction = page.getByRole("region", {
      name: "Available reaction",
      exact: true,
    });
    if (await reaction.isVisible()) {
      await page.getByRole("button", { name: "Do not react" }).click();
    }
    await expect(page.getByLabel("Encounter identity")).toContainText(
      "Round 0",
    );
    await expect(page.getByLabel("Encounter identity")).toContainText(
      "Ilyra Fen acting",
    );
    await page.getByRole("button", { name: "Save", exact: true }).click();
    await expect(page.getByText("Saved", { exact: true })).toBeVisible();
  });

  test("a normal control presents a stale rejection after another client advances state", async ({
    browser,
    page,
  }) => {
    await page.goto("/");
    const second = await browser.newPage();
    await second.goto("/");
    await continueIfNeeded(page);
    await continueIfNeeded(second);

    await second
      .getByRole("button", { name: "End Ilyra Fen activation" })
      .click();
    await expect(second.getByLabel("Encounter identity")).toContainText(
      "Iron Warden acting",
    );

    await page
      .getByRole("button", { name: "End Ilyra Fen activation" })
      .click();
    const alert = page.getByRole("alert");
    await expect(alert).toContainText("stale rejection");
    await expect(alert).toContainText("current revision");
    await expect(
      page.getByRole("button", { name: "Reload current state" }),
    ).toBeVisible();
    await second.close();
  });

  test("mobile game shell remains usable without horizontal overflow", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("/");
    await continueIfNeeded(page);
    await expect(
      page.getByRole("button", { name: "Begin Iron Warden turn" }).first(),
    ).toBeVisible();
    await expect(page.locator("aui-character-status")).toHaveCount(6);
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth,
      ),
    ).toBe(true);
  });

  test("runtime connection failure is visibly classified and retryable", async ({
    page,
  }) => {
    await page.route("**/api/v1/session", (route) =>
      route.abort("connectionrefused"),
    );
    await page.goto("/");

    const alert = page.getByRole("alert");
    await expect(alert).toContainText("network failure");
    await expect(
      page.getByRole("button", { name: "Retry connection" }),
    ).toBeVisible();
  });

  test("invalid runtime payload fails closed at the protocol border", async ({
    page,
  }) => {
    await page.route("**/api/v1/session", (route) =>
      route.fulfill({
        contentType: "application/json",
        status: 200,
        body: '{"product":"Rusty D20"}',
      }),
    );
    await page.goto("/");

    await expect(page.getByRole("alert")).toContainText("unknown failure");
    await expect(
      page.getByText("Game snapshot has an unexpected or invalid shape."),
    ).toBeVisible();
  });
});

async function continueIfNeeded(page: Page): Promise<void> {
  const button = page.getByRole("button", { name: "Continue Adventure" });
  await expect(button).toBeVisible();
  await button.click();
}
