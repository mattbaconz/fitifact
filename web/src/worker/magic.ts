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
  | "audio"
  | "pdf"
  | "zip"
  | "office"
  | "unsupported";

const STILL_IMAGE = new Set<InputKind>(["jpeg", "png", "webp", "heic", "gif", "tiff", "bmp"]);

const HEIC_BRANDS = new Set(["heic", "heix", "hevc", "hevx"]);
const HEIC_COMPAT = new Set(["heic", "heix", "hevc", "hevx", "mif1", "msf1", "heif", "avif", "avis"]);
const AUDIO_BRANDS = new Set(["M4A ", "M4B ", "M4P ", "mp4a"]);

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
    bytes[0] === 0xd0 &&
    bytes[1] === 0xcf &&
    bytes[2] === 0x11 &&
    bytes[3] === 0xe0
  ) {
    return "office";
  }
  if (bytes.length >= 5 && ascii(bytes, 0, 5) === "{\\rtf") {
    return "office";
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
  if (bytes.length >= 3 && ascii(bytes, 0, 3) === "ID3") return "audio";
  if (bytes.length >= 4 && (ascii(bytes, 0, 4) === "fLaC" || ascii(bytes, 0, 4) === "OggS")) return "audio";
  if (bytes.length >= 12 && ascii(bytes, 0, 4) === "RIFF" && ascii(bytes, 8, 12) === "WAVE") return "audio";
  if (bytes.length >= 16 && ascii(bytes, 4, 8) === "ftyp") {
    const boxSize = new DataView(bytes.buffer, bytes.byteOffset, 4).getUint32(0);
    if (boxSize < 16 || boxSize > bytes.length) return "unsupported";
    if (HEIC_BRANDS.has(ascii(bytes, 8, 12))) return "heic";
    for (let offset = 16; offset + 4 <= boxSize; offset += 4) {
      if (HEIC_BRANDS.has(ascii(bytes, offset, offset + 4))) return "heic";
    }
    const brand = ascii(bytes, 8, 12);
    if (AUDIO_BRANDS.has(brand)) return "audio";
    if (!HEIC_COMPAT.has(brand)) return "video";
    return "unsupported";
  }
  if (bytes.length >= 2 && bytes[0] === 0xff && (bytes[1] & 0xe0) === 0xe0) return "audio";
  return "unsupported";
}

export function refuseMessage(kind: InputKind): string {
  switch (kind) {
    case "video":
      return "This is a video. The web app adapts photos. Open it in the Fitifact desktop app.";
    case "matroska":
      return "This is WebM or MKV. The web app adapts photos. Open it in the Fitifact desktop app.";
    case "audio":
      return "This is audio. The web app adapts photos. Open it in the Fitifact desktop app.";
    case "pdf":
      return "This is a PDF. Fitifact adapts photos, video, and sound files, not documents.";
    case "zip":
      return "This is an archive or Office file. Fitifact adapts photos, video, and sound, not ZIP or documents.";
    case "office":
      return "This is an Office document. Fitifact adapts photos, video, and sound files, not documents.";
    default:
      return "This file is not a supported still image. SVG and HTML are never rendered.";
  }
}

export function isStillImage(kind: InputKind): boolean {
  return STILL_IMAGE.has(kind);
}

export function isDesktopMedia(kind: InputKind): boolean {
  return kind === "video" || kind === "matroska" || kind === "audio";
}

function ascii(bytes: Uint8Array, start: number, end: number): string {
  return String.fromCharCode(...bytes.subarray(start, end));
}
