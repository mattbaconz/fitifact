import { describe, expect, it } from "vitest";
import { errorCopy } from "./error-copy";

describe("consumer error copy", () => {
  it("maps engine codes to sentences and never returns the code", () => {
    expect(errorCopy("INPUT_INVALID", "fallback")).toBe(
      "Fitifact couldn't find a format, size, or dimension rule in that text.",
    );
    expect(errorCopy("INSPECTION_LIMIT", "fallback")).toContain("32 MiB");
    expect(errorCopy("UNSUPPORTED_HEIC", "fallback")).toContain("HEIC");
    expect(errorCopy("EXECUTION_CANCELLED", "fallback")).toBe("Stopped. Nothing was saved.");
    expect(errorCopy("INSPECTION_UNSUPPORTED", "SVG and HTML are never rendered.")).toBe(
      "SVG and HTML are never rendered.",
    );
    expect(errorCopy("INPUT_INVALID", "fallback")).not.toContain("INPUT_INVALID");
    expect(errorCopy("image.input_too_large", "image.input_too_large")).toContain("32 MiB");
    expect(errorCopy("image.decoded_too_large", "image.decoded_too_large")).toContain("24 megapixels");
    expect(errorCopy("INSPECTION_LIMIT", "image.input_too_large")).not.toContain("image.input_too_large");
  });
});
