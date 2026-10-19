import { expect, test } from "@playwright/test";
import type { Download, Page } from "@playwright/test";
import { readFile } from "node:fs/promises";
import path from "node:path";

const media = path.resolve("../fixtures/media");
/** Every refusal of a browser encode ends by pointing to the desktop app. */
const DESKTOP_POINTER = /Open it in the Fitifact desktop app\.$/;

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

/** Aims a dropped video at hard constraints pasted as JSON, the custom target for video. */
async function customTarget(page: Page, name: string, container: RegExp, hard: object[]) {
  await page.locator("input[type=file]").setInputFiles(path.join(media, name));
  await expect(page.locator(".file-row")).toContainText(container, { timeout: 30_000 });
  const summary = page.locator(".requirements-disclosure > summary");
  if ((await summary.isVisible()) && (await page.locator(".requirements-disclosure").getAttribute("open")) === null) {
    await summary.click();
  }
  await page.getByRole("button", { name: "Custom", exact: true }).click();
  await page
    .getByLabel("Rejection message or requirements")
    .fill(JSON.stringify({ schema: "fitifact.constraints/v1", hard }));
}

/**
 * The plan once the engine has judged this browser: ready to run, or refused in plain words
 * with the desktop link. Which one depends on the encoders the browser ships.
 */
async function planOutcome(page: Page): Promise<"ready" | "refused"> {
  const ready = page.getByRole("heading", { name: "Minimum changes ready" });
  const refused = page.getByText(DESKTOP_POINTER);
  await expect(ready.or(refused).first()).toBeVisible({ timeout: 60_000 });
  if (await ready.isVisible()) {
    test.info().annotations.push({ type: "browser-encode", description: "ready" });
    return "ready";
  }
  await expect(page.getByRole("link", { name: "Get the desktop app" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Adapt and download" })).toHaveCount(0);
  test.info().annotations.push({ type: "browser-encode", description: (await refused.first().textContent()) ?? "" });
  return "refused";
}

/** Runs the plan and returns the downloaded MP4 once the engine has validated it. */
async function adaptAndVerify(page: Page) {
  const [download] = await Promise.all([
    page.waitForEvent("download", { timeout: 120_000 }),
    page.getByRole("button", { name: "Adapt and download" }).click(),
  ]);
  await expect(page.getByRole("heading", { name: "Adapted and verified" })).toBeVisible();
  await expect(page.locator(".checklist tbody tr.fail")).toHaveCount(0);
  expect(download.suggestedFilename()).toMatch(/\.mp4$/);
  const output = await saved(download);
  expect(output.toString("latin1", 4, 8)).toBe("ftyp");
  expect(output.includes("avc1")).toBe(true);
  return output;
}

/**
 * Chromium on Windows decodes and encodes H.264 and AAC through Media Foundation, so there the
 * encode must run. Elsewhere (Playwright's Linux Chromium ships no proprietary codecs) the
 * engine's measurement decides, and both outcomes are checked in full.
 */
const mustEncode = (browserName: string) => browserName === "chromium" && process.platform === "win32";

const H264_AAC_MP4 = [
  { id: "container", field: "media.container", op: "in", value: ["mp4"] },
  { id: "video-codec", field: "media.video.codec", op: "in", value: ["h264"] },
  { id: "audio-codec", field: "media.audio.codec", op: "in", value: ["aac"] },
];

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

test("H.264 over its byte limit runs the bitrate ladder and validates in Chromium", async ({ page, browserName }) => {
  test.slow(); // a real encode, possibly more than one
  await page.goto("/");
  const limit = 40_000;
  await customTarget(page, "ladder-h264-aac.mp4", /MP4/, [
    ...H264_AAC_MP4,
    { id: "bytes", field: "file.bytes", op: "lte", value: limit },
  ]);
  const outcome = await planOutcome(page);
  if (mustEncode(browserName)) expect(outcome).toBe("ready");
  if (outcome === "refused") return;
  const output = await adaptAndVerify(page);
  expect(output.byteLength).toBeLessThanOrEqual(limit);
  expect(output.includes("mp4a")).toBe(true);
});

test("H.264 over a width limit is scaled to fit and validates in Chromium", async ({ page, browserName }) => {
  test.slow();
  await page.goto("/");
  await customTarget(page, "compatible-h264-aac.mp4", /MP4/, [
    ...H264_AAC_MP4,
    { id: "width", field: "media.video.width", op: "lte", value: 96 },
  ]);
  const outcome = await planOutcome(page);
  if (mustEncode(browserName)) expect(outcome).toBe("ready");
  if (outcome === "refused") return;
  await adaptAndVerify(page);
  await expect(page.locator(".checklist")).toContainText("96");
});

for (const [name, container] of [
  ["mismatch-hevc-aac.mp4", /MP4/],
  ["transcode-vp9-opus.webm", /WEBM/],
] as const) {
  test(`${name} becomes validated H.264 MP4 or is refused in plain words`, async ({ page }) => {
    test.slow();
    await page.goto("/");
    await dropVideo(page, name, container);
    if ((await planOutcome(page)) === "ready") await adaptAndVerify(page);
  });
}

test("HDR video is refused before any encoder starts", async ({ page }) => {
  await page.goto("/");
  await dropVideo(page, "refusal-hdr10-hevc-aac.mp4", /MP4/);
  await expect(page.getByText(/This video is HDR/)).toBeVisible({ timeout: 30_000 });
  await expect(page.getByRole("link", { name: "Get the desktop app" })).toBeVisible();
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

test("video tools that fail to load get a plain refusal", async ({ page, browserName }) => {
  test.skip(browserName !== "chromium", "Only Chromium routes a worker's module imports.");
  await page.context().route(/\/assets\/media-[\w-]+\.js$/, (route) => route.abort());
  await page.goto("/");
  await page.locator("input[type=file]").setInputFiles(path.join(media, "remux-h264-aac.mov"));
  await expect(
    page.getByText("This browser couldn't load the web app's video tools. Open the video in the Fitifact desktop app.").first(),
  ).toBeVisible({ timeout: 30_000 });
});
