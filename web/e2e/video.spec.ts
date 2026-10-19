import { expect, test } from "@playwright/test";
import type { Download, Page } from "@playwright/test";
import { readFile } from "node:fs/promises";
import path from "node:path";

const media = path.resolve("../fixtures/media");
const NEEDS_DESKTOP = "This video needs re-encoding, which the web app can't do yet. Open it in the Fitifact desktop app.";

/**
 * Records the path of every request, including the worker's: WebKit reports a worker's module
 * import as "xhr" rather than "script", so the type is not filtered.
 */
function requestLog(page: Page) {
  const paths: string[] = [];
  page.on("request", (request) => paths.push(new URL(request.url()).pathname));
  return paths;
}

async function dropVideo(page: Page, name: string, container: RegExp) {
  await page.locator("input[type=file]").setInputFiles(path.join(media, name));
  await expect(page.locator(".file-row")).toContainText(container, { timeout: 30_000 });
  await page.locator(".destination-chips").getByRole("button", { name: /Web video/ }).click();
}

async function saved(download: Download) {
  const savedPath = await download.path();
  expect(savedPath).not.toBeNull();
  return readFile(savedPath!);
}

test("a video that already fits is reported compatible and left untouched", async ({ page }) => {
  await page.goto("/");
  await dropVideo(page, "compatible-h264-aac.mp4", /MP4/);
  await expect(page.getByRole("heading", { name: "Already compatible. Nothing changed." })).toBeVisible();
  const original = page.getByRole("link", { name: "Use original" });
  await expect(original).toHaveAttribute("download", "compatible-h264-aac.mp4");
  const [download] = await Promise.all([page.waitForEvent("download"), original.click()]);
  expect((await saved(download)).equals(await readFile(path.join(media, "compatible-h264-aac.mp4")))).toBe(true);
});

for (const [name, container] of [
  ["remux-h264-aac.mov", /MOV/],
  ["remux-h264-aac.mkv", /MKV/],
] as const) {
  test(`${name} is copied into a validated MP4 without re-encoding`, async ({ page }) => {
    await page.goto("/");
    await dropVideo(page, name, container);
    await expect(page.getByRole("heading", { name: "Minimum changes ready" })).toBeVisible();
    const [download] = await Promise.all([
      page.waitForEvent("download"),
      page.getByRole("button", { name: "Adapt and download" }).click(),
    ]);
    await expect(page.getByRole("heading", { name: "Adapted and verified" })).toBeVisible();
    expect(download.suggestedFilename()).toMatch(/\.mp4$/);
    const output = await saved(download);
    expect(output.toString("latin1", 4, 8)).toBe("ftyp");
    await expect(page.locator(".checklist tbody tr.fail")).toHaveCount(0);
  });
}

test("a video that needs re-encoding is refused in plain words with the desktop link", async ({ page }) => {
  await page.goto("/");
  await dropVideo(page, "mismatch-hevc-aac.mp4", /MP4/);
  await expect(page.getByText(NEEDS_DESKTOP)).toBeVisible();
  await expect(page.getByRole("link", { name: "Get the desktop app" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Adapt and download" })).toHaveCount(0);
});

test("a video stream the browser would drop is refused, not lost", async ({ page }) => {
  await page.goto("/");
  await dropVideo(page, "unsupported-extra-video.mp4", /MP4/);
  await expect(page.getByText("Fitifact won’t produce a file that breaks a requirement.")).toBeVisible();
  await expect(page.getByRole("button", { name: "Adapt and download" })).toHaveCount(0);
});

test("the video tools load only for a video", async ({ page, browserName }) => {
  test.skip(browserName === "firefox", "Firefox does not report module imports inside a worker as page requests.");
  const requested = requestLog(page);
  const videoTools = (paths: string[]) => paths.some((url) => /\/assets\/media-[\w-]+\.js$/.test(url));
  await page.goto("/");
  await page.locator("input[type=file]").setInputFiles(path.resolve("../fixtures/image/compatible-jpeg.jpg"));
  await page.locator(".destination-chips").getByRole("button", { name: /Web image/ }).click();
  await expect(page.getByRole("heading", { name: /Already compatible|Minimum changes ready/ })).toBeVisible();
  expect(videoTools(requested)).toBe(false);
  await dropVideo(page, "compatible-h264-aac.mp4", /MP4/);
  await expect(page.getByRole("heading", { name: "Already compatible. Nothing changed." })).toBeVisible();
  expect(videoTools(requested)).toBe(true);
});
