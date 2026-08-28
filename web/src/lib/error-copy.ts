const COPY: Record<string, string> = {
  INPUT_INVALID:
    "Fitifact couldn't find a format, size, or dimension rule in that text.",
  INSPECTION_LIMIT:
    "This image is larger than Fitifact can process in the browser (32 MiB or 24 megapixels).",
  EXECUTION_LIMIT:
    "This image is larger than Fitifact can process in the browser (32 MiB or 24 megapixels).",
  UNSUPPORTED_HEIC: "This phone photo (HEIC) can't be decoded in this build.",
  EXECUTION_CANCELLED: "Stopped. Nothing was saved.",
};

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
  return code;
}

export function errorCopy(code: string, fallback: string): string {
  const mapped = mapErrorCode(code, fallback);
  return COPY[mapped] ?? fallback;
}
