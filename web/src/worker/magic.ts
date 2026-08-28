export type InputKind =
  | "jpeg"
  | "png"
  | "webp"
  | "heic"
  | "gif"
  | "tiff"
  | "bmp"
  | "video"
  | "matroska"
  | "pdf"
  | "zip"
  | "unsupported";

const STILL_IMAGE = new Set<InputKind>(["jpeg", "png", "webp", "heic", "gif", "tiff", "bmp"]);

const HEIC_BRANDS = new Set(["heic", "heix", "hevc", "hevx"]);
const HEIC_COMPAT = new Set(["heic", "heix", "hevc", "hevx", "mif1", "msf1", "heif", "avif", "avis"]);

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
  if (bytes.length >= 12 && ascii(bytes, 0, 4) === "RIFF" && ascii(bytes, 8, 12) === "WEBP") {
    return "webp";
  }
  if (bytes.length >= 6 && (ascii(bytes, 0, 6) === "GIF87a" || ascii(bytes, 0, 6) === "GIF89a")) {
    return "gif";
  }
  if (
    bytes.length >= 4 &&
    ((bytes[0] === 0x49 && bytes[1] === 0x49 && bytes[2] === 0x2a && bytes[3] === 0x00) ||
      (bytes[0] === 0x4d && bytes[1] === 0x4d && bytes[2] === 0x00 && bytes[3] === 0x2a))
  ) {
    return "tiff";
  }
  if (bytes.length >= 2 && ascii(bytes, 0, 2) === "BM") {
    return "bmp";
  }
  if (bytes.length >= 4 && ascii(bytes, 0, 4) === "%PDF") {
    return "pdf";
  }
  if (
    bytes.length >= 4 &&
    bytes[0] === 0x50 &&
    bytes[1] === 0x4b &&
    (bytes[2] === 0x03 || bytes[2] === 0x05) &&
    (bytes[3] === 0x04 || bytes[3] === 0x06)
  ) {
    return "zip";
  }
  if (bytes.length >= 4 && bytes[0] === 0x1a && bytes[1] === 0x45 && bytes[2] === 0xdf && bytes[3] === 0xa3) {
    return "matroska";
  }
  if (bytes.length >= 16 && ascii(bytes, 4, 8) === "ftyp") {
    const boxSize = new DataView(bytes.buffer, bytes.byteOffset, 4).getUint32(0);
    if (boxSize < 16 || boxSize > bytes.length) return "unsupported";
    if (HEIC_BRANDS.has(ascii(bytes, 8, 12))) return "heic";
    for (let offset = 16; offset + 4 <= boxSize; offset += 4) {
      if (HEIC_BRANDS.has(ascii(bytes, offset, offset + 4))) return "heic";
    }
    const brand = ascii(bytes, 8, 12);
    if (!HEIC_COMPAT.has(brand)) return "video";
    return "unsupported";
  }
  return "unsupported";
}

export function refuseMessage(kind: InputKind): string {
  switch (kind) {
    case "video":
      return "This is a video. The web app adapts images. Use the desktop app or CLI after ffmpeg is on PATH.";
    case "matroska":
      return "This is WebM or Matroska. Fitifact adapts MP4 and MOV, not WebM or MKV.";
    case "pdf":
      return "This is a PDF. The web app adapts images and does not convert documents.";
    case "zip":
      return "This is an archive. Fitifact does not unpack or convert ZIP files.";
    default:
      return "This file is not a supported still image. SVG and HTML are never rendered.";
  }
}

export function isStillImage(kind: InputKind): boolean {
  return STILL_IMAGE.has(kind);
}

function ascii(bytes: Uint8Array, start: number, end: number): string {
  return String.fromCharCode(...bytes.subarray(start, end));
}
