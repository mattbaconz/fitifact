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
    expect(errorCopy("PROVIDER_MISSING", "fallback")).toContain("desktop app ships FFmpeg");
    expect(errorCopy("INSPECTION_UNSUPPORTED", "SVG and HTML are never rendered.")).toBe(
      "SVG and HTML are never rendered.",
    );
    expect(errorCopy("INPUT_INVALID", "fallback")).not.toContain("INPUT_INVALID");
    expect(errorCopy("image.input_too_large", "image.input_too_large")).toContain("32 MiB");
    expect(errorCopy("image.decoded_too_large", "image.decoded_too_large")).toContain("24 megapixels");
    expect(errorCopy("INSPECTION_LIMIT", "image.input_too_large")).not.toContain("image.input_too_large");
  });

  it("tells an unreadable image apart from unreadable requirements", () => {
    expect(errorCopy("INPUT_INVALID", "the image could not be decoded")).toContain("couldn't read this image");
    expect(errorCopy("INPUT_INVALID", "cannot read image: unexpected end of file")).toContain("couldn't read this image");
    expect(errorCopy("INPUT_INVALID", "requirements.invalid_numeric: 12.5")).toBe(
      "Fitifact couldn't find a format, size, or dimension rule in that text.",
    );
  });

  it("puts an engine refusal's detail in plain words", () => {
    expect(errorCopy("NO_VALID_PLAN", "image.byte_target_impossible: lossless PNG cannot meet the byte ceiling")).toBe(
      "Lossless PNG cannot meet the size limit.",
    );
    expect(errorCopy("NO_VALID_PLAN", "No plan fits.")).toBe("No plan fits.");
  });
});
