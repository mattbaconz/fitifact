const COPY: Record<string, string> = {
  INPUT_INVALID:
    "Fitifact couldn't find a format, size, or dimension rule in that text.",
  INSPECTION_LIMIT:
    "This image is larger than Fitifact can process in the browser (32 MiB or 24 megapixels).",
  EXECUTION_LIMIT:
    "This image is larger than Fitifact can process in the browser (32 MiB or 24 megapixels).",
  INPUT_UNREADABLE:
    "Fitifact couldn't read this image. The file may be damaged or not the type its name says.",
  UNSUPPORTED_HEIC: "This phone photo (HEIC) can't be decoded in this build.",
  EXECUTION_CANCELLED: "Stopped. Nothing was saved.",
  PROVIDER_MISSING:
    "Video tools did not run. The desktop app ships FFmpeg; reinstall the app. The command-line tool still needs FFmpeg on PATH.",
};

const UNREADABLE_IMAGE =
  /^(the image (header|could not|orientation)|the input is not a recognized image|cannot (read|inspect) image)/;

const LIMIT_CODES = new Set([
  "INSPECTION_LIMIT",
  "EXECUTION_LIMIT",
  "image.input_too_large",
  "image.decoded_too_large",
]);

export function mapErrorCode(code: string, message = ""): string {
  if (
    LIMIT_CODES.has(code) ||
    message.includes("image.input_too_large") ||
    message.includes("image.decoded_too_large")
  ) {
    return code === "EXECUTION_LIMIT" ? "EXECUTION_LIMIT" : "INSPECTION_LIMIT";
  }
  // The engine reports unreadable images and unreadable requirements with one code.
  if (code === "INPUT_INVALID" && UNREADABLE_IMAGE.test(message)) return "INPUT_UNREADABLE";
  return code;
}

export function errorCopy(code: string, fallback: string): string {
  const mapped = mapErrorCode(code, fallback);
  return COPY[mapped] ?? plainDetail(fallback);
}

/** Engine refusals read "image.reason: detail"; people get the detail in plain words. */
function plainDetail(message: string): string {
  const match = /^[a-z]+\.[a-z_]+: (.+)$/.exec(message);
  if (!match) return message;
  const detail = match[1]
    .replace(/byte ceiling/g, "size limit")
    .replace(/byte floor/g, "minimum size")
    .replace(/byte window/g, "size range");
  return `${detail[0].toUpperCase()}${detail.slice(1).replace(/\.?$/, ".")}`;
}
