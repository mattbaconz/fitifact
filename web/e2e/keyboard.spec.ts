import { expect, test } from "@playwright/test";
import type { Locator, Page } from "@playwright/test";
import path from "node:path";
import { expectRealDownload, fixtures, holdAdaptation, JPEG, PNG } from "./support";

// Every primary journey, keyboard only: Tab moves focus, Enter and Space act.
// Plain Tab in every engine, so Safari's default (links skipped) is covered too.
async function tabTo(page: Page, target: Locator, limit = 150) {
  await expect(target).toBeVisible();
  for (let step = 0; step < limit; step++) {
    if (await target.evaluate((element) => element === document.activeElement)) return;
    await page.keyboard.press("Tab");
  }
  throw new Error(`Tab never reached ${target}`);
}

test("choose a file, pick a destination, adapt and download", async ({ page }) => {
  await page.goto("/");
  await tabTo(page, page.locator(".drop-zone-hero"));
  const [chooser] = await Promise.all([page.waitForEvent("filechooser"), page.keyboard.press("Enter")]);
  await chooser.setFiles(path.join(fixtures, "still-bmp.bmp"));
  await tabTo(page, page.locator(".destination-chips").getByRole("button", { name: /Web image/ }));
  await page.keyboard.press("Enter");
  const adapt = page.getByRole("button", { name: "Adapt and download" });
  await expect(adapt).toBeEnabled({ timeout: 30_000 });
  await tabTo(page, adapt);
  await expectRealDownload(page, () => page.keyboard.press("Enter"), "image/jpeg", JPEG);
  await expect(page.getByRole("heading", { name: "Adapted and verified" })).toBeFocused();
});

test("enter requirements, adapt and download", async ({ page }) => {
  await page.goto("/");
  await tabTo(page, page.getByRole("button", { name: "Try a sample image" }).first());
  await page.keyboard.press("Enter");
  const custom = page.getByRole("button", { name: "Custom", exact: true });
  await expect(custom).toBeEnabled({ timeout: 30_000 });
  await tabTo(page, custom);
  await page.keyboard.press("Enter");
  await tabTo(page, page.getByLabel("Rejection message or requirements"));
  await page.keyboard.type("JPEG, max 2 MB");
  await expect(page.getByRole("heading", { name: "Minimum changes ready" })).toBeVisible({ timeout: 15_000 });
  await tabTo(page, page.getByRole("button", { name: "Adapt and download" }));
  await expectRealDownload(page, () => page.keyboard.press("Enter"), "image/jpeg", JPEG);
});

test("approve a crop, adapt and download", async ({ page }) => {
  await page.goto("/");
  await tabTo(page, page.getByRole("button", { name: /Try a crop/ }));
  await page.keyboard.press("Enter");
  const position = page.getByLabel(/crop position/i);
  await tabTo(page, position);
  await page.keyboard.press("Home");
  await expect(position).toHaveValue("0");
  const consent = page.getByRole("checkbox", { name: /I approve removing/ });
  await tabTo(page, consent);
  await page.keyboard.press("Space");
  await expect(consent).toBeChecked();
  await tabTo(page, page.getByRole("button", { name: "Adapt and download" }));
  await expectRealDownload(page, () => page.keyboard.press("Enter"), "image/jpeg", JPEG);
  await expect(page.getByRole("heading", { name: "Adapted and verified" })).toBeFocused();
});

test("keep an already compatible file", async ({ page }) => {
  await page.goto("/");
  await tabTo(page, page.getByRole("button", { name: /Try an unchanged file/ }));
  await page.keyboard.press("Enter");
  await tabTo(page, page.getByRole("link", { name: "Use original" }));
  await expectRealDownload(page, () => page.keyboard.press("Enter"), "image/png", PNG);
});

test("cancel an adaptation", async ({ page }) => {
  await holdAdaptation(page);
  await page.goto("/");
  await tabTo(page, page.getByRole("button", { name: /Try a format change/ }));
  await page.keyboard.press("Enter");
  await tabTo(page, page.getByRole("button", { name: "Adapt and download" }));
  await page.keyboard.press("Enter");
  await tabTo(page, page.getByRole("button", { name: "Cancel processing" }));
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: "Processing cancelled" })).toBeVisible();
});

test("open and close settings", async ({ page }) => {
  await page.goto("/");
  const menu = page.getByRole("button", { name: "Settings and navigation" });
  await tabTo(page, menu);
  await page.keyboard.press("Enter");
  await expect(page.getByRole("dialog", { name: "Settings", exact: true })).toBeVisible();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(menu).toBeFocused();
});
