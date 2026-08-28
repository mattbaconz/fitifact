import { describe, expect, it } from "vitest";
import { PUBLIC_DOCS_URL, publicDocsHref } from "./docs-url";

describe("public docs href", () => {
  it("uses the Pages docs folder when the app is served under /fitifact/", () => {
    expect(publicDocsHref("/fitifact/")).toBe("/fitifact/docs/");
  });

  it("uses the canonical Pages URL outside that base path", () => {
    expect(publicDocsHref("/")).toBe(PUBLIC_DOCS_URL);
  });
});
