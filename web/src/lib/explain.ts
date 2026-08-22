import { formatBytes } from "./constraints";
import type { ImageKind, PlanReport, RasterFormat } from "../types";

export function formatLabel(value: string | null | undefined): string {
  switch (value) {
    case "jpeg":
    case "jpg":
      return "JPEG";
    case "png":
      return "PNG";
    case "webp":
      return "WebP";
    case "heif":
    case "heic":
      return "HEIC";
    default:
      return value?.toUpperCase() || "This file";
  }
}

export function inspectLine(kind: ImageKind | RasterFormat | null | undefined, width: number | null | undefined, height: number | null | undefined, bytes: number): string {
  const size = width && height ? `${width}×${height}` : "size unknown";
  return `${formatLabel(kind)} · ${formatBytes(bytes)} · ${size}`;
}

export function describeProblems(plan: PlanReport): string[] {
  const problems: string[] = [];
  let sawDimensions = false;
  for (const check of plan.report.checks) {
    if (check.result !== "fail") continue;
    if (check.field === "image.format") {
      problems.push(`${formatLabel(check.actual)} isn't accepted`);
    } else if (check.field === "file.bytes") {
      problems.push(`The file is too large (${formatBytes(Number(check.actual) || 0)} vs ${check.required})`);
    } else if (check.field === "image.width" || check.field === "image.height") {
      if (!sawDimensions) {
        problems.push("The dimensions don't match");
        sawDimensions = true;
      }
    } else {
      problems.push(`${check.field} doesn't match`);
    }
  }
  if (plan.plan.target.crop.required && !sawDimensions) {
    problems.push("The image isn't the required shape");
  }
  if (!problems.length && !plan.report.compatible) {
    problems.push("I don't know why this file is being rejected from the supplied rules");
  }
  return problems;
}

export function describeActions(plan: PlanReport): string[] {
  if (plan.report.compatible && plan.plan.noop) return ["Nothing. This file already fits."];
  const actions: string[] = [];
  if (plan.plan.source_format !== plan.plan.target.format) {
    actions.push(`Convert to ${formatLabel(plan.plan.target.format)}`);
  }
  if (plan.plan.target.crop.required) {
    actions.push("Crop to the required shape — choose framing");
  }
  if (plan.plan.source_width !== plan.plan.target.width || plan.plan.source_height !== plan.plan.target.height) {
    actions.push(`Resize to ${plan.plan.target.width}×${plan.plan.target.height}`);
  }
  if (plan.plan.target.max_bytes && (plan.plan.target.quality_warnings.length || plan.plan.target.proportional_reduction_allowed)) {
    actions.push("Reduce quality only as much as required to stay under the size limit");
  }
  return actions;
}
