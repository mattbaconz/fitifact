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

export function errorCopy(code: string, fallback: string): string {
  return COPY[code] ?? fallback;
}
