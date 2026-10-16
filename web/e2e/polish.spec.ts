import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { writeFile } from "node:fs/promises";

for (const theme of ["dark", "light"]) {
  test(`documentation code is readable and copy failures recover in ${theme}`, async ({ page }, testInfo) => {
    test.skip(testInfo.project.name !== "chromium-desktop", "The workflow matrix covers every browser.");
    await page.addInitScript(value => {
      localStorage.setItem("fitifact.theme", value);
      Object.defineProperty(navigator, "clipboard", { configurable: true, value: {
        writeText: async () => { throw new Error("Clipboard unavailable"); },
      } });
    }, theme);
    await page.goto("/docs/cli/");
    await expect(page.getByRole("heading", { name: "CLI", exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Copy code", exact: true }).first().click();
    await expect(page.getByText("Copy unavailable. Select the code and copy it manually.")).toBeVisible();
    expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations).toEqual([]);
    await page.screenshot({ path: testInfo.outputPath("cli.png"), fullPage: true });
  });
}

for (const width of [360, 390, 768, 1280, 1440]) {
  for (const theme of ["dark", "light"]) {
    test(`polish ${width}px ${theme}: local sample, reachable result, accessible docs`, async ({
      page,
    }, testInfo) => {
      test.skip(
        testInfo.project.name !== "chromium-desktop",
        "Dedicated size/theme audit; the workflow suite covers every browser.",
      );
      await page.setViewportSize({ width, height: width < 500 ? 844 : 1000 });
      await page.emulateMedia({ reducedMotion: "reduce" });
      await page.addInitScript(
        (value) => localStorage.setItem("fitifact.theme", value),
        theme,
      );
      const remote: string[] = [];
      const failures: string[] = [];
      page.on("request", (request) => {
        if (
          !request.url().startsWith("blob:") &&
          new URL(request.url()).origin !== "http://127.0.0.1:4173"
        )
          remote.push(request.url());
        if (!["GET", "HEAD"].includes(request.method()))
          failures.push(`Unexpected ${request.method()} ${request.url()}`);
      });
      page.on("pageerror", (error) => failures.push(error.message));
      await page.goto("/");
      await expect(
        page.getByRole("heading", { name: "Make your file work there." }),
      ).toBeVisible();
      await page.screenshot({
        path: testInfo.outputPath("start.png"),
        fullPage: true,
      });
      const initial = await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
        .analyze();
      expect(initial.violations).toEqual([]);
      await page.getByRole("button", { name: /Try an unchanged file/ }).click();
      const download = page.getByRole("link", {
        name: "Use original",
        exact: true,
      });
      await expect(download).toBeVisible();
      await expect(download).toHaveAttribute("download", "sample-640x480.png");
      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= innerWidth,
        ),
      ).toBe(true);
      await expect(download).toBeInViewport();
      await expect(page.locator(".before-after")).toHaveCount(0);
      await page.screenshot({
        path: testInfo.outputPath("result.png"),
        fullPage: true,
      });
      const result = await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
        .analyze();
      expect(result.violations).toEqual([]);
      await page
        .getByRole("button", { name: "Settings and navigation" })
        .click();
      await expect(
        page.getByRole("dialog", { name: "Settings", exact: true }),
      ).toBeVisible();
      await page.keyboard.press("Escape");
      await expect(
        page.getByRole("button", { name: "Settings and navigation" }),
      ).toBeFocused();
      await page.goto("/docs/");
      await expect(
        page.getByRole("heading", { name: "Getting started", exact: true }),
      ).toBeVisible();
      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= innerWidth,
        ),
      ).toBe(true);
      await page.screenshot({
        path: testInfo.outputPath("docs.png"),
        fullPage: true,
      });
      const docs = await new AxeBuilder({ page })
        .withTags(["wcag2a", "wcag2aa", "wcag21aa"])
        .analyze();
      await writeFile(
        testInfo.outputPath("accessibility.json"),
        JSON.stringify(
          {
            start: initial.violations,
            result: result.violations,
            docs: docs.violations,
          },
          null,
          2,
        ),
      );
      expect(docs.violations).toEqual([]);
      expect(remote).toEqual([]);
      expect(failures).toEqual([]);
    });
  }
}

test("200% zoom keeps the working result usable", async ({
  page,
}, testInfo) => {
  test.skip(testInfo.project.name !== "chromium-desktop");
  await page.setViewportSize({ width: 1280, height: 1000 });
  await page.goto("/");
  await page.getByRole("button", { name: /Try an unchanged file/ }).click();
  await expect(page.getByRole("link", { name: "Use original" })).toBeVisible();
  await page.evaluate(() => {
    document.documentElement.style.zoom = "2";
  });
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page
    .getByRole("link", { name: "Use original" })
    .scrollIntoViewIfNeeded();
  await expect(
    page.getByRole("link", { name: "Use original" }),
  ).toBeInViewport();
  await page.screenshot({
    path: testInfo.outputPath("zoom-200.png"),
    fullPage: true,
  });
});
