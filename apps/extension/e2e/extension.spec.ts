import { test as base, chromium, expect, type BrowserContext, type FrameLocator, type Page } from "@playwright/test";
import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";

const dist = path.resolve("dist");
const source = path.resolve("../../fixtures/media/compatible-h264-aac.mp4");
const hdr = path.resolve("../../fixtures/media/refusal-hdr10-hevc-aac.mp4");
const HOSTS = ["discord.com", "mail.google.com", "web.whatsapp.com"];

// Stands in for each site: a plain picker, a WhatsApp-style media picker, and a drop zone.
// It records every file the page itself receives, the way the site's own handlers would.
const PAGE = `<!doctype html><title>upload</title>
<input id="any" type="file" multiple>
<input id="media" type="file" accept="image/*,video/mp4,video/3gpp,video/quicktime" multiple>
<div id="drop" style="width:400px;height:200px;border:1px dashed">Drop here</div>
<script>
  window.got = [];
  document.body.addEventListener("change", (e) => got.push({ how: "change", files: [...e.target.files] }));
  const zone = document.getElementById("drop");
  zone.addEventListener("dragover", (e) => e.preventDefault());
  zone.addEventListener("drop", (e) => { e.preventDefault(); got.push({ how: "drop", files: [...e.dataTransfer.files] }); });
</script>`;

const test = base.extend<{ offsite: string[]; context: BrowserContext; page: Page }>({
  offsite: async ({}, use) => use([]),
  context: async ({ offsite }, use) => {
    const context = await chromium.launchPersistentContext("", {
      channel: "chromium",
      acceptDownloads: true,
      args: [`--disable-extensions-except=${dist}`, `--load-extension=${dist}`],
    });
    // Nothing reaches the real sites; any other request outside the extension is a privacy failure.
    await context.route((url) => url.protocol !== "chrome-extension:", (route) => {
      const request = route.request();
      const url = new URL(request.url());
      if (!HOSTS.includes(url.hostname)) {
        offsite.push(url.href);
        return route.abort();
      }
      return request.resourceType() === "document"
        ? route.fulfill({ contentType: "text/html", body: PAGE })
        : route.fulfill({ status: 404, body: "" });
    });
    await use(context);
    await context.close();
    expect(offsite, "requests outside the extension and the test page").toEqual([]);
  },
  page: async ({ context }, use) => use(context.pages()[0] ?? (await context.newPage())),
});

/** A tiny fixture (H.264/AAC by default), padded with an MP4 `free` box until it is just over `bytes`. */
async function over(bytes: number, name = "clip.mp4", from = source): Promise<string> {
  const video = await readFile(from);
  const box = Buffer.alloc(bytes + 1 - video.length);
  box.writeUInt32BE(box.length, 0);
  box.write("free", 4, "latin1");
  const file = test.info().outputPath(name);
  await writeFile(file, Buffer.concat([video, box]));
  return file;
}

type Received = { how: string; files: Array<{ name: string; size: number; type: string; head: string }> };

function received(page: Page): Promise<Received[]> {
  return page.evaluate(() =>
    Promise.all(
      (window as unknown as { got: Array<{ how: string; files: File[] }> }).got.map(async ({ how, files }) => ({
        how,
        files: await Promise.all(
          files.map(async (file) => ({
            name: file.name,
            size: file.size,
            type: file.type,
            head: new TextDecoder().decode((await file.slice(4, 8).arrayBuffer()) as ArrayBuffer),
          })),
        ),
      })),
    ),
  );
}

const panelOf = (page: Page) => page.frameLocator('iframe[src^="chrome-extension://"]');

/** Drops a fixture, padded just over 20 MB, on the page's drop zone the way a person would. */
async function dropOver(page: Page, name: string, from = source) {
  const head = [...(await readFile(from))];
  await page.evaluate(
    ({ head, name }) => {
      const pad = new Uint8Array(20_000_001 - head.length);
      new DataView(pad.buffer).setUint32(0, pad.length);
      pad.set([0x66, 0x72, 0x65, 0x65], 4);
      const data = new DataTransfer();
      data.items.add(new File([new Uint8Array(head), pad], name, { type: "video/mp4" }));
      document.getElementById("drop")!.dispatchEvent(new DragEvent("drop", { bubbles: true, cancelable: true, dataTransfer: data }));
    },
    { head, name },
  );
}

/**
 * Chromium on Windows encodes H.264 through Media Foundation, so there every held video here must
 * be fittable. Playwright's Linux Chromium ships no proprietary codecs, so there the engine
 * refuses the encode and the tests that need a fitted file are skipped.
 */
const mustEncode = process.platform === "win32";
const NO_H264 = "This Chromium can't encode H.264, so the engine refuses the fit.";

/** Waits for the panel's verdict on a held video and says whether it offers a fit. */
async function decided(scope: Page | FrameLocator, destination: string): Promise<boolean> {
  const heading = scope.getByRole("heading");
  await expect(heading).toHaveText(new RegExp(`^(Too big for|Can't fit this for) ${destination}`));
  const fits = (await heading.textContent())!.startsWith("Too big");
  if (mustEncode) expect(fits, "Chromium on Windows encodes H.264").toBe(true);
  return fits;
}

test("Discord: a video over the cap is held, fitted, verified, and handed to the page", async ({ page }) => {
  await page.goto("https://discord.com/channels/1/2");
  await page.locator("#any").setInputFiles(await over(20_000_000));
  const panel = panelOf(page);
  const fits = await decided(panel, "Discord");
  expect(await received(page), "the page must not see the oversized file").toEqual([]);
  test.skip(!fits, NO_H264);
  await expect(panel.getByText("Discord Free takes videos up to 20 MB.")).toBeVisible();
  await expect(panel.getByText("Re-encode so the file fits the size limit")).toBeVisible();

  await panel.getByRole("button", { name: "Fit for Discord" }).click();
  await expect.poll(() => received(page), { timeout: 90_000 }).toHaveLength(1);
  const [delivery] = await received(page);
  expect(delivery.how).toBe("change");
  expect(delivery.files).toHaveLength(1);
  expect(delivery.files[0]).toMatchObject({ name: "clip.mp4", type: "video/mp4", head: "ftyp" });
  expect(delivery.files[0].size).toBeLessThanOrEqual(20_000_000);
  await expect(panel.getByRole("heading")).toHaveText("Fits Discord now");
  await expect(panel.getByText(/Verified: the new file matches the destination rules/)).toBeVisible();
});

test("a video under the cap reaches the page untouched, with no panel", async ({ page }) => {
  await page.goto("https://mail.google.com/mail/u/0/");
  await page.locator("#any").setInputFiles(source);
  const [delivery] = await received(page);
  expect(delivery.files).toEqual([expect.objectContaining({ name: "compatible-h264-aac.mp4", size: 13_520 })]);
  await expect(page.locator("iframe")).toHaveCount(0);
});

test("Gmail: Use original hands over the original file", async ({ page }) => {
  await page.goto("https://mail.google.com/mail/u/0/");
  const file = await over(25_000_000, "trip.mp4");
  await page.locator("#any").setInputFiles(file);
  const panel = panelOf(page);
  await decided(panel, "Gmail");
  await expect(panel.getByRole("combobox")).toBeHidden();
  await panel.getByRole("button", { name: "Use original" }).click();
  await expect.poll(() => received(page)).toHaveLength(1);
  const [delivery] = await received(page);
  expect(delivery.files[0]).toMatchObject({ name: "trip.mp4", size: 25_000_001 });
  await expect(page.locator("iframe")).toHaveCount(0);
});

test("Discord: declaring a bigger plan lets the original through untouched", async ({ page }) => {
  await page.goto("https://discord.com/channels/1/2");
  await page.locator("#any").setInputFiles(await over(20_000_000));
  const panel = panelOf(page);
  await decided(panel, "Discord");
  await panel.getByRole("combobox", { name: "Your Discord plan" }).selectOption("nitro-basic");
  await expect.poll(() => received(page)).toHaveLength(1);
  expect((await received(page))[0].files[0].size).toBe(20_000_001);
  // The next pick uses the declared plan without asking.
  await page.locator("#any").setInputFiles(await over(20_000_000, "second.mp4"));
  await expect.poll(() => received(page)).toHaveLength(2);
  await expect(page.locator("iframe")).toHaveCount(0);
});

test("WhatsApp: a document upload is left alone; a media upload is held and can be cancelled", async ({ page }) => {
  await page.goto("https://web.whatsapp.com/");
  const file = await over(64_000_000);
  await page.locator("#any").setInputFiles(file);
  await expect.poll(() => received(page)).toHaveLength(1);
  await expect(page.locator("iframe")).toHaveCount(0);

  await page.locator("#media").setInputFiles(file);
  const panel = panelOf(page);
  await decided(panel, "WhatsApp");
  await panel.getByRole("button", { name: "Cancel" }).click();
  await expect(page.locator("iframe")).toHaveCount(0);
  expect(await received(page)).toHaveLength(1);
  expect(await page.locator("#media").evaluate((input: HTMLInputElement) => input.value)).toBe("");
});

test("Discord: a dropped video is held, fitted, and dropped again on the same target", async ({ page }) => {
  await page.goto("https://discord.com/channels/1/2");
  await dropOver(page, "drop.mp4");
  const panel = panelOf(page);
  const fits = await decided(panel, "Discord");
  expect(await received(page)).toEqual([]);
  test.skip(!fits, NO_H264);
  await panel.getByRole("button", { name: "Fit for Discord" }).click();
  await expect.poll(() => received(page), { timeout: 90_000 }).toHaveLength(1);
  const [delivery] = await received(page);
  expect(delivery.how).toBe("drop");
  expect(delivery.files[0]).toMatchObject({ name: "drop.mp4", type: "video/mp4", head: "ftyp" });
  expect(delivery.files[0].size).toBeLessThanOrEqual(20_000_000);
});

test("Discord: a video the browser can't fit is explained, and Use original drops it again untouched", async ({ page }) => {
  await page.goto("https://discord.com/channels/1/2");
  await dropOver(page, "hdr.mp4", hdr);
  const panel = panelOf(page);
  await expect(panel.getByRole("heading")).toHaveText("Can't fit this for Discord here");
  await expect(panel.getByText(/Open it in the Fitifact desktop app/)).toBeVisible();
  await expect(panel.getByRole("button", { name: "Fit for Discord" })).toBeHidden();
  expect(await received(page)).toEqual([]);
  await panel.getByRole("button", { name: "Use original" }).click();
  await expect.poll(() => received(page)).toHaveLength(1);
  const [delivery] = await received(page);
  expect(delivery.how).toBe("drop");
  expect(delivery.files[0]).toMatchObject({ name: "hdr.mp4", size: 20_000_001 });
  await expect(page.locator("iframe")).toHaveCount(0);
});

test("the popup sets the plan and opens a tab that fits a chosen video for download", async ({ context, page }) => {
  // Learn the extension's id from the panel it frames over a held file.
  await page.goto("https://discord.com/channels/1/2");
  await page.locator("#any").setInputFiles(await over(20_000_000));
  const src = await page.locator('iframe[src^="chrome-extension://"]').getAttribute("src");
  // chrome-extension: URLs have an opaque origin, so take the scheme and id by hand.
  const origin = src!.match(/^chrome-extension:\/\/[a-p]{32}/)![0];

  const popup = await context.newPage();
  await popup.goto(`${origin}/popup.html`);
  const plan = popup.getByRole("combobox", { name: "Your Discord plan" });
  await expect(plan).toHaveValue("free");
  await plan.selectOption("nitro");
  const opened = context.waitForEvent("page");
  await popup.getByRole("button", { name: "Fit a video" }).click();
  const tab = await opened;
  await expect(tab).toHaveURL(`${origin}/panel.html`);
  await expect(tab.getByRole("heading")).toHaveText("Fit a video");
  await expect(tab.getByRole("combobox", { name: "Your Discord plan" })).toHaveValue("nitro");

  await tab.getByRole("combobox", { name: "Where it's going" }).selectOption("gmail");
  await tab.locator("#file").setInputFiles(await over(25_000_000, "talk.mov"));
  test.skip(!(await decided(tab, "Gmail")), NO_H264);
  await expect(tab.getByRole("button", { name: "Use original" })).toBeHidden();
  await tab.getByRole("button", { name: "Fit for Gmail" }).click();
  const link = tab.getByRole("link", { name: "Download talk.mp4" });
  await expect(link).toBeVisible({ timeout: 90_000 });
  const downloading = tab.waitForEvent("download");
  await link.click();
  const download = await downloading;
  expect(download.suggestedFilename()).toBe("talk.mp4");
  const saved = await readFile((await download.path())!);
  expect(saved.length).toBeLessThanOrEqual(25_000_000);
  expect(saved.subarray(4, 8).toString("latin1")).toBe("ftyp");
});
