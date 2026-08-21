import { expect, test } from "@playwright/test";
import type { Locator, Page } from "@playwright/test";
import { readFile } from "node:fs/promises";
import path from "node:path";

const fixtures = path.resolve("../fixtures/image");

async function compile(page: Page, requirement: string) {
  await page.getByLabel("Upload instructions").fill(requirement);
  await page.getByRole("button", { name: "Review requirements" }).click();
  await expect(page.getByRole("heading", { name: "Ready for an image" })).toBeVisible();
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

test("happy path adapts PNG to JPEG and exposes a validated download", async ({ page }) => {
  await page.goto("/");
  await compile(page, "JPEG, max 2 MB");
  await page.getByLabel("Choose an image").setInputFiles(path.join(fixtures, "mismatch-png.png"));
  await expect(page.getByRole("heading", { name: "Minimum changes ready" })).toBeVisible();
  await page.getByRole("button", { name: "Adapt and validate" }).click();
  await expect(page.getByRole("heading", { name: "Image adapted and validated" })).toBeVisible();
  await expect(page.getByText("validated against the requirements you confirmed", { exact: false })).toBeVisible();
  const download = page.getByRole("link", { name: "Download JPG" });
  await expect(download).toHaveAttribute("download", "mismatch-png.fitifact.jpg");
  await expect(page.locator(".checklist li.fail, .checklist li.unknown")).toHaveCount(0);
  await expectRealDownload(page, download, "image/jpeg", [0xff, 0xd8, 0xff]);
  await page.getByLabel("Maximum bytes").fill("1999999");
  await expect(download).toHaveCount(0);
  await expect(page.locator(".checklist")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Adapt and validate" })).toHaveCount(0);
});

test("already-compatible path preserves the original", async ({ page }) => {
  await page.goto("/");
  await compile(page, "JPEG, max 2 MB");
  await page.getByLabel("Choose an image").setInputFiles(path.join(fixtures, "compatible-jpeg.jpg"));
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
  await page.getByLabel("Maximum bytes").fill("1999999");
  await expect(download).toHaveCount(0);
  await expect(page.locator(".checklist")).toHaveCount(0);
  await page.getByRole("button", { name: "Review target changes" }).click();
  await expect(page.getByRole("link", { name: "Use original image" })).toBeVisible();
});

test("a failed replacement clears the previous source and is never rendered", async ({ page }) => {
  await page.goto("/");
  await compile(page, "JPEG, max 2 MB");
  await page.getByLabel("Choose an image").setInputFiles(path.join(fixtures, "compatible-jpeg.jpg"));
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
  await expect(page.locator("img")).toHaveCount(0);
});

test("crop approval is keyboard operable and required before adaptation", async ({ page }) => {
  await page.goto("/");
  await compile(page, "PNG, exactly 8 x 4");
  await page.getByLabel("Choose an image").setInputFiles(path.join(fixtures, "mismatch-png.png"));
  await expect(page.getByRole("heading", { name: "Crop approval required" })).toBeVisible();
  const position = page.getByLabel(/crop position/i);
  await position.focus();
  await page.keyboard.press("Home");
  await expect(position).toHaveValue("0");
  const adapt = page.getByRole("button", { name: "Adapt and validate" });
  await expect(adapt).toBeDisabled();
  await page.getByLabel(/I approve removing/).check();
  await adapt.click();
  await expect(page.getByRole("heading", { name: "Image adapted and validated" })).toBeVisible();
});

test("active worker processing can be cancelled", async ({ page }) => {
  await page.goto("/");
  await compile(page, "JPEG, exactly 4000 x 4000");
  await page.getByLabel("Choose an image").setInputFiles(path.join(fixtures, "mismatch-png.png"));
  await expect(page.getByRole("heading", { name: "Minimum changes ready" })).toBeVisible();
  await page.getByRole("button", { name: "Adapt and validate" }).click();
  const cancel = page.getByRole("button", { name: "Cancel processing" });
  await expect(page.getByLabel("JPEG")).toBeDisabled();
  await expect(page.locator(".drop-zone")).toHaveAttribute("aria-disabled", "true");
  await expect(page.getByLabel("Choose an image")).toBeDisabled();
  await cancel.click();
  await expect(page.getByRole("heading", { name: "Processing cancelled" })).toBeVisible();
  await expect(page.getByText("No output was saved")).toBeVisible();
});

test("complete range and format alternatives survive compile, plan, and adaptation", async ({ page }) => {
  await page.goto("/");
  await compile(page, "JPEG or PNG, min 640 x 480, max 1920 x 1080, max 2 MB");
  await expect(page.getByLabel("JPEG")).toBeChecked();
  await expect(page.getByLabel("PNG")).toBeChecked();
  await expect(page.getByLabel("Minimum width")).toHaveValue("640");
  await expect(page.getByLabel("Maximum width")).toHaveValue("1920");
  await expect(page.getByLabel("Minimum height")).toHaveValue("480");
  await expect(page.getByLabel("Maximum height")).toHaveValue("1080");
  await expect(page.getByLabel("Maximum bytes")).toHaveValue("2000000");
  await page.getByLabel("Choose an image").setInputFiles(path.join(fixtures, "transparent-png.png"));
  await expect(page.locator(".plan-summary p", { hasText: "Proposed:" })).toContainText("PNG");
  await page.getByLabel("Maximum bytes").fill("1999999");
  await expect(page.locator(".plan-summary")).toHaveCount(0);
  await expect(page.locator(".checklist")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Adapt and validate" })).toHaveCount(0);
  await page.getByRole("button", { name: "Review target changes" }).click();
  await expect(page.locator(".plan-summary p", { hasText: "Proposed:" })).toContainText("PNG");
  await page.getByRole("button", { name: "Adapt and validate" }).click();
  await expect(page.getByRole("heading", { name: "Image adapted and validated" })).toBeVisible();
  await expect(page.locator(".checklist li")).toHaveCount(6);
  await expect(page.locator(".checklist li.fail, .checklist li.unknown")).toHaveCount(0);
  const download = page.getByRole("link", { name: "Download PNG" });
  await expect(download).toHaveAttribute("download", "transparent-png.fitifact.png");
  await expectRealDownload(page, download, "image/png", [0x89, 0x50, 0x4e, 0x47, 13, 10, 26, 10]);
});

test("requirements and target edits immediately invalidate stale contracts", async ({ page }) => {
  await page.goto("/");
  await compile(page, "JPEG, max 2 MB");
  await page.getByLabel("Choose an image").setInputFiles(path.join(fixtures, "compatible-jpeg.jpg"));
  await expect(page.getByRole("link", { name: "Use original image" })).toBeVisible();
  await page.getByLabel("Upload instructions").fill("JPEG, exactly 12.5 x 4");
  await expect(page.getByRole("link", { name: /Use original|Download/ })).toHaveCount(0);
  await expect(page.locator(".checklist")).toHaveCount(0);
  await expect(page.getByLabel("Choose an image")).toBeDisabled();
  await expect(page.locator(".target-form")).toHaveCount(0);
  await page.getByRole("button", { name: "Review requirements" }).click();
  await expect(page.getByText("INPUT_INVALID")).toBeVisible();
  await expect(page.locator(".target-form")).toHaveCount(0);
});

test("oversized File is refused before main-thread arrayBuffer allocation", async ({ page }) => {
  await page.goto("/");
  await compile(page, "JPEG, max 2 MB");
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
  await expect(page.getByText("INSPECTION_LIMIT")).toBeVisible();
  await expect(page.getByText("main-thread arrayBuffer must not run")).toHaveCount(0);
});

test("off-gate HEIC is explicit and does not load a cloud fallback", async ({ page }) => {
  test.skip(process.env.FITIFACT_HEIC_APPROVED === "true", "off-gate behavior requires the default build");
  await page.goto("/");
  await compile(page, "JPEG, max 2 MB");
  const heic = Buffer.alloc(24);
  heic.writeUInt32BE(24, 0);
  heic.write("ftypheic", 4, "ascii");
  await page.getByLabel("Choose an image").setInputFiles({ name: "photo.heic", mimeType: "image/heic", buffer: heic });
  await expect(page.getByRole("heading", { name: "HEIC is unsupported in this build" })).toBeVisible();
  await expect(page.getByText("has not approved the optional local decoder")).toBeVisible();
});

test("approved HEIC fixture decodes and validates through the real worker", async ({ page }) => {
  test.skip(process.env.FITIFACT_HEIC_APPROVED !== "true", "approved decoder build only");
  await page.goto("/");
  await compile(page, "JPEG, max 2 MB");
  await page.getByLabel("Choose an image").setInputFiles(path.join(fixtures, "synthetic-single.heic"));
  await expect(page.getByRole("heading", { name: "Minimum changes ready" })).toBeVisible();
  await page.getByRole("button", { name: "Adapt and validate" }).click();
  await expect(page.getByRole("heading", { name: "Image adapted and validated" })).toBeVisible();
  await expect(page.locator(".checklist li.fail, .checklist li.unknown")).toHaveCount(0);
  const download = page.getByRole("link", { name: "Download JPG" });
  await expect(download).toHaveAttribute("download", "synthetic-single.fitifact.jpg");
  await expectRealDownload(page, download, "image/jpeg", [0xff, 0xd8, 0xff]);
});
