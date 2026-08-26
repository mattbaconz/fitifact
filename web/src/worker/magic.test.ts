import { describe, expect, it } from "vitest";
import { classifyInput } from "./magic";

describe("local magic detection", () => {
  it("recognizes JPEG and the full PNG signature", () => {
    expect(classifyInput(Uint8Array.from([0xff, 0xd8, 0xff, 0x00]))).toBe("jpeg");
    expect(classifyInput(Uint8Array.from([0x89, 0x50, 0x4e, 0x47, 13, 10, 26, 10]))).toBe("png");
    expect(classifyInput(Uint8Array.from([0x89, 0x50, 0x4e, 0x47, 0, 0, 0, 0]))).toBe("unsupported");
    const webp = new Uint8Array(12);
    webp.set(new TextEncoder().encode("RIFF"), 0);
    webp.set(new TextEncoder().encode("WEBP"), 8);
    expect(classifyInput(webp)).toBe("webp");
  });

  it("detects HEIC from ISO BMFF brands rather than extensions", () => {
    const heic = new Uint8Array(24);
    heic.set([0, 0, 0, 24], 0);
    heic.set(new TextEncoder().encode("ftypheic"), 4);
    expect(classifyInput(heic)).toBe("heic");
    const avif = heic.slice();
    avif.set(new TextEncoder().encode("avif"), 8);
    avif.set(new TextEncoder().encode("mif1"), 16);
    expect(classifyInput(avif)).toBe("unsupported");
    const genericHeif = heic.slice();
    genericHeif.set(new TextEncoder().encode("mif1"), 8);
    genericHeif.set(new TextEncoder().encode("msf1"), 16);
    expect(classifyInput(genericHeif)).toBe("unsupported");
    const heicCompatible = avif.slice();
    heicCompatible.set(new TextEncoder().encode("heix"), 20);
    expect(classifyInput(heicCompatible)).toBe("heic");
  });

  it("requires a complete bounded ftyp box", () => {
    const truncated = new Uint8Array(16);
    truncated.set([0, 0, 0, 24], 0);
    truncated.set(new TextEncoder().encode("ftypheic"), 4);
    expect(classifyInput(truncated)).toBe("unsupported");
  });

  it("recognizes GIF, TIFF, BMP, video, PDF, and ZIP magic", () => {
    expect(classifyInput(new TextEncoder().encode("GIF89a"))).toBe("gif");
    const tiff = new Uint8Array([0x49, 0x49, 0x2a, 0x00]);
    expect(classifyInput(tiff)).toBe("tiff");
    expect(classifyInput(new TextEncoder().encode("BM"))).toBe("bmp");
    const video = new Uint8Array(16);
    video.set([0, 0, 0, 16], 0);
    video.set(new TextEncoder().encode("ftypisom"), 4);
    expect(classifyInput(video)).toBe("video");
    expect(classifyInput(new TextEncoder().encode("%PDF-1.7"))).toBe("pdf");
    expect(classifyInput(Uint8Array.from([0x50, 0x4b, 0x03, 0x04, 0, 0]))).toBe("zip");
  });

  it("never promotes SVG or HTML text into a renderable type", () => {
    expect(classifyInput(new TextEncoder().encode("<svg><script>alert(1)</script></svg>"))).toBe("unsupported");
    expect(classifyInput(new TextEncoder().encode("<!doctype html><img src=x>"))).toBe("unsupported");
  });
});
