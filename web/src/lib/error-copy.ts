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

const CODEC_NAMES: Record<string, string> = {
  h264: "H.264",
  hevc: "HEVC",
  vp8: "VP8",
  vp9: "VP9",
  av1: "AV1",
};

/** What the browser lacks for the engine's plan, by the codes `plan_media_probe` reports. */
const BROWSER_GAPS: Record<string, string> = {
  hdr: "This video is HDR, and the web app can't convert HDR to standard video the way the desktop app does.",
  display_transform:
    "This video is stored rotated, mirrored, or stretched, which the web app can't re-encode yet.",
  video_decoder: "This browser can't decode {codec} video.",
  h264_encoder: "This browser has no H.264 encoder the web app can use.",
  h264_encoder_output:
    "This browser's H.264 encoder writes video other apps may misread, so the web app won't use it.",
  audio_decoder: "This browser can't decode this video's soundtrack.",
  aac_encoder: "This browser can't encode the AAC audio this target needs.",
  audio_only_encode: "Only the soundtrack needs re-encoding, which the web app can't do yet.",
};

/** Why this browser can't run the engine's plan for a video, in plain words. */
export function browserVideoRefusal(missing: readonly string[], codec: string | null): string {
  const reasons = new Set(
    missing.flatMap((gap) => {
      const reason = BROWSER_GAPS[gap];
      return reason ? [reason.replace("{codec}", CODEC_NAMES[codec ?? ""] ?? "this")] : [];
    }),
  );
  const why = reasons.size
    ? [...reasons].join(" ")
    : "This video needs re-encoding the web app can't do in this browser.";
  return `${why} Open it in the Fitifact desktop app.`;
}
