import type { CropRectangle, ErrorReport, ProductState } from "../types";

export type WorkerRequest =
  | { id: number; type: "compile"; requirements: string }
  | { id: number; type: "compile_constraints"; constraintsJson: string }
  | { id: number; type: "compile_profile"; profileId: string }
  | { id: number; type: "inspect"; file: File }
  | { id: number; type: "plan"; constraintsJson: string }
  | { id: number; type: "replan"; previousConstraintsJson: string; constraintsJson: string }
      | { id: number; type: "adapt"; constraintsJson: string; crop: CropRectangle | null; firstFrameConsent: boolean };

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

export function isWorkerResponse(value: unknown): value is WorkerResponse {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Record<string, unknown>;
  return (
    typeof candidate.id === "number" &&
    (candidate.type === "progress" || candidate.type === "result" || candidate.type === "failure")
  );
}
