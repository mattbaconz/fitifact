export type InputKind = "jpeg" | "png" | "heic" | "unsupported";

const HEIC_BRANDS = new Set(["heic", "heix", "hevc", "hevx"]);

export function classifyInput(bytes: Uint8Array): InputKind {
  if (bytes.length >= 3 && bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff) {
    return "jpeg";
  }
  if (
    bytes.length >= 8 &&
    bytes[0] === 0x89 &&
    bytes[1] === 0x50 &&
    bytes[2] === 0x4e &&
    bytes[3] === 0x47 &&
    bytes[4] === 0x0d &&
    bytes[5] === 0x0a &&
    bytes[6] === 0x1a &&
    bytes[7] === 0x0a
  ) {
    return "png";
  }
  if (bytes.length >= 16 && ascii(bytes, 4, 8) === "ftyp") {
    const boxSize = new DataView(bytes.buffer, bytes.byteOffset, 4).getUint32(0);
    if (boxSize < 16 || boxSize > bytes.length) return "unsupported";
    if (HEIC_BRANDS.has(ascii(bytes, 8, 12))) return "heic";
    for (let offset = 16; offset + 4 <= boxSize; offset += 4) {
      if (HEIC_BRANDS.has(ascii(bytes, offset, offset + 4))) return "heic";
    }
  }
  return "unsupported";
}

function ascii(bytes: Uint8Array, start: number, end: number): string {
  return String.fromCharCode(...bytes.subarray(start, end));
}
