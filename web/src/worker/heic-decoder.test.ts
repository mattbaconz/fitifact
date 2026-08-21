import { describe, expect, it } from "vitest";
import { decodedRgbaLength, HeicDecodeFailure } from "./heic-decoder";
import { productStateForError } from "./protocol";

describe("HEIC decoded resource limits", () => {
  it("derives the allocation from the core-provided pixel limit", () => {
    expect(decodedRgbaLength(3, 2, 6)).toBe(24);
    expect(() => decodedRgbaLength(3, 2, 5)).toThrowError(
      expect.objectContaining({ code: "INSPECTION_LIMIT" }) as HeicDecodeFailure,
    );
  });

  it("maps an actual decoded-limit failure to the resource-limit product state", () => {
    try {
      decodedRgbaLength(6_000, 4_001, 24_000_000);
      throw new Error("expected decoded limit failure");
    } catch (error) {
      expect(error).toBeInstanceOf(HeicDecodeFailure);
      const failure = error as HeicDecodeFailure;
      expect(failure.code).toBe("INSPECTION_LIMIT");
      expect(productStateForError({ code: failure.code })).toBe("resource_limit");
    }
  });
});
