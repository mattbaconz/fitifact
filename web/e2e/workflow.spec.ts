import { expect, test } from "@playwright/test";
import type { Locator, Page } from "@playwright/test";
import { readFile } from "node:fs/promises";
import path from "node:path";

const fixtures = path.resolve("../fixtures/image");

async function dropImage(page: Page, name: string) {
  await page.getByLabel("Choose an image").setInputFiles(path.join(fixtures, name));
  await expect(page.getByLabel("Rejection message or requirements")).toBeEnabled({ timeout: 30_000 });
  await expect(page.locator(".file-row")).toContainText(/JPEG|PNG|WebP|HEIC|TIFF|BMP|GIF/);
}

async function pasteRequirements(page: Page, requirement: string) {
  await page.getByLabel("Rejection message or requirements").fill(requirement);
  await expect(page.getByRole("heading", { name: "I understood this as" })).toBeVisible();
  await expect(page.locator(".target-summary")).not.toHaveText("", { timeout: 15_000 });
}

async function openEditor(page: Page) {
  const edit = page.getByRole("button", { name: "Edit" });
  if (await edit.count()) await edit.click();
}

async function expectRealDownload(
  page: Page,
  link: Locator,
  mime: "image/jpeg" | "image/png",
  signature: number[],
  original?: Buffer,
) {
  const [download] = await Promise.all([page.waitForEvent("download"), link.click()]);
  expect(download.suggestedFilename()).toMatch(mime === "image/jpeg" ? /\.jpg$/ : /\.png$/);
  const savedPath = await download.path();
  expect(savedPath).not.toBeNull();
  const saved = await readFile(savedPath!);
  expect(Array.from(saved.subarray(0, signature.length))).toEqual(signature);
  expect(saved.byteLength).toBeGreaterThan(signature.length);
  if (original) expect(saved.equals(original)).toBe(true);
}

test("drop zone is visible on first paint without a requirements placeholder", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator(".drop-zone")).toBeInViewport();
  await expect(page.getByRole("button", { name: "Menu" })).toHaveAttribute("aria-expanded", "false");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Review requirements" })).toHaveCount(0);
  await expect(page.getByLabel("Rejection message or requirements")).toHaveCount(0);
});

test("happy path drops first, auto-parses, and exposes a validated download", async ({ page }) => {
  await page.goto("/");
  await dropImage(page, "mismatch-png.png");
  await expect(page.locator(".file-row")).toContainText("PNG");
  await pasteRequirements(page, "JPEG, max 2 MB");
  await expect(page.getByRole("heading", { name: "Minimum changes ready" })).toBeVisible();
  await page.getByRole("button", { name: "Fix image" }).click();
  await expect(page.getByRole("heading", { name: "Image adapted and validated" })).toBeVisible();
  await expect(page.getByText("validated against the requirements you confirmed", { exact: false })).toBeVisible();
  const download = page.getByRole("link", { name: "Download JPG" });
  await expect(download).toHaveAttribute("download", "mismatch-png.fitifact.jpg");
  await expect(page.locator(".checklist li.fail, .checklist li.unknown")).toHaveCount(0);
  await expectRealDownload(page, download, "image/jpeg", [0xff, 0xd8, 0xff]);
  await openEditor(page);
  await page.getByLabel("Maximum bytes").fill("1999999");
  await expect(download).toHaveCount(0);
  await expect(page.locator(".checklist")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Fix image" })).toHaveCount(0);
});

test("already-compatible path preserves the original", async ({ page }) => {
  await page.goto("/");
  await dropImage(page, "compatible-jpeg.jpg");
  await pasteRequirements(page, "JPEG, max 2 MB");
  await expect(page.getByRole("heading", { name: "Already compatible" })).toBeVisible();
  const download = page.getByRole("link", { name: "Use original image" });
  await expect(download).toHaveAttribute("download", "compatible-jpeg.fitifact.jpg");
  await expectRealDownload(
    page,
    download,
    "image/jpeg",
    [0xff, 0xd8, 0xff],
    await readFile(path.join(fixtures, "compatible-jpeg.jpg")),
  );
  await openEditor(page);
  await page.getByLabel("Maximum bytes").fill("1999999");
  await expect(download).toHaveCount(0);
  await expect(page.locator(".checklist")).toHaveCount(0);
  await page.getByRole("button", { name: "Review target changes" }).click();
  await expect(page.getByRole("link", { name: "Use original image" })).toBeVisible();
});

test("a failed replacement clears the previous source and is never rendered", async ({ page }) => {
  await page.goto("/");
  await dropImage(page, "compatible-jpeg.jpg");
  await pasteRequirements(page, "JPEG, max 2 MB");
  await expect(page.getByRole("link", { name: "Use original image" })).toBeVisible();
  await page.getByLabel("Choose an image").setInputFiles({
    name: "attack.svg",
    mimeType: "image/svg+xml",
    buffer: Buffer.from("<svg><script>document.body.textContent='unsafe'</script></svg>"),
  });
  await expect(page.getByRole("heading", { name: "Cannot satisfy these requirements" })).toBeVisible();
  await expect(page.getByText("SVG and HTML are never rendered")).toBeVisible();
  await expect(page.getByRole("link", { name: /Use original|Download/ })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Review target changes" })).toHaveCount(0);
  await expect(page.locator(".plan-summary")).toHaveCount(0);
  await expect(page.locator("main img")).toHaveCount(0);
});

test("crop approval is keyboard operable and required before adaptation", async ({ page }) => {
  await page.goto("/");
  await dropImage(page, "mismatch-png.png");
  await pasteRequirements(page, "PNG, exactly 8 x 4");
  await expect(page.getByRole("heading", { name: "Crop approval required" })).toBeVisible();
  const position = page.getByLabel(/crop position/i);
  await position.focus();
  await page.keyboard.press("Home");
  await expect(position).toHaveValue("0");
  const adapt = page.getByRole("button", { name: "Fix image" });
  await expect(adapt).toBeDisabled();
  const consent = page.getByLabel(/I approve removing/);
  await consent.focus();
  await page.keyboard.press("Space");
  await expect(consent).toBeChecked();
  await adapt.click();
  await expect(page.getByRole("heading", { name: "Image adapted and validated" })).toBeVisible();
});

test("active worker processing can be cancelled", async ({ page }) => {
  await page.goto("/");
  await dropImage(page, "mismatch-png.png");
  await pasteRequirements(page, "JPEG, exactly 4000 x 4000");
  await expect(page.getByRole("heading", { name: "Minimum changes ready" })).toBeVisible();
  await page.getByRole("button", { name: "Fix image" }).click();
  const cancel = page.getByRole("button", { name: "Cancel processing" });
  await expect(page.getByLabel("Choose an image")).toBeDisabled();
  await expect(page.locator(".drop-zone")).toHaveAttribute("aria-disabled", "true");
  await cancel.click();
  await expect(page.getByRole("heading", { name: "Processing cancelled" })).toBeVisible();
  await expect(page.getByText("No output was saved")).toBeVisible();
});

test("complete range and format alternatives survive compile, plan, and adaptation", async ({ page }) => {
  await page.goto("/");
  await dropImage(page, "transparent-png.png");
  await pasteRequirements(page, "JPEG or PNG, min 640 x 480, max 1920 x 1080, max 2 MB");
  await openEditor(page);
  await expect(page.getByLabel("JPEG")).toBeChecked();
  await expect(page.getByLabel("PNG")).toBeChecked();
  await expect(page.getByLabel("Minimum width")).toHaveValue("640");
  await expect(page.getByLabel("Maximum width")).toHaveValue("1920");
  await expect(page.getByLabel("Minimum height")).toHaveValue("480");
  await expect(page.getByLabel("Maximum height")).toHaveValue("1080");
  await expect(page.getByLabel("Maximum bytes")).toHaveValue("2000000");
  await expect(page.getByRole("heading", { name: "Minimum changes ready" })).toBeVisible();
  await expect(page.locator(".plan-summary li", { hasText: "Convert to" })).toHaveCount(0);
  await page.getByLabel("Maximum bytes").fill("1999999");
  await expect(page.locator(".checklist")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Fix image" })).toHaveCount(0);
  await page.getByRole("button", { name: "Review target changes" }).click();
  await page.getByRole("button", { name: "Fix image" }).click();
  await expect(page.getByRole("heading", { name: "Image adapted and validated" })).toBeVisible();
  await expect(page.locator(".checklist li")).toHaveCount(6);
  await expect(page.locator(".checklist li.fail, .checklist li.unknown")).toHaveCount(0);
  const download = page.getByRole("link", { name: "Download PNG" });
  await expect(download).toHaveAttribute("download", "transparent-png.fitifact.png");
  await expectRealDownload(page, download, "image/png", [0x89, 0x50, 0x4e, 0x47, 13, 10, 26, 10]);
});

test("requirements and target edits immediately invalidate stale contracts", async ({ page }) => {
  await page.goto("/");
  await dropImage(page, "compatible-jpeg.jpg");
  await pasteRequirements(page, "JPEG, max 2 MB");
  await expect(page.getByRole("link", { name: "Use original image" })).toBeVisible();
  await page.getByLabel("Rejection message or requirements").fill("JPEG, exactly 12.5 x 4");
  await expect(page.getByRole("link", { name: /Use original|Download/ })).toHaveCount(0);
  await expect(page.locator(".checklist")).toHaveCount(0);
  await expect(page.locator(".error-copy")).toBeVisible({ timeout: 15_000 });
});

test("oversized File is refused before main-thread arrayBuffer allocation", async ({ page }) => {
  await page.goto("/");
  await page.evaluate(() => {
    File.prototype.arrayBuffer = () => Promise.reject(new Error("main-thread arrayBuffer must not run"));
    const bytes = new Uint8Array(32 * 1024 * 1024 + 1);
    bytes.set([0xff, 0xd8, 0xff]);
    const transfer = new DataTransfer();
    transfer.items.add(new File([bytes], "oversized.jpg", { type: "image/jpeg" }));
    const input = document.querySelector<HTMLInputElement>("#image-file")!;
    input.files = transfer.files;
    input.dispatchEvent(new Event("change", { bubbles: true }));
  });
  await expect(page.getByRole("heading", { name: "Resource limit reached" })).toBeVisible();
  await expect(page.getByText(/byte local input limit/i)).toBeVisible();
  await expect(page.getByText("main-thread arrayBuffer must not run")).toHaveCount(0);
});

test("off-gate HEIC is explicit and does not load a cloud fallback", async ({ page }) => {
  test.skip(process.env.FITIFACT_HEIC_APPROVED !== "false", "off-gate behavior requires an explicit decoder-free build");
  await page.goto("/");
  const heic = Buffer.alloc(24);
  heic.writeUInt32BE(24, 0);
  heic.write("ftypheic", 4, "ascii");
  await page.getByLabel("Choose an image").setInputFiles({ name: "photo.heic", mimeType: "image/heic", buffer: heic });
  await expect(page.getByRole("heading", { name: "This is a phone photo this build cannot decode yet" })).toBeVisible();
});

test("approved HEIC fixture decodes and validates through the real worker", async ({ page }) => {
  test.skip(process.env.FITIFACT_HEIC_APPROVED === "false", "decoder-free build");
  await page.goto("/");
  await dropImage(page, "synthetic-single.heic");
  await pasteRequirements(page, "JPEG, max 2 MB");
  await expect(page.getByRole("heading", { name: "Minimum changes ready" })).toBeVisible();
  await page.getByRole("button", { name: "Fix image" }).click();
  await expect(page.getByRole("heading", { name: "Image adapted and validated" })).toBeVisible();
  await expect(page.locator(".checklist li.fail, .checklist li.unknown")).toHaveCount(0);
  const download = page.getByRole("link", { name: "Download JPG" });
  await expect(download).toHaveAttribute("download", "synthetic-single.fitifact.jpg");
  await expectRealDownload(page, download, "image/jpeg", [0xff, 0xd8, 0xff]);
});

test("still WebP adapts to JPEG", async ({ page }) => {
  await page.goto("/");
  await dropImage(page, "still-webp.webp");
  await pasteRequirements(page, "JPEG, max 2 MB");
  await expect(page.getByRole("heading", { name: "Minimum changes ready" })).toBeVisible();
  await page.getByRole("button", { name: "Fix image" }).click();
  await expect(page.getByRole("heading", { name: "Image adapted and validated" })).toBeVisible();
  const download = page.getByRole("link", { name: "Download JPG" });
  await expectRealDownload(page, download, "image/jpeg", [0xff, 0xd8, 0xff]);
});

test("TIFF BMP and still GIF adapt to JPEG", async ({ page }) => {
  await page.goto("/");
  for (const [name, label] of [
    ["still-tiff.tiff", "TIFF"],
    ["still-bmp.bmp", "BMP"],
    ["still-gif.gif", "GIF"],
  ] as const) {
    await dropImage(page, name);
    await expect(page.locator(".file-row")).toContainText(label);
    await pasteRequirements(page, "JPEG, max 2 MB");
    await expect(page.getByRole("heading", { name: "Minimum changes ready" })).toBeVisible();
    await page.getByRole("button", { name: "Fix image" }).click();
    await expect(page.getByRole("heading", { name: "Image adapted and validated" })).toBeVisible();
    await expectRealDownload(page, page.getByRole("link", { name: "Download JPG" }), "image/jpeg", [0xff, 0xd8, 0xff]);
  }
});

test("animated GIF requires first-frame consent", async ({ page }) => {
  await page.goto("/");
  await dropImage(page, "animated-gif.gif");
  await pasteRequirements(page, "JPEG, max 2 MB");
  await expect(page.getByRole("heading", { name: "First-frame approval required" })).toBeVisible();
  const adapt = page.getByRole("button", { name: "Fix image" });
  await expect(adapt).toBeDisabled();
  await page.getByLabel(/I approve keeping only the first frame/).check();
  await adapt.click();
  await expect(page.getByRole("heading", { name: "Image adapted and validated" })).toBeVisible();
});

test("JPG PNG or WebP paste can keep WebP", async ({ page }) => {
  await page.goto("/");
  await dropImage(page, "still-webp.webp");
  await pasteRequirements(page, "JPG, PNG, or WebP, max 2 MB");
  await openEditor(page);
  await expect(page.getByLabel("WebP")).toBeChecked();
  await expect(page.getByRole("heading", { name: "Already compatible" })).toBeVisible();
});

test("video and PDF drops refuse in one sentence", async ({ page }) => {
  await page.goto("/");
  const mp4 = Buffer.alloc(16);
  mp4.writeUInt32BE(16, 0);
  mp4.write("ftypisom", 4, "ascii");
  await page.getByLabel("Choose an image").setInputFiles({ name: "clip.mp4", mimeType: "video/mp4", buffer: mp4 });
  await expect(page.getByText("This is a video. The web app adapts images. The CLI remuxes and transcodes.")).toBeVisible();
  await page.getByLabel("Choose an image").setInputFiles({
    name: "doc.pdf",
    mimeType: "application/pdf",
    buffer: Buffer.from("%PDF-1.7"),
  });
  await expect(page.getByText("This is a PDF. The web app adapts images and does not convert documents.")).toBeVisible();
});

test("sidebar is closed on first paint and closes on Escape", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("button", { name: "Menu" })).toHaveAttribute("aria-expanded", "false");
  await page.getByRole("button", { name: "Menu" }).click();
  await expect(page.getByRole("dialog", { name: "Menu" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Menu" })).toHaveAttribute("aria-expanded", "true");
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Menu" })).toHaveAttribute("aria-expanded", "false");
});

test("clipboard image paste inspects the file", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator(".drop-zone")).toBeInViewport();
  const bytes = [...await readFile(path.join(fixtures, "compatible-jpeg.jpg"))];
  await page.evaluate((data) => {
    const file = new File([new Uint8Array(data)], "pasted.jpg", { type: "image/jpeg" });
    const dt = new DataTransfer();
    dt.items.add(file);
    const event = new Event("paste", { bubbles: true, cancelable: true });
    Object.defineProperty(event, "clipboardData", { value: dt, configurable: true });
    window.dispatchEvent(event);
  }, bytes);
  await expect(page.getByLabel("Rejection message or requirements")).toBeEnabled({ timeout: 30_000 });
  await expect(page.locator(".file-row")).toContainText("JPEG");
});

test("last-used target is applied after a new drop", async ({ page }) => {
  await page.goto("/");
  await dropImage(page, "mismatch-png.png");
  await pasteRequirements(page, "JPEG, max 2 MB");
  await expect(page.getByRole("heading", { name: "Minimum changes ready" })).toBeVisible();
  await page.reload();
  await dropImage(page, "mismatch-png.png");
  await expect(page.locator(".target-summary")).toContainText(/JPG|JPEG/i);
  await expect(page.getByRole("heading", { name: "Minimum changes ready" })).toBeVisible();
});
