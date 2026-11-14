import { expect } from "@playwright/test";
import type { Locator, Page } from "@playwright/test";
import { readFile } from "node:fs/promises";
import path from "node:path";

export const fixtures = path.resolve("../fixtures/image");
export const JPEG = [0xff, 0xd8, 0xff];
export const PNG = [0x89, 0x50, 0x4e, 0x47, 13, 10, 26, 10];

export async function dropImage(page: Page, name: string) {
  await page.locator("input[type=file]").setInputFiles(path.join(fixtures, name));
  await expect(page.locator(".target-tabs button").filter({ hasText: /^Custom$/ })).toBeEnabled({ timeout: 30_000 });
  await expect(page.locator(".file-row")).toContainText(/JPEG|PNG|WebP|HEIC|TIFF|BMP|GIF/);
}

export async function pasteRequirements(page: Page, requirement: string) {
  const summary = page.locator(".requirements-disclosure > summary");
  if (await summary.isVisible() && !(await page.locator(".requirements-disclosure").getAttribute("open") !== null)) await summary.click();
  await page.getByRole("button", { name: "Custom", exact: true }).click();
  await page.getByLabel("Rejection message or requirements").fill(requirement);
  await expect(page.getByText(/Understood:/)).toBeAttached();
  await expect(page.locator(".target-summary")).not.toHaveText("", { timeout: 15_000 });
}

/** Triggers a download (a click, or any action such as a key press) and checks the saved bytes. */
export async function expectRealDownload(
  page: Page,
  trigger: Locator | (() => Promise<unknown>),
  mime: "image/jpeg" | "image/png",
  signature: number[],
  original?: Buffer,
) {
  const [download] = await Promise.all([
    page.waitForEvent("download"),
    typeof trigger === "function" ? trigger() : trigger.click(),
  ]);
  expect(download.suggestedFilename()).toMatch(mime === "image/jpeg" ? /\.jpg$/ : /\.png$/);
  const savedPath = await download.path();
  expect(savedPath).not.toBeNull();
  const saved = await readFile(savedPath!);
  expect(Array.from(saved.subarray(0, signature.length))).toEqual(signature);
  expect(saved.byteLength).toBeGreaterThan(signature.length);
  if (original) expect(saved.equals(original)).toBe(true);
  return download;
}

/** Holds adaptation requests before they reach the worker, so the adapting state stays put until Cancel. */
export async function holdAdaptation(page: Page) {
  await page.addInitScript(() => {
    const send = Worker.prototype.postMessage as (this: Worker, ...args: unknown[]) => void;
    Worker.prototype.postMessage = function (this: Worker, message: unknown, ...rest: unknown[]) {
      if ((message as { type?: string } | null)?.type === "adapt") return;
      send.call(this, message, ...rest);
    };
  });
}

/**
 * Records the path of every request, including the worker's: WebKit reports a worker's module
 * import as "xhr" rather than "script", so the type is not filtered.
 */
export function requestLog(page: Page) {
  const paths: string[] = [];
  page.on("request", (request) => paths.push(new URL(request.url()).pathname));
  return paths;
}

export const videoTools = (paths: string[]) => paths.some((url) => /\/assets\/media-[\w-]+\.js$/.test(url));

/** Waits until no request, worker requests included, has been open for a second. */
export function whenQuiet(page: Page) {
  let open = 0;
  let last = Date.now();
  page.on("request", () => {
    open += 1;
    last = Date.now();
  });
  const settle = () => {
    open -= 1;
    last = Date.now();
  };
  page.on("requestfinished", settle);
  page.on("requestfailed", settle);
  return () => expect.poll(() => open <= 0 && Date.now() - last > 1000, { timeout: 30_000 }).toBe(true);
}
