import { formatBytes } from "./constraints";
import type { ImageKind, PlanReport, RasterFormat, RequirementParse } from "../types";

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
    case "gif":
      return "GIF";
    case "tiff":
    case "tif":
      return "TIFF";
    case "bmp":
      return "BMP";
    default:
      return value?.toUpperCase() || "This file";
  }
}

export function checkLabel(field: string): string {
  switch (field) {
    case "image.format":
      return "Format";
    case "image.width":
      return "Width";
    case "image.height":
      return "Height";
    case "file.bytes":
      return "File size";
    case "media.container":
      return "Container";
    case "media.video.codec":
      return "Video codec";
    case "media.audio.codec":
      return "Audio codec";
    default:
      return field;
  }
}

function numericToken(value: string): number | null {
  const match = value.replace(/,/g, "").match(/-?\d+(?:\.\d+)?/);
  if (!match) return null;
  const n = Number(match[0]);
  return Number.isFinite(n) ? n : null;
}

export function formatCheckValue(field: string, value: string | null | undefined): string {
  if (value == null || value === "") return "Unknown";
  if (field === "file.bytes") {
    const n = numericToken(value);
    if (n != null) {
      const prefix = /^(<=|≤|gte|>=|≥)/.test(value.trim()) ? (value.trim().startsWith(">") || value.trim().startsWith("≥") ? "≥" : "≤") : "";
      return `${prefix}${formatBytes(n)}`;
    }
  }
  if (field === "image.format") {
    return value
      .split(/[,\s/]+/)
      .map((part) => part.trim())
      .filter(Boolean)
      .map((part) => formatLabel(part.replace(/^<=|^>=|^≤|^≥/u, "")))
      .join(" or ");
  }
  return value;
}

export function leftoverNote(texts: string[]): string | null {
  const ignored = texts.map((text) => text.trim()).filter(Boolean);
  if (!ignored.length) return null;
  return `Not used: ${ignored.join(" · ")}.`;
}

export function understoodNote(parse: RequirementParse | null): string | null {
  const hard = parse?.constraints?.hard;
  if (!hard?.length) return null;
  const parts: string[] = [];
  for (const constraint of hard) {
    if (constraint.field === "image.format") {
      const raw = Array.isArray(constraint.value) ? constraint.value.join(",") : String(constraint.value);
      parts.push(formatCheckValue("image.format", raw));
    } else if (constraint.field === "file.bytes") {
      parts.push(`max ${formatCheckValue("file.bytes", String(constraint.value))}`);
    } else if (constraint.field === "image.width" || constraint.field === "image.height") {
      const axis = constraint.field === "image.width" ? "width" : "height";
      if (constraint.op === "eq") parts.push(`${axis} ${constraint.value}`);
      else if (constraint.op === "lte") parts.push(`max ${axis} ${constraint.value}`);
      else if (constraint.op === "gte") parts.push(`min ${axis} ${constraint.value}`);
    }
  }
  return parts.length ? `I took: ${parts.join(", ")}.` : null;
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
      problems.push(
        `The file is too large (${formatCheckValue("file.bytes", check.actual)} vs ${formatCheckValue("file.bytes", check.required)})`,
      );
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
    actions.push(`The destination needs ${formatLabel(plan.plan.target.format)}`);
  }
  if (plan.plan.target.crop.required) {
    actions.push("Crop to the required shape — choose framing");
  }
  if (plan.plan.target.first_frame?.required) {
    actions.push("Keep only the first frame or page — extra frames are discarded");
  }
  if (plan.plan.source_width !== plan.plan.target.width || plan.plan.source_height !== plan.plan.target.height) {
    actions.push(`Resize to ${plan.plan.target.width}×${plan.plan.target.height}`);
  }
  if (plan.plan.target.max_bytes && (plan.plan.target.quality_warnings.length || plan.plan.target.proportional_reduction_allowed)) {
    actions.push("Reduce quality only as much as required to stay under the size limit");
  }
  return actions;
}
