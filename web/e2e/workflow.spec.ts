import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";
import path from "node:path";

const fixtures = path.resolve("../fixtures/image");

async function compile(page: Page, requirement: string) {
  await page.getByLabel("Upload instructions").fill(requirement);
  await page.getByRole("button", { name: "Review requirements" }).click();
  await expect(page.getByRole("heading", { name: "Requirements ready" })).toBeVisible();
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
});

test("already-compatible path preserves the original", async ({ page }) => {
  await page.goto("/");
  await compile(page, "JPEG, max 2 MB");
  await page.getByLabel("Choose an image").setInputFiles(path.join(fixtures, "compatible-jpeg.jpg"));
  await expect(page.getByRole("heading", { name: "Already compatible" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Use original image" })).toHaveAttribute("download", "compatible-jpeg.fitifact.jpg");
});

test("unsupported markup is refused and never rendered", async ({ page }) => {
  await page.goto("/");
  await compile(page, "PNG, max 2 MB");
  await page.getByLabel("Choose an image").setInputFiles({
    name: "attack.svg",
    mimeType: "image/svg+xml",
    buffer: Buffer.from("<svg><script>document.body.textContent='unsafe'</script></svg>"),
  });
  await expect(page.getByRole("heading", { name: "Cannot satisfy these requirements" })).toBeVisible();
  await expect(page.getByText("SVG and HTML are never rendered")).toBeVisible();
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
