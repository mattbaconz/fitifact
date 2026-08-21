import { expect, test } from "@playwright/test";
import type { Locator, Page } from "@playwright/test";
import { readFile } from "node:fs/promises";
import path from "node:path";

const fixtures = path.resolve("../fixtures/image");

async function compile(page: Page, requirement: string) {
  await page.getByLabel("Upload instructions").fill(requirement);
  await page.getByRole("button", { name: "Review requirements" }).click();
  await expect(page.getByRole("heading", { name: "Requirements ready" })).toBeVisible();
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
  const download = page.getByRole("link", { name: "Download JPG" });
  await expect(download).toHaveAttribute("download", "mismatch-png.fitifact.jpg");
  await expect(page.locator(".checklist li.fail, .checklist li.unknown")).toHaveCount(0);
  const href = await download.getAttribute("href");
  await page.getByLabel("Maximum bytes").fill("1999999");
  await expect(download).toHaveAttribute("href", href!);
  await expectRealDownload(page, download, "image/jpeg", [0xff, 0xd8, 0xff]);
});

test("already-compatible path preserves the original", async ({ page }) => {
  await page.goto("/");
  await compile(page, "JPEG, max 2 MB");
  await page.getByLabel("Choose an image").setInputFiles(path.join(fixtures, "compatible-jpeg.jpg"));
  await expect(page.getByRole("heading", { name: "Already compatible" })).toBeVisible();
  const download = page.getByRole("link", { name: "Use original image" });
  await expect(download).toHaveAttribute("download", "compatible-jpeg.fitifact.jpg");
  const href = await download.getAttribute("href");
  await page.getByLabel("Maximum bytes").fill("1999999");
  await expect(download).toHaveAttribute("href", href!);
  await expectRealDownload(
    page,
    download,
    "image/jpeg",
    [0xff, 0xd8, 0xff],
    await readFile(path.join(fixtures, "compatible-jpeg.jpg")),
  );
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
  await expect(page.getByLabel("Format")).toBeDisabled();
  await expect(page.locator(".drop-zone")).toHaveAttribute("aria-disabled", "true");
  await expect(page.getByLabel("Choose an image")).toBeDisabled();
  await cancel.click();
  await expect(page.getByRole("heading", { name: "Processing cancelled" })).toBeVisible();
  await expect(page.getByText("No output was saved")).toBeVisible();
});

test("off-gate HEIC is explicit and does not load a cloud fallback", async ({ page }) => {
  await page.goto("/");
  await compile(page, "JPEG, max 2 MB");
  const heic = Buffer.alloc(24);
  heic.writeUInt32BE(24, 0);
  heic.write("ftypheic", 4, "ascii");
  await page.getByLabel("Choose an image").setInputFiles({ name: "photo.heic", mimeType: "image/heic", buffer: heic });
  await expect(page.getByRole("heading", { name: "HEIC is unsupported in this build" })).toBeVisible();
  await expect(page.getByText("has not approved the optional local decoder")).toBeVisible();
});
