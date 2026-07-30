import { expect, liveScenario } from "./support/live-scenario";

liveScenario(
  "Rust-owned authored encounter live evidence @live",
  async ({ page, request, collector, liveBaseUrl }) => {
    liveScenario.setTimeout(120_000);
    collector.addNonClaim(
      "This bounded scenario does not claim free movement, browser topology authority, or native-window input.",
    );

    await page.goto(liveBaseUrl);
    const newWardensGate = page.getByRole("button", {
      name: "New Adventure · The Warden's Gate",
      exact: true,
    });
    const continueAdventure = page.getByRole("button", {
      name: "Continue Adventure",
      exact: true,
    });
    await expect(newWardensGate.or(continueAdventure)).toBeVisible();
    if (await newWardensGate.isVisible()) {
      await collector.milestone("empty game ready", { screenshot: true });
      await newWardensGate.click();
      await expect(
        page.getByRole("heading", { name: "The Warden's Gate Camp" }),
      ).toBeVisible();
      await expect(page.getByLabel("Armor defense readout")).toContainText(
        "18",
      );
      await collector.milestone("durable adventure camp loadout", {
        screenshot: true,
        layerSnapshot: {
          inventory: await page
            .getByRole("region", { name: "Mara Venn pack", exact: true })
            .innerText(),
          equipment: await page
            .getByRole("region", {
              name: "Mara Venn equipment",
              exact: true,
            })
            .innerText(),
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
    } else if (await continueAdventure.isVisible()) {
      await continueAdventure.click();
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
      const dungeonRenderer = page.locator(
        "aui-game-viewport [data-renderer-backend]",
      );
      await collector.milestone("first-person dungeon entry", {
        screenshot: true,
        layerSnapshot: {
          viewport: await dungeonRenderer.getAttribute("aria-label"),
          renderer: await dungeonRenderer.getAttribute("data-renderer-backend"),
          rendererErrors: await dungeonRenderer.getByRole("alert").count(),
          status: await page.getByLabel("Party status").innerText(),
        },
      });
      const dungeonCanvas = dungeonRenderer.locator("canvas");
      await dungeonCanvas.evaluate((element) => {
        const target = element as HTMLCanvasElement & {
          cameraTweenWitness?: string[];
        };
        target.cameraTweenWitness = [];
        new MutationObserver(() => {
          target.cameraTweenWitness?.push(
            [
              target.dataset["cameraMotion"] ?? "none",
              target.dataset["cameraTransitionState"] ?? "unknown",
              target.dataset["cameraPose"] ?? "unknown",
            ].join("|"),
          );
        }).observe(target, {
          attributeFilter: [
            "data-camera-motion",
            "data-camera-pose",
            "data-camera-transition-state",
          ],
        });
      });
      await page.getByRole("button", { name: "↶ Left" }).click();
      await expect(dungeonRenderer).toHaveAttribute(
        "aria-label",
        /facing north at cell 1, 1/,
      );
      await page.getByRole("button", { name: "Right ↷" }).click();
      await expect(dungeonRenderer).toHaveAttribute(
        "aria-label",
        /facing east at cell 1, 1/,
      );
      await expect(dungeonCanvas).toHaveAttribute(
        "data-camera-transition-state",
        "settled",
      );
      const tweenWitness = await dungeonCanvas.evaluate((element) => {
        const target = element as HTMLCanvasElement & {
          cameraTweenWitness?: string[];
        };
        return target.cameraTweenWitness ?? [];
      });
      expect(
        tweenWitness.some((entry) => entry.startsWith("turn-left|running|")),
      ).toBe(true);
      expect(
        tweenWitness.some((entry) => entry.startsWith("turn-right|running|")),
      ).toBe(true);
      expect(
        new Set(tweenWitness.map((entry) => entry.split("|").at(-1))).size,
      ).toBeGreaterThan(2);
      await collector.milestone(
        "Engine camera tween remains projection-bound",
        {
          screenshot: true,
          layerSnapshot: {
            cameraMotion:
              await dungeonCanvas.getAttribute("data-camera-motion"),
            cameraPose: await dungeonCanvas.getAttribute("data-camera-pose"),
            transitionState: await dungeonCanvas.getAttribute(
              "data-camera-transition-state",
            ),
            witness: tweenWitness,
          },
        },
      );
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
    await expect(page.locator("aui-character-status")).toHaveCount(6);
    await page.getByRole("button", { name: "Longsword Strike" }).click();
    const tacticalBoard = page.getByRole("application", {
      name: /Rendered tactical combat board/,
    });
    await expect(tacticalBoard).toHaveAttribute(
      "data-interaction-mode",
      "targeting",
    );
    await collector.milestone("authored action awaits a rendered-grid target", {
      screenshot: true,
      layerSnapshot: {
        board: await tacticalBoard.getAttribute("aria-label"),
        status: await page.locator(".targeting-status").innerText(),
      },
    });
    await tacticalBoard.focus();
    await page.keyboard.press("Enter");
    await expect(page.getByLabel("Encounter identity")).toContainText(
      "Gate Skirmisher acting",
    );
    const partySnapshot = (await (
      await request.get(`${liveBaseUrl}/api/v1/session`)
    ).json()) as {
      encounter: {
        log: Array<{ source: string; text: string; details: string[] }>;
      };
    };
    const partyReceipt = partySnapshot.encounter.log.find(
      (entry) => entry.source === "Longsword Strike",
    );
    expect(partyReceipt?.details.join(" ")).toContain("d20");
    await expect(
      page.getByRole("button", { name: "Resolve deterministic roll" }),
    ).toHaveCount(0);
    await collector.milestone("authored action resolved automatically", {
      screenshot: true,
      layerSnapshot: {
        receipt: partyReceipt,
        viewport: await page
          .getByRole("region", { name: /tactical encounter/ })
          .getAttribute("aria-label"),
        renderer: await page
          .getByRole("region", { name: /tactical encounter/ })
          .getAttribute("data-renderer-backend"),
        rendererErrors: await page
          .getByRole("region", { name: /tactical encounter/ })
          .getByRole("alert")
          .count(),
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
    const nextPartyTurn = page
      .getByLabel("Encounter identity")
      .getByText("Ilyra Fen acting", { exact: true });
    await expect(reaction.or(nextPartyTurn)).toBeVisible();
    if (await reaction.isVisible()) {
      await expect(reaction).toContainText(/Iron Warden|Gate Skirmisher/);
      await page.getByRole("button", { name: "Do not react" }).click();
    }
    await expect(nextPartyTurn).toBeVisible();
    const oppositionSnapshot = (await (
      await request.get(`${liveBaseUrl}/api/v1/session`)
    ).json()) as {
      encounter: {
        log: Array<{ source: string; text: string; details: string[] }>;
      };
    };
    expect(
      oppositionSnapshot.encounter.log.some(
        (entry) =>
          entry.source !== "Longsword Strike" &&
          entry.details.some((detail) => detail.includes("d20")),
      ),
    ).toBe(true);
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
      page.getByRole("button", { name: "End Ilyra Fen activation" }),
    ).toBeVisible();
    await collector.milestone("mobile encounter shell", { screenshot: true });
  },
);
