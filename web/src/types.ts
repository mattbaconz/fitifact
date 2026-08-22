export type ProductState =
  | "idle"
  | "requirements_ready"
  | "processing"
  | "planned"
  | "compatible"
  | "crop_approval_required"
  | "adapted"
  | "cannot_satisfy"
  | "validation_failure"
  | "resource_limit"
  | "cancelled"
  | "unsupported_heic"
  | "error";

export interface ErrorReport {
  schema: "fitifact.error/v1";
  code: string;
  message: string;
  details?: Record<string, unknown>;
}

export interface Constraint {
  id: string;
  field: "image.format" | "image.width" | "image.height" | "file.bytes" | "file.family";
  op: "eq" | "in" | "lte" | "gte";
  value: string | number | string[];
}

export interface ConstraintSet {
  schema: "fitifact.constraints/v1";
  hard: Constraint[];
  preferences: { preserve_audio: boolean; preserve_resolution: boolean };
}

export interface RequirementParse {
  schema: "fitifact.requirements/v1";
  constraints: ConstraintSet | null;
  source_spans: Array<{ start: number; end: number; text: string; constraint_ids: string[] }>;
  ambiguities: Array<{ start: number; end: number; text: string; message: string }>;
  unresolved: Array<{ start: number; end: number; text: string }>;
}

export interface ConstraintCheck {
  constraint_id: string;
  field: string;
  actual: string | null;
  required: string;
  result: "pass" | "fail" | "unknown";
}

export interface CompatibilityReport {
  schema: "fitifact.check/v1";
  compatible: boolean;
  checks: ConstraintCheck[];
}

export interface Artifact {
  schema: "fitifact.artifact/v1";
  byte_length: number;
  family: string;
  image?: {
    format: "jpeg" | "png" | null;
    width: number | null;
    height: number | null;
    alpha: boolean | null;
    animated: boolean | null;
  };
}

export interface CropRectangle {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface ImagePlan {
  schema: "fitifact.image-adapt-plan/v1";
  plan: {
    schema: "fitifact.plan/v1";
    planner_version: string;
    steps: Array<{ operation: "image.adapt" }>;
  };
  noop: boolean;
  source_format: "jpeg" | "png";
  source_width: number;
  source_height: number;
  target: {
    format: "jpeg" | "png";
    width: number;
    height: number;
    max_bytes: number | null;
    preservation: string[];
    metadata: string;
    crop: {
      required: boolean;
      explicit_consent_required: boolean;
      target_aspect_width: number;
      target_aspect_height: number;
    };
    quality_warnings: string[];
    upscale_warnings: string[];
    proportional_reduction_allowed: boolean;
  };
  warnings: string[];
}

export interface PlanReport {
  schema: "fitifact.web-plan/v1";
  inspection: Artifact;
  report: CompatibilityReport;
  plan: ImagePlan;
}

export interface AdaptReport {
  status: "compatible" | "adapted";
  source: Artifact;
  output_artifact: Artifact;
  report: CompatibilityReport;
  plan: ImagePlan;
  disclosures: string[];
  stats: {
    jpeg_encodes: number;
    dimension_reductions: number;
    jpeg_quality: number | null;
  };
}

export interface EditableTarget {
  formats: Array<"jpeg" | "png">;
  maxBytes: string;
  widthExact: string;
  widthMin: string;
  widthMax: string;
  heightExact: string;
  heightMin: string;
  heightMax: string;
}

export function isErrorReport(value: unknown): value is ErrorReport {
  return Boolean(
    value &&
      typeof value === "object" &&
      "schema" in value &&
      (value as { schema: unknown }).schema === "fitifact.error/v1",
  );
}
