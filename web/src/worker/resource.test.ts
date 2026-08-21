import { describe, expect, it, vi } from "vitest";
import { EncodedResourceLimit, enterWasmWithEncodedLimit, readFileWithinLimit } from "./resource";

describe("encoded resource boundary", () => {
  it("rejects File.size before allocating its bytes", async () => {
    const arrayBuffer = vi.fn(async () => new ArrayBuffer(1));
    await expect(readFileWithinLimit({ size: 33, arrayBuffer }, 32)).rejects.toBeInstanceOf(EncodedResourceLimit);
    expect(arrayBuffer).not.toHaveBeenCalled();
  });

  it("rechecks allocated length and refuses the WASM entry", async () => {
    const wasmEntry = vi.fn(() => "entered");
    await expect(readFileWithinLimit(
      { size: 31, arrayBuffer: async () => new ArrayBuffer(33) },
      32,
    )).rejects.toBeInstanceOf(EncodedResourceLimit);
    expect(wasmEntry).not.toHaveBeenCalled();

    const buffer = await readFileWithinLimit(
      { size: 31, arrayBuffer: async () => new ArrayBuffer(32) },
      32,
    );
    expect(enterWasmWithEncodedLimit(buffer.byteLength, 32, wasmEntry)).toBe("entered");
  });
});
