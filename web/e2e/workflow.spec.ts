import { expect, test } from "@playwright/test";
import type { Locator, Page } from "@playwright/test";
import { readFile } from "node:fs/promises";
import path from "node:path";

const fixtures = path.resolve("../fixtures/image");

async function dropImage(page: Page, name: string) {
  await page.locator("input[type=file]").setInputFiles(path.join(fixtures, name));
  await expect(page.locator(".target-tabs button").filter({ hasText: /^Custom$/ })).toBeEnabled({ timeout: 30_000 });
  await expect(page.locator(".file-row")).toContainText(/JPEG|PNG|WebP|HEIC|TIFF|BMP|GIF/);
}

async function pasteRequirements(page: Page, requirement: string) {
  const summary = page.locator(".requirements-disclosure > summary");
  if (await summary.isVisible() && !(await page.locator(".requirements-disclosure").getAttribute("open") !== null)) await summary.click();
  await page.getByRole("button", { name: "Custom", exact: true }).click();
  await page.getByLabel("Rejection message or requirements").fill(requirement);
  await expect(page.getByText(/Understood:/)).toBeAttached();
  await expect(page.locator(".target-summary")).not.toHaveText("", { timeout: 15_000 });
}

async function openEditor(page: Page) {
  const summary = page.locator(".requirements-disclosure > summary");
  if (await summary.isVisible() && await page.locator(".requirements-disclosure").getAttribute("open") === null) await summary.click();
  const edit = page.getByRole("button", { name: "Edit requirements", exact: true });
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
  await expect(page.getByRole("heading", { name: "Make your file work there." })).toBeVisible();
  await expect(page.getByText(/Photos ·.*max 32 MiB or 24 megapixels/)).toBeVisible();
  await expect(page.locator(".drop-zone")).toBeInViewport();
  await expect(page.getByRole("button", { name: "Try a sample image" }).first()).toBeVisible();
  await expect(page.getByRole("contentinfo")).toContainText("Local · nothing is uploaded");
  await expect(page.getByRole("contentinfo").getByRole("link", { name: "Docs" })).toHaveAttribute("href", "/docs/");
  await expect(page.getByRole("link", { name: "GitHub" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Settings and navigation" })).toHaveAttribute("aria-expanded", "false");
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
  await page.getByRole("button", { name: "Adapt file" }).click();
  await expect(page.getByRole("heading", { name: "Adapted and verified" })).toBeVisible();
  await expect(page.getByText("extra undocumented rules", { exact: false })).toBeVisible();
  await expect(page.locator(".file-thumb")).toBeVisible();
  await expect(page.locator(".before-after figcaption", { hasText: "Before" })).toBeVisible();
  await expect(page.locator(".before-after figcaption", { hasText: "After" })).toBeVisible();
  const download = page.getByRole("link", { name: "Download JPG" });
  await expect(download).toHaveAttribute("download", "mismatch-png.fitifact.jpg");
  await expect(page.locator(".checklist li.fail, .checklist li.unknown")).toHaveCount(0);
  await expectRealDownload(page, download, "image/jpeg", [0xff, 0xd8, 0xff]);
  await openEditor(page);
  await page.getByLabel("Maximum size").fill("1999999");
  await expect(download).toHaveCount(0);
  await expect(page.locator(".checklist")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Adapt file" })).toHaveCount(0);
});

test("already-compatible path preserves the original", async ({ page }) => {
  await page.goto("/");
  await dropImage(page, "compatible-jpeg.jpg");
  await pasteRequirements(page, "JPEG, max 2 MB");
  await expect(page.getByRole("heading", { name: "Already compatible. Nothing changed." })).toBeVisible();
  await expect(page.locator(".intel-verified")).toContainText("already matches the destination");
  const download = page.getByRole("link", { name: "Use original" });
  await expect(download).toHaveAttribute("download", "compatible-jpeg.jpg");
  await expectRealDownload(
    page,
    download,
    "image/jpeg",
    [0xff, 0xd8, 0xff],
    await readFile(path.join(fixtures, "compatible-jpeg.jpg")),
  );
  await openEditor(page);
  await page.getByLabel("Maximum size").fill("1999999");
  await expect(download).toHaveCount(0);
  await expect(page.locator(".checklist")).toHaveCount(0);
  await page.getByRole("button", { name: "Review target changes" }).click();
  await expect(page.getByRole("link", { name: "Use original" })).toBeVisible();
});

test("a failed replacement keeps the previous source and names the rejected file", async ({ page }) => {
  await page.goto("/");
  await dropImage(page, "compatible-jpeg.jpg");
  await pasteRequirements(page, "JPEG, max 2 MB");
  await expect(page.getByRole("link", { name: "Use original" })).toBeVisible();
  await page.locator("input[type=file]").setInputFiles({
    name: "attack.svg",
    mimeType: "image/svg+xml",
    buffer: Buffer.from("<svg><script>document.body.textContent='unsafe'</script></svg>"),
  });
  await expect(page.getByText("attack.svg")).toBeVisible();
  await expect(page.getByText("SVG and HTML are never rendered")).toBeVisible();
  await expect(page.locator(".download-button")).toBeVisible();
  await expect(page.locator(".file-row")).toContainText("JPEG");
  await expect(page.getByLabel("Rejection message or requirements")).toHaveValue("JPEG, max 2 MB");
  await expect(page.locator(".file-thumb")).toBeVisible();
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
  const adapt = page.getByRole("button", { name: "Adapt file" });
  await expect(adapt).toBeDisabled();
  const consent = page.getByLabel(/I approve removing/);
  await consent.focus();
  await page.keyboard.press("Space");
  await expect(consent).toBeChecked();
  await adapt.click();
  await expect(page.getByRole("heading", { name: "Adapted and verified" })).toBeVisible();
});

test("active worker processing can be cancelled", async ({ page }) => {
  await page.goto("/");
  await dropImage(page, "mismatch-png.png");
  await pasteRequirements(page, "JPEG, exactly 4000 x 4000");
  await expect(page.getByRole("heading", { name: "Minimum changes ready" })).toBeVisible();
  await page.getByRole("button", { name: "Adapt file" }).click();
  const cancel = page.getByRole("button", { name: "Cancel processing" });
  await expect(page.locator("input[type=file]")).toBeDisabled();
  await expect(page.locator(".drop-zone")).toHaveAttribute("aria-disabled", "true");
  await cancel.click();
  await expect(page.getByRole("heading", { name: "Processing cancelled" })).toBeVisible();
  await expect(page.getByText("No output was saved")).toBeVisible();
  await page.getByRole("button", { name: "Retry", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Minimum changes ready" })).toBeVisible();
});

test("complete range and format alternatives survive compile, plan, and adaptation", async ({ page }) => {
  await page.goto("/");
  await dropImage(page, "transparent-png.png");
  await pasteRequirements(page, "JPEG or PNG, min 640 x 480, max 1920 x 1080, max 2 MB");
  await openEditor(page);
  await expect(page.getByRole("checkbox", { name: "JPEG", exact: true })).toBeChecked();
  await expect(page.getByLabel("PNG")).toBeChecked();
  await expect(page.getByLabel("Minimum width")).toHaveValue("640");
  await expect(page.getByLabel("Maximum width")).toHaveValue("1920");
  await expect(page.getByLabel("Minimum height")).toHaveValue("480");
  await expect(page.getByLabel("Maximum height")).toHaveValue("1080");
  await expect(page.getByLabel("Maximum size")).toHaveValue("2000000");
  await expect(page.getByRole("heading", { name: "Minimum changes ready" })).toBeVisible();
  await expect(page.locator(".plan-summary li", { hasText: "Convert to" })).toHaveCount(0);
  await page.getByLabel("Maximum size").fill("1999999");
  await expect(page.locator(".checklist")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Adapt file" })).toHaveCount(0);
  await page.getByRole("button", { name: "Review target changes" }).click();
  await page.getByRole("button", { name: "Adapt file" }).click();
  await expect(page.getByRole("heading", { name: "Adapted and verified" })).toBeVisible();
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
  await expect(page.getByRole("link", { name: "Use original" })).toBeVisible();
  await page.getByRole("button", { name: "Change requirements", exact: true }).click();
  await page.getByLabel("Rejection message or requirements").fill("JPEG, exactly 12.5 x 4");
  await expect(page.locator(".download-button")).toHaveCount(0);
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
  await expect(page.getByText(/larger than Fitifact can process in the browser/i)).toBeVisible();
  await expect(page.getByText("main-thread arrayBuffer must not run")).toHaveCount(0);
});

test("off-gate HEIC is explicit and does not load a cloud fallback", async ({ page }) => {
  test.skip(process.env.FITIFACT_HEIC_APPROVED !== "false", "off-gate behavior requires an explicit decoder-free build");
  await page.goto("/");
  const heic = Buffer.alloc(24);
  heic.writeUInt32BE(24, 0);
  heic.write("ftypheic", 4, "ascii");
  await page.locator("input[type=file]").setInputFiles({ name: "photo.heic", mimeType: "image/heic", buffer: heic });
  await expect(page.getByRole("heading", { name: "This is a phone photo this build cannot decode yet" })).toBeVisible();
});

test("approved HEIC fixture decodes and validates through the real worker", async ({ page }) => {
  test.skip(process.env.FITIFACT_HEIC_APPROVED === "false", "decoder-free build");
  await page.goto("/");
  await dropImage(page, "synthetic-single.heic");
  await pasteRequirements(page, "JPEG, max 2 MB");
  await expect(page.getByRole("heading", { name: "Minimum changes ready" })).toBeVisible();
  await page.getByRole("button", { name: "Adapt file" }).click();
  await expect(page.getByRole("heading", { name: "Adapted and verified" })).toBeVisible();
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
  await page.getByRole("button", { name: "Adapt file" }).click();
  await expect(page.getByRole("heading", { name: "Adapted and verified" })).toBeVisible();
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
    await page.getByRole("button", { name: "Adapt file" }).click();
    await expect(page.getByRole("heading", { name: "Adapted and verified" })).toBeVisible();
    await expectRealDownload(page, page.getByRole("link", { name: "Download JPG" }), "image/jpeg", [0xff, 0xd8, 0xff]);
  }
});

test("animated GIF requires first-frame consent", async ({ page }) => {
  await page.addInitScript(() => localStorage.setItem("fitifact.settings.v1", JSON.stringify({ fitifactSuffix: true, firstFrameConsentDefault: true })));
  await page.goto("/");
  await dropImage(page, "animated-gif.gif");
  await pasteRequirements(page, "JPEG, max 2 MB");
  await expect(page.getByRole("heading", { name: "First-frame approval required" })).toBeVisible();
  const adapt = page.getByRole("button", { name: "Adapt file" });
  await expect(adapt).toBeDisabled();
  await page.getByLabel(/I approve keeping only the first frame/).check();
  await adapt.click();
  await expect(page.getByRole("heading", { name: "Adapted and verified" })).toBeVisible();
  await dropImage(page, "animated-gif.gif");
  await expect(page.getByLabel(/I approve keeping only the first frame/)).not.toBeChecked();
  await expect(page.getByRole("button", { name: "Adapt file" })).toBeDisabled();
});

test("result exports contain actual report and constraint schemas", async ({ page }) => {
  await page.goto("/");
  await dropImage(page, "mismatch-png.png");
  await pasteRequirements(page, "JPEG, max 2 MB");
  await page.getByRole("button", { name: "Adapt file" }).click();
  await expect(page.getByRole("heading", { name: "Adapted and verified" })).toBeVisible();
  for (const [label, schema] of [["Download report JSON", "fitifact.check/v1"], ["Download constraints JSON", "fitifact.constraints/v1"]]) {
    const [download] = await Promise.all([page.waitForEvent("download"), page.getByRole("link", { name: label, exact: true }).click()]);
    const report = JSON.parse(await readFile((await download.path())!, "utf8"));
    expect(label.includes("constraints") ? report.schema : report.report.schema).toBe(schema);
    if (label.includes("report")) {
      expect(report.report.compatible).toBe(true);
      expect(report.output_artifact.image.format).toBe("jpeg");
      expect(report.plan.schema).toBe("fitifact.image-adapt-plan/v1");
    }
    if (label.includes("constraints")) expect(report.hard.some((rule: { value: unknown }) => rule.value === 2000000)).toBe(true);
  }
});

test("saved targets can be named, renamed, removed, restored, and reloaded", async ({ page }) => {
  await page.goto("/");
  await dropImage(page, "mismatch-png.png");
  await pasteRequirements(page, "JPEG, max 2 MB");
  await page.getByRole("button", { name: "Saved targets", exact: true }).click();
  await page.getByLabel("Save current requirements").fill("Application photo");
  await page.getByRole("button", { name: "Save target", exact: true }).click();
  await expect(page.getByRole("button", { name: "Application photo", exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Rename Application photo", exact: true }).click();
  await page.getByLabel("Rename target").fill("Portfolio photo");
  await page.getByRole("button", { name: "Save target", exact: true }).click();
  await page.getByRole("button", { name: "Remove Portfolio photo", exact: true }).click();
  await expect(page.getByRole("button", { name: "Portfolio photo", exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: "Undo removal" }).click();
  await page.reload();
  await dropImage(page, "mismatch-png.png");
  await page.getByRole("button", { name: "Saved targets", exact: true }).click();
  await page.getByRole("button", { name: "Portfolio photo", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Minimum changes ready" })).toBeVisible();
});

test("a corrupt replacement retains the valid source and original download", async ({ page }) => {
  await page.goto("/");
  await dropImage(page, "compatible-jpeg.jpg");
  await pasteRequirements(page, "JPEG, max 2 MB");
  await page.locator("input[type=file]").setInputFiles({ name: "broken.jpg", mimeType: "image/jpeg", buffer: Buffer.from([0xff, 0xd8, 0xff, 0, 0, 0, 0]) });
  await expect(page.getByText("broken.jpg", { exact: true })).toBeVisible();
  await expect(page.locator(".file-row")).toContainText("compatible-jpeg.jpg");
  await expectRealDownload(page, page.getByRole("link", { name: "Use original" }), "image/jpeg", [0xff, 0xd8, 0xff], await readFile(path.join(fixtures, "compatible-jpeg.jpg")));
});

test("JPG PNG or WebP paste can keep WebP", async ({ page }) => {
  await page.goto("/");
  await dropImage(page, "still-webp.webp");
  await pasteRequirements(page, "JPG, PNG, or WebP, max 2 MB");
  await openEditor(page);
  await expect(page.getByLabel("WebP")).toBeChecked();
  await expect(page.getByRole("heading", { name: "Already compatible. Nothing changed." })).toBeVisible();
});

test("video and PDF drops refuse in one sentence", async ({ page }) => {
  await page.goto("/");
  const mp4 = Buffer.alloc(16);
  mp4.writeUInt32BE(16, 0);
  mp4.write("ftypisom", 4, "ascii");
  await page.locator("input[type=file]").setInputFiles({ name: "clip.mp4", mimeType: "video/mp4", buffer: mp4 });
  await expect(page.getByText("This is a video. The web app adapts photos. Open it in the Fitifact desktop app.")).toBeVisible();
  await expect(page.locator(".reject-card").getByRole("link", { name: "Get the desktop app" })).toHaveAttribute("href", "/docs/#desktop");
  await page.locator("input[type=file]").setInputFiles({
    name: "doc.pdf",
    mimeType: "application/pdf",
    buffer: Buffer.from("%PDF-1.7"),
  });
  await expect(page.getByText("This is a PDF. Fitifact adapts photos, video, and sound files, not documents.")).toBeVisible();
  await expect(page.locator(".reject-card").getByRole("link", { name: "Get the desktop app" })).toHaveCount(0);
  await page.locator("input[type=file]").setInputFiles({
    name: "notes.doc",
    mimeType: "application/msword",
    buffer: Buffer.from([0xd0, 0xcf, 0x11, 0xe0, 0, 0, 0, 0]),
  });
  await expect(page.getByText("This is an Office document. Fitifact adapts photos, video, and sound files, not documents.")).toBeVisible();
});

test("sidebar is closed on first paint and closes on Escape", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("button", { name: "Settings and navigation" })).toHaveAttribute("aria-expanded", "false");
  await page.getByRole("button", { name: "Settings and navigation" }).click();
  await expect(page.getByRole("dialog", { name: "Settings", exact: true })).toBeVisible();
  await expect(page.getByRole("dialog", { name: "Settings", exact: true }).getByRole("link", { name: "Docs" })).toHaveAttribute("href", "/docs/");
  await expect(page.locator(".menu-button")).toHaveAttribute("aria-expanded", "true");
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Settings and navigation" })).toHaveAttribute("aria-expanded", "false");
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
  await expect(page.getByRole("button", { name: "Custom", exact: true })).toBeEnabled({ timeout: 30_000 });
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

test("inspect without a target shows the file and keeps Adapt file disabled", async ({ page }) => {
  await page.addInitScript(() => localStorage.clear());
  await page.goto("/");
  await dropImage(page, "compatible-jpeg.jpg");
  await expect(page.locator(".file-thumb")).toBeVisible();
  await expect(page.getByRole("heading", { name: "Your file is inspected." })).toBeVisible();
  await page.getByRole("button", { name: "Custom", exact: true }).click();
  await expect(page.getByPlaceholder("Or paste the exact rejection (size, format, dimensions).")).toBeVisible();
  await expect(page.getByRole("button", { name: "Adapt file" })).toBeDisabled();
});

test("idle drop offers a sample image without destination tiles", async ({ page }) => {
  await page.goto("/");
  await expect(page.locator(".format-chips")).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Make your file work there." })).toBeVisible();
  await page.getByRole("button", { name: "Try a sample image" }).first().click();
  await expect(page.locator(".file-row")).toContainText("PNG", { timeout: 30_000 });
  await expect(page.getByRole("heading", { name: "Where does it need to work?" })).toBeVisible();
  await expect(page.locator(".file-row")).toBeVisible();
});

test("paste leftover shows understood versus ignored rules", async ({ page }) => {
  await page.goto("/");
  await dropImage(page, "compatible-jpeg.jpg");
  await pasteRequirements(page, "JPEG, max 2 MB, keep faces centered");
  await expect(page.getByRole("heading", { name: "Already compatible. Nothing changed.", exact: true })).toBeVisible();
  await expect(page.locator(".status-card").getByText(/Not used: keep faces centered/)).toBeVisible();
  const summary = page.locator(".requirements-disclosure > summary");
  if (await summary.isVisible() && await page.locator(".requirements-disclosure").getAttribute("open") === null) await summary.click();
  await expect(page.getByText(/Understood:/)).toBeVisible();
  await expect(page.locator(".destination-card").getByText(/Not used: keep faces centered/)).toBeVisible();
});

test("a Discord destination chip adapts a JPEG under the documented image cap", async ({ page }) => {
  await page.goto("/");
  await dropImage(page, "compatible-jpeg.jpg");
  await page.locator(".destination-chips").getByRole("button", { name: /Discord/ }).click();
  await expect(page.getByText("Using Discord free upload (the cap you set).")).toBeAttached({ timeout: 30_000 });
  await expect(page.locator(".file-row")).toBeVisible();
  await expect(page.locator(".target-summary")).toContainText(/20(\.00)?\s*MB/i);
  await expect(page.getByRole("heading", { name: /Already compatible. Nothing changed.|Minimum changes ready/ })).toBeVisible();
  await expect(page.getByRole("heading", { name: /Already compatible. Nothing changed.|Minimum changes ready/ })).toBeInViewport();
  const destBox = await page.locator(".requirements-column").boundingBox();
  const resultBox = await page.getByRole("heading", { name: /Already compatible. Nothing changed.|Minimum changes ready/ }).boundingBox();
  expect(destBox && resultBox && resultBox.y > destBox.y).toBeTruthy();
  await expect(page.getByRole("button", { name: "Destination still said no? Paste the new message." })).toBeVisible();
});

test("a GitHub destination chip plans a sample PNG under the documented image cap", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: "Try a sample image" }).first().click();
  await expect(page.locator(".file-row")).toContainText("PNG", { timeout: 30_000 });
  await page.getByRole("button", { name: /GitHub/ }).click();
  await expect(page.getByText("Using GitHub comments.")).toBeAttached({ timeout: 30_000 });
  await expect(page.locator(".file-row")).toBeVisible();
  await expect(page.locator(".target-summary")).toContainText(/10(\.00)?\s*MB/i);
  await expect(page.getByRole("heading", { name: /Already compatible. Nothing changed.|Minimum changes ready/ })).toBeVisible();
});

test("Pages inspect grid has no Web video chip", async ({ page }) => {
  await page.goto("/");
  await dropImage(page, "compatible-jpeg.jpg");
  await expect(page.getByRole("heading", { name: "Where does it need to work?" })).toBeVisible();
  await expect(page.getByRole("button", { name: /Web video/ })).toHaveCount(0);
  await expect(page.locator(".destination-chips").getByRole("button", { name: /Discord/ })).toBeVisible();
});

test("docs explain photos, desktop, and no-op without leaving the site", async ({ page }) => {
  await page.goto("/docs/");
  await expect(page.getByRole("heading", { name: "Getting started", exact: true })).toBeVisible();
  await expect(page.getByRole("heading", { name: /^Desktop/, level: 2 })).toBeVisible();
  await expect(page.getByText(/nothing changes/).first()).toBeVisible();
  if (!(await page.getByRole("link", { name: /Open Fitifact/ }).count())) await page.getByRole("button", { name: "Open Sidebar" }).click();
  await expect(page.getByRole("link", { name: /Open Fitifact/ }).first()).toHaveAttribute("href", "/");
  await page.goto("/docs/desktop-guide/");
  await expect(page.getByText(/CLI builds/)).toBeVisible();
  await expect(page.getByText(/Locally built installers are unsigned/)).toBeVisible();
});

test("documentation search runs locally and opens a refreshable section", async ({ page }) => {
  const remote: string[] = [];
  page.on("request", request => { if (new URL(request.url()).origin !== "http://127.0.0.1:4173") remote.push(request.url()); });
  await page.goto("/docs/");
  const trigger = page.getByRole("button", { name: /^(Search|Open Search)/ }).first();
  await trigger.click();
  await page.getByRole("textbox", { name: "Search", exact: true }).fill("remux");
  await page.getByRole("button", { name: "Remux a video", exact: true }).click();
  await expect(page).toHaveURL(/\/docs\/examples\/#remux-a-video/);
  await page.reload();
  await expect(page.getByRole("heading", { name: /Remux a video/, level: 2 })).toBeVisible();
  expect(remote).toEqual([]);
});

test("last-used profile restores after a new drop", async ({ page }) => {
  await page.goto("/");
  await dropImage(page, "compatible-jpeg.jpg");
  await page.locator(".destination-chips").getByRole("button", { name: /Discord/ }).click();
  await expect(page.getByText("Using Discord free upload (the cap you set).")).toBeAttached({ timeout: 30_000 });
  await page.reload();
  await dropImage(page, "compatible-jpeg.jpg");
  await expect(page.getByText("Same as last time: Discord free (the cap you set).")).toBeAttached({ timeout: 30_000 });
  await expect(page.locator(".file-row")).toBeVisible();
  await expect(page.locator(".target-summary")).toContainText(/20(\.00)?\s*MB/i);
  await expect(page.getByRole("heading", { name: /Already compatible. Nothing changed.|Minimum changes ready/ })).toBeVisible();
});

test("a ?for= link picks the destination for the first file only", async ({ page }) => {
  await page.goto("/?for=gmail");
  await dropImage(page, "compatible-jpeg.jpg");
  await expect(page.getByText("Using Gmail attachment.")).toBeAttached({ timeout: 30_000 });
  await expect(page.locator(".target-summary")).toContainText(/25(\.00)?\s*MB/i);
  const summary = page.locator(".requirements-disclosure > summary");
  if (await summary.isVisible() && await page.locator(".requirements-disclosure").getAttribute("open") === null) await summary.click();
  await page.locator(".destination-chips").getByRole("button", { name: /Discord/ }).click();
  await expect(page.locator(".target-summary")).toContainText(/20(\.00)?\s*MB/i, { timeout: 30_000 });
  await dropImage(page, "compatible-jpeg.jpg");
  await expect(page.getByRole("heading", { name: /Already compatible. Nothing changed.|Minimum changes ready/ })).toBeVisible({ timeout: 30_000 });
  await expect(page.locator(".target-summary")).toContainText(/20(\.00)?\s*MB/i);
  await expect(page.getByText("Using Gmail attachment.")).toHaveCount(0);
});

test("sample scenarios replace remembered requirements and require crop approval", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: /Try an unchanged file/ }).click();
  await expect(page.locator(".download-button")).toHaveText("Use original");
  await page.getByRole("link", { name: "Fitifact home", exact: true }).click();
  await page.getByRole("button", { name: /Try a format change/ }).click();
  await expect(page.getByRole("button", { name: "Adapt file", exact: true })).toBeEnabled();
  await expect(page.locator(".target-summary")).toContainText("JPG");
  await expect(page.locator(".download-button")).toHaveCount(0);
  await page.getByRole("link", { name: "Fitifact home", exact: true }).click();
  await page.getByRole("button", { name: /Try a crop/ }).click();
  const consent = page.getByRole("checkbox", { name: /I approve removing/ });
  await expect(consent).not.toBeChecked();
  await expect(page.getByRole("button", { name: "Adapt file", exact: true })).toBeDisabled();
  await consent.check();
  await page.getByRole("button", { name: "Adapt file", exact: true }).click();
  await expect(page.locator(".download-button")).toHaveText("Download JPG");
  await expect(page.locator(".output-facts")).toContainText("480×480");
  await expect(page.getByRole("heading", { name: "Adapted and verified", exact: true })).toBeFocused();
  await expect(page.locator(".download-button")).toBeInViewport();
});
