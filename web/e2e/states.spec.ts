import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";
import type { Locator, Page, TestInfo } from "@playwright/test";
import { writeFile } from "node:fs/promises";
import { dropImage, holdAdaptation, pasteRequirements } from "./support";

// One designed screen per state at 1280×900 and 390×844, in both themes, each checked by axe.
// Behavior in every browser is covered by the workflow, keyboard, and privacy specs.
test.beforeEach(async ({ page }, testInfo) => {
  test.skip(
    !["chromium-desktop", "chromium-mobile"].includes(testInfo.project.name),
    "Screens are captured at the two reference viewports.",
  );
  await page.emulateMedia({ reducedMotion: "reduce" });
});

async function capture(page: Page, testInfo: TestInfo, state: string, anchor: Locator) {
  await expect(anchor).toBeVisible({ timeout: 30_000 });
  await anchor.scrollIntoViewIfNeeded();
  const findings: Record<string, unknown> = {};
  for (const theme of ["dark", "light"]) {
    await page.evaluate((value) => {
      document.documentElement.dataset.theme = value;
    }, theme);
    const { violations } = await new AxeBuilder({ page })
      .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
      .analyze();
    findings[theme] = violations.map(({ id, impact, nodes }) => ({
      id,
      impact,
      targets: nodes.map((node) => node.target),
    }));
    await page.screenshot({ path: testInfo.outputPath(`${state}-${theme}.png`), animations: "disabled" });
    expect(
      violations.filter(({ impact }) => impact === "serious" || impact === "critical"),
      `${state} in ${theme}`,
    ).toEqual([]);
  }
  await writeFile(testInfo.outputPath(`${state}-axe.json`), JSON.stringify(findings, null, 2));
}

test("empty", async ({ page }, testInfo) => {
  await page.goto("/");
  await capture(page, testInfo, "empty", page.getByRole("heading", { name: "Make your file work there." }));
});

test("inspecting", async ({ page }, testInfo) => {
  let release = () => {};
  const held = new Promise<void>((resolve) => (release = resolve));
  await page.route("**/samples/*.png", async (route) => {
    await held;
    await route.continue();
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Try a sample image" }).first().click();
  await capture(page, testInfo, "inspecting", page.getByRole("heading", { name: "Inspecting your file" }));
  release();
  await expect(page.getByRole("heading", { name: "Where does it need to work?" })).toBeVisible({ timeout: 30_000 });
});

test("consent", async ({ page }, testInfo) => {
  await page.goto("/");
  await page.getByRole("button", { name: /Try a crop/ }).click();
  await capture(page, testInfo, "consent", page.getByRole("heading", { name: "Crop approval required" }));
});

test("adapting, with cancel", async ({ page }, testInfo) => {
  await holdAdaptation(page);
  await page.goto("/");
  await page.getByRole("button", { name: /Try a format change/ }).click();
  await page.getByRole("button", { name: "Adapt and download" }).click();
  const cancel = page.getByRole("button", { name: "Cancel processing" });
  await capture(page, testInfo, "adapting", cancel);
  await cancel.click();
  await expect(page.getByRole("heading", { name: "Processing cancelled" })).toBeVisible();
});

test("already fits", async ({ page }, testInfo) => {
  await page.goto("/");
  await page.getByRole("button", { name: /Try an unchanged file/ }).click();
  await capture(page, testInfo, "already-fits", page.getByRole("heading", { name: "Already compatible. Nothing changed." }));
});

test("adapted", async ({ page }, testInfo) => {
  await page.goto("/");
  await page.getByRole("button", { name: /Try a format change/ }).click();
  await Promise.all([
    page.waitForEvent("download"),
    page.getByRole("button", { name: "Adapt and download" }).click(),
  ]);
  await capture(page, testInfo, "adapted", page.getByRole("heading", { name: "Adapted and verified" }));
});

test("warning", async ({ page }, testInfo) => {
  await page.goto("/");
  await dropImage(page, "compatible-jpeg.jpg");
  await pasteRequirements(page, "JPEG, max 2 MB, keep faces centered");
  await expect(page.locator(".result-caveat")).toContainText("Not used: keep faces centered");
  await capture(page, testInfo, "warning", page.getByRole("heading", { name: "Already compatible. Nothing changed." }));
});

test("refusal", async ({ page }, testInfo) => {
  await page.goto("/");
  const mp4 = Buffer.alloc(16);
  mp4.writeUInt32BE(16, 0);
  mp4.write("ftypisom", 4, "ascii");
  await page.locator("input[type=file]").setInputFiles({ name: "clip.mp4", mimeType: "video/mp4", buffer: mp4 });
  await capture(page, testInfo, "refusal-input", page.getByRole("heading", { name: "Fitifact can't adapt this file" }));
  await page.getByRole("button", { name: "Try a sample image" }).first().click();
  await pasteRequirements(page, "PNG, max 1 KB");
  await page.getByRole("button", { name: "Adapt and download" }).click();
  await capture(page, testInfo, "refusal", page.getByRole("heading", { name: "Cannot satisfy these requirements" }));
  await expect(page.getByText("Lossless PNG cannot meet the size limit.")).toBeVisible();
  await expect(page.getByRole("button", { name: "Retry" })).toHaveCount(0);
});

test("error", async ({ page }, testInfo) => {
  await page.goto("/");
  await page.locator("input[type=file]").setInputFiles({
    name: "broken.jpg",
    mimeType: "image/jpeg",
    buffer: Buffer.from([0xff, 0xd8, 0xff, 0, 0, 0, 0]),
  });
  await capture(page, testInfo, "error", page.getByRole("heading", { name: "Could not process this file" }));
  await expect(page.getByText(/couldn't read this image/)).toBeVisible();
});
