import { expect, test } from "@playwright/test";
import path from "node:path";
import { requestLog, videoTools, whenQuiet } from "./support";

const media = path.resolve("../fixtures/media");

test("/discord/ has its own head, the guide, and the tier picker", async ({ page }) => {
  const errors: string[] = [];
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/discord/");
  await expect(page).toHaveTitle("Discord video compressor: fit 20 MB free, 50 MB Nitro Basic | Fitifact");
  await expect(page.locator('link[rel="canonical"]')).toHaveAttribute("href", "https://fitifact.com/discord/");
  await expect(page.locator('meta[property="og:url"]')).toHaveAttribute("content", "https://fitifact.com/discord/");
  await expect(page.locator('meta[name="description"]')).toHaveAttribute("content", /20 MB free limit/);
  await expect(page.getByRole("heading", { level: 1, name: "Fit a video for Discord" })).toBeVisible();
  await expect(page.getByRole("radio", { name: /^Free/ })).toBeChecked();
  // The static guide moves inside the app's main, above the footer.
  await expect(page.locator("main #page-guide")).toHaveCount(1);
  await expect(page.locator(".limits-table")).toContainText("20,000,000 bytes");
  await expect(page.getByRole("heading", { name: "Why did my 18 MB clip fail?" })).toBeVisible();
  // The copy above the table says 18 tools; keep the rows in step with it.
  await expect(page.locator(".audit-table tbody tr")).toHaveCount(18);
  expect(errors).toEqual([]);
});

test("/discord/ fits a clip for the picked tier with the network off", async ({ page, context, browserName }) => {
  test.skip(browserName === "firefox", "Firefox does not report module imports inside a worker as page requests.");
  test.skip(browserName === "webkit", "Playwright's WebKit offline emulation fails reads of the chosen file itself.");
  const requested = requestLog(page);
  const quiet = whenQuiet(page);
  await page.goto("/discord/");
  await page.getByRole("radio", { name: /^Nitro Basic/ }).check();
  await expect.poll(() => videoTools(requested), { timeout: 30_000 }).toBe(true);
  await quiet();
  await context.setOffline(true);
  await page.locator("input[type=file]").setInputFiles(path.join(media, "compatible-h264-aac.mp4"));
  await expect(page.getByRole("heading", { name: /Already compatible|Minimum changes ready/ })).toBeVisible({
    timeout: 30_000,
  });
  // On mobile this line sits in the requirements panel, collapsed once a result shows.
  await expect(page.getByText("Using Discord Nitro Basic upload (the cap you set).")).toBeAttached();
});

test("robots.txt and sitemap.xml list the pages", async ({ request }) => {
  const robots = await request.get("/robots.txt");
  expect(robots.status()).toBe(200);
  expect(await robots.text()).toContain("Sitemap: https://fitifact.com/sitemap.xml");
  const sitemap = await request.get("/sitemap.xml");
  expect(sitemap.status()).toBe(200);
  for (const url of ["https://fitifact.com/", "https://fitifact.com/discord/", "https://fitifact.com/docs/"]) {
    expect(await sitemap.text()).toContain(`<loc>${url}</loc>`);
  }
});
