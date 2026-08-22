import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { describe, expect, it, vi } from "vitest";
import {
  decodeSingleHeic,
  decodedRgbaLength,
  HeicDecodeFailure,
  requireSingleHeicImage,
} from "./heic-decoder";
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

  it("decodes the owned single-image HEIC fixture through the approved decoder", async () => {
    const fixture = fileURLToPath(
      new URL("../../../fixtures/image/synthetic-single.heic", import.meta.url),
    );
    const decoded = await decodeSingleHeic(new Uint8Array(await readFile(fixture)), 24_000_000);
    expect(decoded.width).toBe(16);
    expect(decoded.height).toBe(12);
    expect(decoded.rgba).toHaveLength(16 * 12 * 4);
    expect(new Set(decoded.rgba)).not.toEqual(new Set([0]));
  });

  it("rejects zero and multiple top-level images and frees every returned handle", () => {
    expect(() => requireSingleHeicImage([])).toThrowError(
      "The HEIC file did not contain a decodable image.",
    );
    const first = { free: vi.fn() };
    const second = { free: vi.fn() };
    expect(() => requireSingleHeicImage([first, second])).toThrowError(
      "Animated or multi-image HEIC files are not supported.",
    );
    expect(first.free).toHaveBeenCalledOnce();
    expect(second.free).toHaveBeenCalledOnce();
  });
});
