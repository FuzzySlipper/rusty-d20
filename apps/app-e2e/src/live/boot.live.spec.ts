import { expect, liveScenario } from "./support/live-scenario";

liveScenario(
  "Rust-owned authored encounter live evidence @live",
  async ({ page, collector, liveBaseUrl }) => {
    collector.addNonClaim(
      "This live scenario certifies the Warden path landing, Engine-backed camp loadout, Rust-owned first-person grid traversal, an authored landmark, encounter activation at its dungeon trigger, and one complete player/opposition round. Tactical overhead combat remains a later milestone.",
    );

    await page.goto(liveBaseUrl);
    const newWardensGate = page.getByRole("button", {
      name: "New Adventure · The Warden's Gate",
      exact: true,
    });
    if (await newWardensGate.isVisible()) {
      await collector.milestone("empty game ready", { screenshot: true });
      await newWardensGate.click();
      await expect(
        page.getByRole("heading", { name: "The Warden's Gate Camp" }),
      ).toBeVisible();
      await expect(page.getByLabel("Armor defense readout")).toContainText(
        "16",
      );
      await collector.milestone("durable adventure camp loadout", {
        screenshot: true,
        layerSnapshot: {
          inventory: await page
            .getByRole("region", { name: "Inventory", exact: true })
            .innerText(),
          equipment: await page.getByLabel("Equipment").innerText(),
        },
      });
      await page.setViewportSize({ width: 390, height: 844 });
      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= window.innerWidth,
        ),
      ).toBe(true);
      await collector.milestone("mobile camp loadout", { screenshot: true });
      await page.setViewportSize({ width: 1280, height: 720 });
    } else if (
      await page.getByRole("button", { name: "Continue Adventure" }).isVisible()
    ) {
      await page.getByRole("button", { name: "Continue Adventure" }).click();
    }
    if (
      await page.getByRole("button", { name: "Enter the dungeon" }).isVisible()
    ) {
      await page.getByRole("button", { name: "Enter the dungeon" }).click();
      await expect(
        page
          .getByRole("region", { name: "Dungeon exploration" })
          .getByRole("heading", { name: "Warden's Gate Pass" })
          .first(),
      ).toBeVisible();
      await collector.milestone("first-person dungeon entry", {
        screenshot: true,
        layerSnapshot: {
          viewport: await page.getByRole("img").getAttribute("aria-label"),
          renderer: await page
            .getByRole("img")
            .getAttribute("data-renderer-backend"),
          rendererErrors: await page
            .getByRole("img")
            .getByRole("alert")
            .count(),
          status: await page.getByLabel("Party status").innerText(),
        },
      });
      for (let step = 0; step < 4; step += 1) {
        await page.getByRole("button", { name: "↑ Forward" }).click();
      }
      await expect(
        page.getByRole("heading", { name: "Silent murder holes" }),
      ).toBeVisible();
      await page.getByRole("button", { name: "Inspect" }).click();
      await collector.milestone("authored dungeon landmark inspected", {
        screenshot: true,
      });
      for (let step = 0; step < 4; step += 1) {
        await page.getByRole("button", { name: "↑ Forward" }).click();
      }
    }
    await expect(page.locator("aui-character-status")).toHaveCount(2);
    await page.getByRole("button", { name: "Longsword Strike" }).click();
    await expect(page.getByLabel("Latest outcome explanation")).toContainText(
      "d20",
    );
    await expect(
      page.getByRole("button", { name: "Resolve deterministic roll" }),
    ).toHaveCount(0);
    await collector.milestone("authored action resolved automatically", {
      screenshot: true,
      layerSnapshot: {
        latest: await page.getByLabel("Latest outcome explanation").innerText(),
      },
    });

    await page
      .getByRole("button", { name: /^Begin .+ turn$/ })
      .first()
      .click();
    const reaction = page.getByRole("region", {
      name: "Available reaction",
      exact: true,
    });
    if (await reaction.isVisible()) {
      await expect(reaction).toContainText(/Iron Warden|Gate Skirmisher/);
      await page.getByRole("button", { name: "Do not react" }).click();
    }
    await expect(page.getByLabel("Latest outcome explanation")).toContainText(
      "d20",
    );
    await collector.milestone("opposition action resolved automatically", {
      screenshot: true,
      layerSnapshot: {
        latest: await page.getByLabel("Latest outcome explanation").innerText(),
      },
    });
    await expect(page.getByLabel("Encounter identity")).toContainText(
      "Ilyra Fen acting",
    );
    await page.getByRole("button", { name: "Save", exact: true }).click();
    await expect(page.getByText("Saved", { exact: true })).toBeVisible();
    await collector.milestone(
      "opposition receipt advanced round and saved state",
      {
        screenshot: true,
        layerSnapshot: {
          route: page.url(),
          encounter: await page.getByLabel("Encounter identity").innerText(),
          latest: await page
            .getByLabel("Latest outcome explanation")
            .innerText(),
        },
      },
    );

    await page.setViewportSize({ width: 390, height: 844 });
    await expect(
      page.getByRole("button", { name: "Longsword Strike" }),
    ).toBeVisible();
    await collector.milestone("mobile encounter shell", { screenshot: true });
  },
);
