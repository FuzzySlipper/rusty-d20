import { expect, test, type Page } from "@playwright/test";

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
    await expect(page.getByLabel("Armor defense readout")).toContainText("18");
    await expect(
      page.getByRole("region", { name: "Inventory", exact: true }),
    ).toBeVisible();
    await expect(page.getByLabel("Equipment")).toBeVisible();
    await expect(page.getByLabel("Equipment")).toContainText(
      "Mara's training blade",
    );
    await expect(page.getByLabel("Equipment")).toContainText(
      "Mara's field bow",
    );
    await expect(page.getByLabel("Camp stash")).toContainText("Spare buckler");

    await page.getByRole("button", { name: "Take" }).click();
    await expect(page.getByRole("alert")).toContainText("capacity rejection");
    await expect(page.getByRole("alert")).toContainText("maximum: 4");
    await testInfo.attach("capacity-rejection.png", {
      body: await page.screenshot({ fullPage: true }),
      contentType: "image/png",
    });
    await page.getByRole("button", { name: "Dismiss" }).click();
    await expect(page.getByLabel("Armor defense readout")).toContainText("18");
    await expect(page.getByText("Carried 4/4")).toBeVisible();

    const chainInventory = page.getByRole("button", {
      name: /Mara's chain armor · equipped body/,
    });
    await chainInventory.focus();
    await chainInventory.press("Enter");
    await expect(page.getByLabel("Armor defense readout")).toContainText("16");
    const unequippedChain = page.getByRole("button", {
      name: "Mara's chain armor",
    });
    await unequippedChain.focus();
    await unequippedChain.press("Space");
    await expect(page.getByLabel("Armor defense readout")).toContainText("18");

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
    await expect(page.getByLabel("Equipment")).toContainText(
      "Ilyra's chain armor",
    );
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
    await expect(
      page.getByRole("img", {
        name: /Warden's Gate Pass, facing east at cell 1, 1/,
      }),
    ).toBeVisible();
    for (let step = 0; step < 4; step += 1) {
      await page.getByRole("button", { name: "↑ Forward" }).click();
    }
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
    const tacticalBoard = page.getByRole("region", {
      name: "Authoritative tactical combat board",
    });
    await expect(tacticalBoard.getByRole("gridcell")).toHaveCount(96);
    await expect(
      tacticalBoard.getByRole("gridcell", {
        name: /Mara Venn, party, at 7, 4, acting/,
      }),
    ).toBeVisible();
    await tacticalBoard
      .getByRole("gridcell", { name: "Move to 7, 3, cost 1" })
      .click();
    await expect(
      tacticalBoard.getByRole("gridcell", {
        name: /Mara Venn, party, at 7, 3, acting/,
      }),
    ).toBeVisible();
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
  if (await button.isVisible()) {
    await button.click();
  }
}
