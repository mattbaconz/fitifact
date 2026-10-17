import { afterEach, describe, expect, it, vi } from "vitest";
import { fileActions } from "./share";

const png = new File([new Uint8Array([0x89, 0x50])], "out.png", { type: "image/png" });
const jpeg = new File([new Uint8Array([0xff, 0xd8])], "out.jpg", { type: "image/jpeg" });

function platform(options: { supports?: (type: string) => boolean; clipboardItem?: boolean; canShare?: (data: ShareData) => boolean }) {
  vi.stubGlobal("ClipboardItem", options.clipboardItem === false ? undefined : Object.assign(function ClipboardItem() {}, options.supports ? { supports: options.supports } : {}));
  vi.stubGlobal("navigator", { clipboard: { write: async () => {} }, ...(options.canShare ? { canShare: options.canShare } : {}) });
}

afterEach(() => vi.unstubAllGlobals());

describe("fileActions", () => {
  it("offers nothing without a validated file", () => {
    expect(fileActions(null)).toEqual({ copy: false, share: false });
  });

  it("copies only types the clipboard reports it accepts", () => {
    platform({ supports: (type) => type === "image/png" });
    expect(fileActions(png).copy).toBe(true);
    expect(fileActions(jpeg).copy).toBe(false);
  });

  it("falls back to PNG-only copy when the clipboard cannot report support", () => {
    platform({});
    expect(fileActions(png).copy).toBe(true);
    expect(fileActions(jpeg).copy).toBe(false);
  });

  it("hides copy without an async clipboard and shares only where files are shareable", () => {
    platform({ clipboardItem: false, canShare: (data) => Boolean(data.files?.length) });
    expect(fileActions(png)).toEqual({ copy: false, share: true });
  });

  it("treats a throwing canShare as unsupported", () => {
    platform({ canShare: () => { throw new TypeError("blocked"); } });
    expect(fileActions(png).share).toBe(false);
  });
});
