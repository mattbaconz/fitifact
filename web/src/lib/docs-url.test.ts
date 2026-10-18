import { describe, expect, it } from "vitest";
import { PUBLIC_DOCS_URL, desktopGuideHref, needsDesktopHandoff, publicDocsHref } from "./docs-url";

describe("public docs href", () => {
  it("uses the local docs folder for every base path", () => {
    expect(publicDocsHref("/fitifact/")).toBe("/fitifact/docs/");
    expect(publicDocsHref("/")).toBe("/docs/");
    expect(desktopGuideHref("/")).toBe("/docs/#desktop");
    expect(PUBLIC_DOCS_URL).toContain("/fitifact/docs/");
  });

  it("offers a desktop handoff only when the refuse copy points there", () => {
    expect(needsDesktopHandoff("This is a video. The web app adapts photos. Open it in the Fitifact desktop app.")).toBe(true);
    expect(needsDesktopHandoff("This is a PDF. Fitifact adapts photos, video, and sound files, not documents.")).toBe(false);
  });
});
