import { expect, test } from "@playwright/test";
import { expectRealDownload, JPEG } from "./support";

test("a full journey stays on this origin, and the CSP refuses other origins", async ({ page, context, baseURL }) => {
  const origin = new URL(baseURL!).origin;
  const offOrigin = (url: URL) => url.protocol !== "blob:" && url.protocol !== "data:" && url.origin !== origin;
  const requested: string[] = [];
  const reachedNetwork: string[] = [];
  page.on("request", (request) => {
    if (offOrigin(new URL(request.url()))) requested.push(request.url());
  });
  // Anything that gets past the page's own policy is recorded and aborted, so it never leaves the machine.
  await context.route(offOrigin, (route) => {
    reachedNetwork.push(route.request().url());
    return route.abort();
  });

  await page.goto("/");
  await page.getByRole("button", { name: /Try a format change/ }).click();
  await expectRealDownload(page, page.getByRole("button", { name: "Adapt and download" }), "image/jpeg", JPEG);
  await expect(page.getByRole("heading", { name: "Adapted and verified" })).toBeVisible();
  await page.goto("/docs/");
  await expect(page.getByRole("heading", { name: "Getting started", exact: true })).toBeVisible();
  expect(requested).toEqual([]);

  await page.goto("/");
  const probe = await page.evaluate(async () => {
    const refused: string[] = [];
    document.addEventListener("securitypolicyviolation", (event) => refused.push(event.effectiveDirective));
    const fetched = await fetch("https://csp-probe.invalid/").then(() => "sent", () => "refused");
    const image = new Image();
    const settled = new Promise((resolve) => image.addEventListener("error", resolve, { once: true }));
    image.src = "https://csp-probe.invalid/probe.png";
    await settled;
    await new Promise((resolve) => setTimeout(resolve, 100));
    return { fetched, refused };
  });
  expect(probe.fetched).toBe("refused");
  expect(probe.refused).toEqual(expect.arrayContaining(["connect-src", "img-src"]));
  expect(reachedNetwork).toEqual([]);
});
