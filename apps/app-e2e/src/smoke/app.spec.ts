import { expect, test, type Page } from "@playwright/test";


async function expectCombatOverlayLayout(
  page: Page,
  minimumControlHeight = 0,
): Promise<void> {
  const layout = await page.evaluate(() => {
    const action = document.querySelector(".combat-action-panel");
    const log = document.querySelector(".combat-log-panel");
    const status = document.querySelector(".combat-status");
    if (
      !(action instanceof HTMLElement) ||
      !(log instanceof HTMLElement) ||
      !(status instanceof HTMLElement)
    ) {
      throw new Error("combat overlay panels are missing");
    }
    const actionBox = action.getBoundingClientRect();
    const logBox = log.getBoundingClientRect();
    const statusBox = status.getBoundingClientRect();
    const controls = Array.from(action.querySelectorAll("button"), (control) =>
      control.getBoundingClientRect(),
    );
    return {
      action: {
        bottom: actionBox.bottom,
        left: actionBox.left,
        right: actionBox.right,
        top: actionBox.top,
      },
      controls: controls.map((control) => ({
        height: control.height,
        left: control.left,
        right: control.right,
      })),
      log: {
        bottom: logBox.bottom,
        clientWidth: log.clientWidth,
        left: logBox.left,
        right: logBox.right,
        scrollWidth: log.scrollWidth,
        top: logBox.top,
      },
      status: {
        bottom: statusBox.bottom,
        top: statusBox.top,
      },
      viewportHeight: window.innerHeight,
      viewportWidth: window.innerWidth,
    };
  });

  expect(layout.action.left).toBeGreaterThanOrEqual(0);
  expect(layout.action.right).toBeLessThanOrEqual(layout.log.left);
  expect(layout.log.right).toBeLessThanOrEqual(layout.viewportWidth);
  expect(layout.action.bottom, JSON.stringify(layout)).toBeCloseTo(
    layout.log.bottom,
    0,
  );
  expect(layout.action.bottom).toBeGreaterThan(layout.viewportHeight * 0.9);
  expect(layout.status.bottom).toBeLessThanOrEqual(layout.action.top);
  expect(layout.log.scrollWidth).toBeLessThanOrEqual(layout.log.clientWidth);
  expect(
    layout.controls.every(
      (control) =>
        control.height >= minimumControlHeight &&
        control.left >= layout.action.left &&
        control.right <= layout.action.right,
    ),
  ).toBe(true);
}

async function expectCombatLogAtBottom(page: Page): Promise<void> {
  const entries = page.locator("aui-combat-log .entries");
  await expect(entries).toBeVisible();
  await expect
    .poll(() =>
      entries.evaluate(
        (element) =>
          Math.abs(
            element.scrollHeight - element.clientHeight - element.scrollTop,
          ) <= 1,
      ),
    )
    .toBe(true);
}

async function clickRenderedTacticalCell(
  page: Page,
  x: number,
  y: number,
  boardWidth: number,
  boardHeight: number,
): Promise<void> {
  const point = await renderedTacticalCellPoint(
    page,
    x,
    y,
    boardWidth,
    boardHeight,
  );
  await page.mouse.click(point.x, point.y);
}

async function renderedTacticalCellPoint(
  page: Page,
  x: number,
  y: number,
  boardWidth: number,
  boardHeight: number,
): Promise<{ readonly x: number; readonly y: number }> {
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
  return point;
}

async function exposeRendererAtPoint(
  page: Page,
  point: { readonly x: number; readonly y: number },
): Promise<void> {
  const hit = await page.evaluate(async ({ x, y }) => {
    const overlay = document.querySelector<HTMLElement>(".game-overlay");
    if (overlay === null) {
      return {
        className: null,
        clientHeight: null,
        scrollHeight: null,
        tagName: null,
        scrollTop: null,
      };
    }
    const maximum = Math.max(0, overlay.scrollHeight - overlay.clientHeight);
    const stopCount = Math.max(2, Math.ceil(maximum / 80) + 1);
    const stops = Array.from(
      { length: stopCount },
      (_, index) => (maximum * index) / (stopCount - 1),
    );
    for (const scrollTop of stops) {
      overlay.scrollTop = scrollTop;
      await new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
      );
      const target = document.elementFromPoint(x, y);
      if (target instanceof HTMLElement && target.dataset["rendererBoundary"] !== undefined) {
        return {
          className: target.className,
          clientHeight: overlay.clientHeight,
          scrollHeight: overlay.scrollHeight,
          tagName: target.tagName,
          scrollTop: overlay.scrollTop,
        };
      }
    }
    const target = document.elementFromPoint(x, y);
    return {
      className:
        target instanceof HTMLElement ? target.className : target?.nodeName,
      clientHeight: overlay.clientHeight,
      scrollHeight: overlay.scrollHeight,
      tagName: target?.tagName ?? null,
      scrollTop: overlay.scrollTop,
    };
  }, point);
  expect(
    hit.tagName,
    `native renderer boundary occluded by ${hit.className} at overlay scroll ${hit.scrollTop}; ${hit.clientHeight}/${hit.scrollHeight}`,
  ).toBe("SECTION");
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
    browser,
    page,
    request,
  }, testInfo) => {
    test.setTimeout(120_000);
    const health = await request.get("/healthz");
    expect(health.ok()).toBe(true);
    await expect(health.json()).resolves.toEqual({
      status: "ok",
      version: "0.1.0",
    });

    const initialSession = (await (
      await request.get("/api/v1/session")
    ).json()) as {
      campaign: { id: string } | null;
    };
    if (initialSession.campaign !== null) {
      const saveStatus = (await (
        await request.get("/api/v1/session/save-status")
      ).json()) as {
        campaignId: string | null;
        revision: number | null;
        saveIdentity: string;
      };
      const reset = await request.post("/api/v1/session/reset", {
        data: {
          expectedAdventureId: saveStatus.campaignId,
          expectedRevision: saveStatus.revision,
          expectedSaveIdentity: saveStatus.saveIdentity,
        },
      });
      expect(reset.ok()).toBe(true);
    }

    await page.goto("/");
    const nativeBoundary = page.locator("aui-game-viewport [data-renderer-boundary]");
    await expect(nativeBoundary).toHaveCount(1);
    await expect(
      page.locator("aui-game-viewport [data-scene-mode]"),
    ).toHaveAttribute("data-scene-mode", "catalog");
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
    await expect(nativeBoundary).toHaveCount(1);
    await expect(
      page.locator("aui-game-viewport [data-scene-mode]"),
    ).toHaveAttribute("data-scene-mode", "camp");
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
    const dungeonViewport = page.locator(
      "aui-game-viewport [data-scene-mode='exploration']",
    );
    await expect(dungeonViewport).toBeVisible();
    await expect(dungeonViewport).toHaveAttribute(
      "aria-label",
      /Warden's Gate Pass, facing east at cell 1, 1/,
    );
    await expect(dungeonViewport).toHaveAttribute(
      "data-renderer-boundary",
      "native-engine-host",
    );

    const partyTrigger = page.getByRole("button", { name: "Party" });
    const beforePartyInspection = await (
      await request.get("/api/v1/session")
    ).json();
    await partyTrigger.click();
    const partySheet = page.getByRole("dialog", { name: "Party" });
    await expect(partySheet).toBeVisible();
    expect(
      await partySheet.evaluate((dialog) => dialog.matches(":modal")),
    ).toBe(true);
    await expect(
      partySheet.getByRole("tab", { name: "Mara Venn" }),
    ).toBeFocused();
    const maraSheet = partySheet.getByRole("tabpanel", {
      name: "Mara Venn",
    });
    await expect(maraSheet).toContainText("Level 1 · 900 XP");
    await expect(
      maraSheet.getByRole("region", { name: "Abilities" }),
    ).toContainText("Might 18 (+4)");
    await expect(
      maraSheet.getByRole("region", { name: "Defenses" }),
    ).toContainText("Armor 18");
    await expect(
      maraSheet.getByRole("region", { name: "Features and feats" }),
    ).toContainText("Hold the Line");
    await expect(
      maraSheet.getByRole("region", { name: "Features and feats" }),
    ).toContainText("controlling a threatened position");
    await expect(
      maraSheet.getByRole("region", { name: "Actions", exact: true }),
    ).toContainText("Longsword Strike");
    await expect(
      maraSheet.getByRole("region", { name: "Reactions" }),
    ).toContainText("Parry");
    await expect(
      maraSheet.getByRole("region", { name: "Current loadout" }),
    ).toContainText("Mara's chain armor");
    await testInfo.attach("exploration-party-sheet-desktop.png", {
      body: await page.screenshot({ fullPage: true }),
      contentType: "image/png",
    });

    await page.keyboard.press("w");
    await expect(
      (await request.get("/api/v1/session")).json(),
    ).resolves.toEqual(beforePartyInspection);
    const maraTab = partySheet.getByRole("tab", { name: "Mara Venn" });
    await maraTab.press("ArrowRight");
    const ilyraTab = partySheet.getByRole("tab", { name: "Ilyra Fen" });
    await expect(ilyraTab).toBeFocused();
    await expect(ilyraTab).toHaveAttribute("aria-selected", "true");
    const ilyraSheet = partySheet.getByRole("tabpanel", {
      name: "Ilyra Fen",
    });
    await expect(ilyraSheet).toContainText("Level 1 · 760 XP");
    await expect(
      ilyraSheet.getByRole("region", { name: "Features and feats" }),
    ).toContainText("Pathfinder Instinct");

    await page.setViewportSize({ width: 390, height: 844 });
    const partySheetBox = await partySheet.boundingBox();
    expect(partySheetBox).not.toBeNull();
    expect(partySheetBox?.x ?? -1).toBeGreaterThanOrEqual(0);
    expect(partySheetBox?.y ?? -1).toBeGreaterThanOrEqual(0);
    expect(
      (partySheetBox?.x ?? 0) + (partySheetBox?.width ?? 0),
    ).toBeLessThanOrEqual(390);
    expect(
      (partySheetBox?.y ?? 0) + (partySheetBox?.height ?? 0),
    ).toBeLessThanOrEqual(844);
    expect(
      await partySheet.evaluate(
        (dialog) => dialog.scrollWidth <= dialog.clientWidth,
      ),
    ).toBe(true);
    await testInfo.attach("exploration-party-sheet-mobile.png", {
      body: await page.screenshot(),
      contentType: "image/png",
    });
    await partySheet.getByRole("tab", { name: "Mara Venn" }).click();
    await page.keyboard.press("Escape");
    await expect(partySheet).toHaveCount(0);
    await expect(partyTrigger).toBeFocused();

    await page.setViewportSize({ width: 1280, height: 720 });
    await page.getByRole("button", { name: "Save" }).click();
    await expect(page.getByText("Saved", { exact: true })).toBeVisible();
    const reopenedPage = await page.context().newPage();
    await reopenedPage.goto("/");
    await continueIfNeeded(reopenedPage);
    await reopenedPage.getByRole("button", { name: "Party" }).click();
    const reopenedPartySheet = reopenedPage.getByRole("dialog", {
      name: "Party",
    });
    await expect(reopenedPartySheet).toBeVisible();
    await reopenedPartySheet
      .getByRole("tab", { name: "Ilyra Fen" })
      .click();
    await expect(
      reopenedPartySheet.getByRole("tabpanel", { name: "Ilyra Fen" }),
    ).toContainText("Level 1 · 760 XP");
    await reopenedPartySheet
      .getByRole("tab", { name: "Mara Venn" })
      .click();
    await reopenedPartySheet.getByRole("button", { name: "Close" }).click();
    await expect(reopenedPartySheet).toHaveCount(0);
    await reopenedPage.close();

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
    const rejectedStep = await request.post(
      "/api/v1/session/exploration/command",
      {
        data: {
          expectedRevision: beforeExplorationInventory.revision,
          command: "step-backward",
        },
      },
    );
    expect(rejectedStep.status()).toBe(422);
    await expect(rejectedStep.json()).resolves.toMatchObject({
      kind: "invalid",
      message: "solid dungeon stone or a closed door blocks that step",
    });
    await expect(dungeonViewport).toHaveAttribute(
      "aria-label",
      /facing east at cell 1, 1/,
    );
    await expect(
      (await request.get("/api/v1/session")).json(),
    ).resolves.toEqual(beforeExplorationInventory);
    await page.emulateMedia({ reducedMotion: "reduce" });
    await page.getByRole("button", { name: "↶ Left" }).click();
    await expect(dungeonViewport).toHaveAttribute(
      "aria-label",
      /facing north at cell 1, 1/,
    );
    await page.getByRole("button", { name: "Right ↷" }).click();
    await expect(dungeonViewport).toHaveAttribute(
      "aria-label",
      /facing east at cell 1, 1/,
    );
    await page.emulateMedia({ reducedMotion: "no-preference" });

    await testInfo.attach("engine-dungeon-corridor.png", {
      body: await dungeonViewport.screenshot(),
      contentType: "image/png",
    });
    await page.setViewportSize({ width: 390, height: 844 });
    await expect(nativeBoundary).toBeVisible();
    expect(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth,
      ),
    ).toBe(true);
    await page.locator(".game-overlay").evaluate(async (overlay) => {
      overlay.scrollTop = 0;
      await new Promise<void>((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
      );
    });
    const narrowForward = page.getByRole("button", { name: "↑ Forward" });
    await narrowForward.focus();
    await expect(narrowForward).toBeFocused();
    await page.getByRole("button", { name: "↶ Left" }).click();
    await expect(dungeonViewport).toHaveAttribute(
      "aria-label",
      /facing north at cell 1, 1/,
    );
    await page.getByRole("button", { name: "Right ↷" }).click();
    await expect(dungeonViewport).toHaveAttribute(
      "aria-label",
      /facing east at cell 1, 1/,
    );
    await testInfo.attach("engine-dungeon-corridor-mobile.png", {
      body: await page.locator("aui-game-viewport").screenshot(),
      contentType: "image/png",
    });
    await page.setViewportSize({ width: 1280, height: 720 });

    await page.getByRole("button", { name: "↶ Left" }).click();
    await expect(dungeonViewport).toHaveAttribute(
      "aria-label",
      /facing north at cell 1, 1/,
    );
    await page.getByRole("button", { name: "Right ↷" }).click();
    await expect(dungeonViewport).toHaveAttribute(
      "aria-label",
      /facing east at cell 1, 1/,
    );

    for (const facing of ["north", "west", "south", "east"]) {
      await page.getByRole("button", { name: "↶ Left" }).click();
      const rotatedViewport = page.getByRole("img", {
        name: new RegExp(`Warden's Gate Pass, facing ${facing} at cell 1, 1`),
      });
      await expect(rotatedViewport).toBeVisible();
      if (facing === "north" || facing === "south") {
        await testInfo.attach(`engine-dungeon-facing-${facing}.png`, {
          body: await rotatedViewport.screenshot(),
          contentType: "image/png",
        });
      }
    }
    for (let step = 0; step < 4; step += 1) {
      await page.getByRole("button", { name: "↑ Forward" }).click();
      await expect(dungeonViewport).toHaveAttribute(
        "aria-label",
        new RegExp(`facing east at cell ${String(step + 2)}, 1`),
      );
    }
    const movedDungeonViewport = page.locator("aui-game-viewport");
    await expect(movedDungeonViewport.locator("[data-renderer-boundary]")).toBeVisible();
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

    await expect(nativeBoundary).toHaveCount(1);
    await expect(
      page.locator("aui-game-viewport [data-scene-mode]"),
    ).toHaveAttribute("data-scene-mode", "encounter");
    await expect(page.locator("aui-character-status")).toHaveCount(6);
    await expect(page.getByText("Mara Venn", { exact: true })).toBeVisible();
    await expect(
      page
        .locator("aui-character-status")
        .filter({ hasText: "Iron Warden" })
        .getByText("Iron Warden", { exact: true }),
    ).toBeVisible();
    await expect(page.getByLabel("Encounter identity")).toContainText(
      "Engine d0b5e672b83d",
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
    await expectCombatOverlayLayout(page);
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
    await expectCombatOverlayLayout(page, 44);
    await page.getByRole("button", { name: "Move", exact: true }).click();
    await expect(tacticalBoard).toHaveAttribute(
      "data-interaction-mode",
      "movement",
    );
    await clickRenderedTacticalCell(page, 7, 3, 12, 8);
    await expect(page.locator(".targeting-status")).toContainText(
      "Route previewed to 7, 3 at cost 1",
    );
    await testInfo.attach("movement-preview-mobile.png", {
      body: await page.screenshot(),
      contentType: "image/png",
    });
    await page
      .getByRole("status")
      .getByRole("button", { name: "Cancel movement" })
      .click();
    await testInfo.attach("renderer-root-encounter-mobile.png", {
      body: await page.screenshot(),
      contentType: "image/png",
    });
    await page.setViewportSize({ width: 1280, height: 720 });
    await clickRenderedTacticalCell(page, 7, 3, 12, 8);
    await expect(tacticalBoard).toHaveAttribute(
      "data-last-pick-identity",
      "cell:7:3",
    );
    await expect(page.locator(".targeting-status")).toContainText(
      "Choose Move from the hotbar",
    );
    const beforeMovement = (await (
      await request.get("/api/v1/session")
    ).json()) as {
      encounter: {
        participants: Array<{
          character: { id: number };
          x: number;
          y: number;
        }>;
      };
    };
    expect(
      beforeMovement.encounter.participants.find(
        (participant) => participant.character.id === 101,
      ),
    ).toMatchObject({ x: 7, y: 4 });

    await page.getByRole("button", { name: "Move", exact: true }).click();
    await expect(
      page.getByRole("button", { name: "Move", exact: true }),
    ).toHaveAttribute("aria-pressed", "true");
    await expect(
      page.getByRole("navigation", {
        name: "Legal movement destinations",
      }),
    ).toContainText("Preview move to 7, 3, cost 1");
    await page
      .getByRole("status")
      .getByRole("button", { name: "Cancel movement" })
      .click();
    await expect(tacticalBoard).toHaveAttribute(
      "data-interaction-mode",
      "readonly",
    );

    await page.getByRole("button", { name: "Move", exact: true }).click();
    await clickRenderedTacticalCell(page, 7, 3, 12, 8);
    await expect(page.locator(".targeting-status")).toContainText(
      "Route previewed to 7, 3 at cost 1",
    );
    const previewedMovement = (await (
      await request.get("/api/v1/session")
    ).json()) as typeof beforeMovement;
    expect(
      previewedMovement.encounter.participants.find(
        (participant) => participant.character.id === 101,
      ),
    ).toMatchObject({ x: 7, y: 4 });
    await testInfo.attach("movement-preview-desktop.png", {
      body: await page.screenshot(),
      contentType: "image/png",
    });
    await clickRenderedTacticalCell(page, 7, 3, 12, 8);
    await expect
      .poll(async () => {
        const snapshot = (await (
          await request.get("/api/v1/session")
        ).json()) as {
          encounter: {
            participants: Array<{
              character: { id: number };
              x: number;
              y: number;
            }>;
          };
        };
        const mara = snapshot.encounter.participants.find(
          (participant) => participant.character.id === 101,
        );
        return mara === undefined ? null : { x: mara.x, y: mara.y };
      })
      .toEqual({ x: 7, y: 3 });
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

    await page.setViewportSize({ width: 1280, height: 1000 });
    await expect(page.getByLabel("Target")).toHaveCount(0);
    let failedActionRequests = 0;
    await page.route("**/api/v1/session/action", async (route) => {
      failedActionRequests += 1;
      await route.fulfill({
        contentType: "application/json",
        status: 409,
        body: JSON.stringify({
          kind: "stale",
          message: "synthetic stale targeting proof",
          retryable: true,
        }),
      });
    });
    await page.getByRole("button", { name: "Longsword Strike" }).click();
    await expect(
      page.getByRole("button", { name: "Longsword Strike" }),
    ).toHaveAttribute("aria-pressed", "true");
    await expect(tacticalBoard).toHaveAttribute(
      "data-interaction-mode",
      "targeting",
    );
    await expect(tacticalBoard).toHaveAttribute(
      "aria-label",
      /Targeting Longsword Strike/,
    );
    await expect(
      page.getByRole("navigation", {
        name: "Legal targets for Longsword Strike",
      }),
    ).toContainText("Iron Warden");
    await testInfo.attach("action-first-targeting-desktop.png", {
      body: await page.screenshot(),
      contentType: "image/png",
    });
    expect(failedActionRequests).toBe(0);
    await clickRenderedTacticalCell(page, 8, 4, 12, 8);
    await expect.poll(() => failedActionRequests).toBe(1);
    await expect(page.getByRole("alert")).toContainText("stale rejection");
    await expect(tacticalBoard).toHaveAttribute(
      "data-interaction-mode",
      "readonly",
    );
    await page.getByRole("button", { name: "Dismiss" }).click();
    await page.unroute("**/api/v1/session/action");

    const mobileContext = await browser.newContext({
      hasTouch: true,
      isMobile: true,
      viewport: { width: 390, height: 844 },
    });
    const mobilePage = await mobileContext.newPage();
    await mobilePage.goto("/");
    await continueIfNeeded(mobilePage);
    const mobileLogEntries = mobilePage.locator("aui-combat-log .entries");
    await expect(mobileLogEntries).toBeVisible();
    await mobileLogEntries.evaluate((element) => {
      element.scrollTop = 0;
    });
    let releaseAction: (() => void) | undefined;
    const actionRelease = new Promise<void>((resolve) => {
      releaseAction = resolve;
    });
    let markActionStarted: (() => void) | undefined;
    const actionStarted = new Promise<void>((resolve) => {
      markActionStarted = resolve;
    });
    let mobileActionRequests = 0;
    await mobilePage.route("**/api/v1/session/action", async (route) => {
      mobileActionRequests += 1;
      markActionStarted?.();
      await actionRelease;
      await route.continue();
    });
    await mobilePage.getByRole("button", { name: "Longsword Strike" }).click();
    const mobileBoard = mobilePage.getByRole("application", {
      name: /Rendered tactical combat board/,
    });
    await expect(mobileBoard).toHaveAttribute(
      "data-interaction-mode",
      "targeting",
    );
    await testInfo.attach("action-first-targeting-mobile-touch.png", {
      body: await mobilePage.screenshot(),
      contentType: "image/png",
    });
    const touchTarget = await renderedTacticalCellPoint(
      mobilePage,
      8,
      4,
      12,
      8,
    );
    await exposeRendererAtPoint(mobilePage, touchTarget);
    await mobilePage.touchscreen.tap(touchTarget.x, touchTarget.y);
    await actionStarted;
    await mobilePage.touchscreen.tap(touchTarget.x, touchTarget.y);
    expect(mobileActionRequests).toBe(1);
    releaseAction?.();
    const mobileReaction = mobilePage.getByRole("region", {
      name: "Available reaction",
      exact: true,
    });
    const mobileNextParty = mobilePage
      .getByLabel("Encounter identity")
      .getByText("Ilyra Fen acting", { exact: true });
    await expect(mobileReaction.or(mobileNextParty)).toBeVisible();
    if (await mobileReaction.isVisible()) {
      await mobilePage.getByRole("button", { name: "Do not react" }).click();
    }
    await expect(mobileNextParty).toBeVisible();
    await expectCombatLogAtBottom(mobilePage);
    const mobileReceiptEntry = mobilePage
      .locator("aui-combat-log .entry")
      .filter({ hasText: "Roll-source position 0." });
    const mobileReceiptDetails = mobileReceiptEntry.locator(".entry__details");
    await mobileReceiptEntry.locator(".entry__summary").tap();
    await expect(mobileReceiptDetails).toBeVisible();
    await expect(mobileReceiptDetails).toBeInViewport();
    await expect(mobileReceiptDetails).toContainText(/d20 \d+ \+ modifier/);
    await expect(mobileReceiptDetails).toContainText("against defense");
    await expect(mobileReceiptDetails).toContainText("Roll-source position 0.");
    const mobileReceiptBox = await mobileReceiptDetails.boundingBox();
    expect(mobileReceiptBox).not.toBeNull();
    expect(mobileReceiptBox?.x ?? -1).toBeGreaterThanOrEqual(0);
    expect(
      (mobileReceiptBox?.x ?? 0) + (mobileReceiptBox?.width ?? 0),
    ).toBeLessThanOrEqual(390);
    await testInfo.attach("rules-log-receipt-mobile-touch.png", {
      body: await mobilePage.screenshot(),
      contentType: "image/png",
    });
    await expect(
      mobilePage.getByRole("button", { name: /^Begin .+ turn$/ }),
    ).toHaveCount(0);
    await mobilePage.unroute("**/api/v1/session/action");
    await mobileContext.close();

    await page.reload();
    await continueIfNeeded(page);
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
    await expectCombatLogAtBottom(page);
    const receiptEntry = page
      .locator("aui-combat-log .entry")
      .filter({ hasText: "Roll-source position 0." });
    const receiptSummary = receiptEntry.locator(".entry__summary");
    const receiptDetails = receiptEntry.locator(".entry__details");
    await receiptSummary.hover();
    await expect(receiptDetails).toBeVisible();
    await expect(receiptDetails).toContainText(/d20 \d+ \+ modifier/);
    await expect(receiptDetails).toContainText("against defense");
    await expect(receiptDetails).toContainText("Roll-source position 0.");
    await receiptSummary.focus();
    await expect(receiptSummary).toBeFocused();
    await expect(receiptDetails).toBeVisible();
    const desktopReceiptBox = await receiptDetails.boundingBox();
    expect(desktopReceiptBox).not.toBeNull();
    expect(desktopReceiptBox?.x ?? -1).toBeGreaterThanOrEqual(0);
    expect(
      (desktopReceiptBox?.x ?? 0) + (desktopReceiptBox?.width ?? 0),
    ).toBeLessThanOrEqual(1280);
    await testInfo.attach("rules-log-receipt-desktop-focus.png", {
      body: await page.screenshot(),
      contentType: "image/png",
    });

    await expect(
      page.getByRole("button", { name: /^Begin .+ turn$/ }),
    ).toHaveCount(0);
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
    await expect(second.getByLabel("Encounter identity")).not.toContainText(
      "Ilyra Fen acting",
    );
    await expect(
      second.getByRole("button", { name: /^Begin .+ turn$/ }),
    ).toHaveCount(0);

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
      page.getByRole("button", { name: /^Begin .+ turn$/ }),
    ).toHaveCount(0);
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
