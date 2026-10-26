import type { CropRectangle, ErrorReport, ProductState } from "../types";

export type WorkerRequest =
  | { id: number; type: "warm" }
  | { id: number; type: "compile"; requirements: string }
  | { id: number; type: "compile_constraints"; constraintsJson: string }
  | { id: number; type: "compile_profile"; profileId: string }
  | { id: number; type: "inspect"; file: File }
  | { id: number; type: "plan"; constraintsJson: string }
  | { id: number; type: "replan"; previousConstraintsJson: string; constraintsJson: string }
      | { id: number; type: "adapt"; constraintsJson: string; crop: CropRectangle | null; firstFrameConsent: boolean }
  | { id: number; type: "media_inspect"; file: File }
  | { id: number; type: "media_plan"; constraintsJson: string }
  | { id: number; type: "media_adapt"; constraintsJson: string; outputName: string };

/** How a player shows a video track: display size with pixel aspect applied, rotation, mirroring. */
export interface DisplayGeometry {
  display_width: number;
  display_height: number;
  rotation: number;
  flip: boolean;
}

/** The encode the engine asks the worker to try before it judges this browser. */
export interface MeasureRequest {
  video: { codec: "avc"; width: number; height: number; bitrate: number };
  audio: { codec: "aac"; channels: number; sample_rate: number | null; bitrate: number } | null;
}

/** What the worker measured for a {@link MeasureRequest}. */
export interface Measured {
  geometry: DisplayGeometry | null;
  video_decode: boolean;
  h264_sample: number[] | null;
  audio_decode: boolean;
  aac_encode: boolean;
}

/** The engine's H.264 ladder for this browser: the exact size and each bitrate to try in order. */
export interface TranscodeExecution {
  operation: "media.transcode_video";
  width: number;
  height: number;
  video_bitrates: number[];
  audio: "copy" | { codec: "aac"; channels: number; bitrate: number };
  max_bytes: number | null;
}

/** What this browser does with the engine's plan for a video. */
export type BrowserVerdict =
  | { status: "ready"; execution: { operation: "media.remux" } | TranscodeExecution }
  | { status: "measure"; measure: MeasureRequest }
  /** `missing` holds the engine's gap codes; the worker adds `reason`, the same in plain words. */
  | { status: "refused"; missing: string[]; reason: string };

export type WorkerResponse =
  | { id: number; type: "progress"; stage: string; percent: number }
  | { id: number; type: "result"; report: unknown; output?: ArrayBuffer; preview?: ArrayBuffer; constraintsSnapshot?: string }
  | { id: number; type: "failure"; state: ProductState; report: ErrorReport };

export function productStateForError(report: Pick<ErrorReport, "code">): ProductState {
  switch (report.code) {
    case "UNSUPPORTED_HEIC":
      return "unsupported_heic";
    case "INSPECTION_LIMIT":
    case "EXECUTION_LIMIT":
    case "image.input_too_large":
    case "image.decoded_too_large":
      return "resource_limit";
    case "VALIDATION_FAILED":
      return "validation_failure";
    case "NO_VALID_PLAN":
    case "SECURITY_BLOCKED":
    case "INSPECTION_UNSUPPORTED":
      return "cannot_satisfy";
    case "EXECUTION_CANCELLED":
      return "cancelled";
    default:
      return "error";
  }
}

/** A load that failed because the device is offline, said in those words. */
export function offlineReport(): ErrorReport {
  return {
    schema: "fitifact.error/v1",
    code: "NETWORK_UNAVAILABLE",
    message:
      "This device is offline, and Fitifact hadn't finished loading its tools. Reconnect and try again. Once they load, this page keeps working without a network.",
    details: {},
  };
}

export function isWorkerResponse(value: unknown): value is WorkerResponse {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.id === "number" &&
    (candidate.type === "progress" || candidate.type === "result" || candidate.type === "failure")
  );
}
