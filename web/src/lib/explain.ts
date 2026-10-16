import { formatBytes } from "./constraints";
import type { CompatibilityReport, ConstraintCheck, ImageKind, PlanReport, RasterFormat, RequirementParse } from "../types";

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
    case "mp4":
      return "MP4";
    case "mov":
    case "quicktime":
      return "MOV";
    case "webm":
      return "WebM";
    case "mkv":
    case "matroska":
      return "MKV";
    case "h264":
    case "avc":
      return "H.264";
    case "hevc":
    case "h265":
      return "HEVC";
    case "vp9":
      return "VP9";
    case "vp8":
      return "VP8";
    case "av1":
      return "AV1";
    case "aac":
      return "AAC";
    case "opus":
      return "Opus";
    case "none":
      return "none";
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
      return "File type";
    case "media.video.codec":
      return "Video";
    case "media.audio.codec":
      return "Sound";
    case "media.video.width":
      return "Width";
    case "media.video.height":
      return "Height";
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
  if (field === "image.format" || field === "media.container" || field === "media.video.codec" || field === "media.audio.codec") {
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
  return parts.length ? `Understood: ${parts.join(", ")}.` : null;
}

export function inspectLine(kind: ImageKind | RasterFormat | null | undefined, width: number | null | undefined, height: number | null | undefined, bytes: number): string {
  const size = width && height ? `${width}×${height}` : "size unknown";
  return `${formatLabel(kind)} · ${formatBytes(bytes)} · ${size}`;
}

function problemForCheck(check: ConstraintCheck): string | null {
  if (check.result !== "fail") return null;
  if (check.field === "image.format") {
    return `${formatLabel(check.actual)} isn't accepted`;
  }
  if (check.field === "file.bytes") {
    return `The file is too large (${formatCheckValue("file.bytes", check.actual)} vs ${formatCheckValue("file.bytes", check.required)})`;
  }
  if (check.field === "image.width" || check.field === "image.height") {
    return "The dimensions don't match";
  }
  if (check.field === "media.container") {
    return `This is ${formatCheckValue("media.container", check.actual)}; the destination needs ${formatCheckValue("media.container", check.required)}`;
  }
  if (check.field === "media.video.codec") {
    return "The video won't play there as-is";
  }
  if (check.field === "media.audio.codec") {
    return "The sound won't play there as-is";
  }
  if (check.field === "media.video.width" || check.field === "media.video.height") {
    return "The frame is larger than allowed";
  }
  return `${checkLabel(check.field)} doesn't match`;
}

export function describeProblemsFromChecks(report: CompatibilityReport | null | undefined): string[] {
  if (!report) return [];
  const problems: string[] = [];
  let sawImageDimensions = false;
  let sawVideoDimensions = false;
  for (const check of report.checks) {
    if (check.field === "image.width" || check.field === "image.height") {
      if (check.result !== "fail") continue;
      if (!sawImageDimensions) {
        problems.push("The dimensions don't match");
        sawImageDimensions = true;
      }
      continue;
    }
    if (check.field === "media.video.width" || check.field === "media.video.height") {
      if (check.result !== "fail") continue;
      if (!sawVideoDimensions) {
        problems.push("The frame is larger than allowed");
        sawVideoDimensions = true;
      }
      continue;
    }
    const line = problemForCheck(check);
    if (line) problems.push(line);
  }
  if (!problems.length && !report.compatible) {
    problems.push("This file does not match the destination rules");
  }
  return problems;
}

export function describeProblems(plan: PlanReport): string[] {
  const problems = describeProblemsFromChecks(plan.report);
  const sawDimensions = problems.includes("The dimensions don't match");
  if (plan.plan.target.crop.required && !sawDimensions) {
    problems.push("The image isn't the required shape");
  }
  return problems;
}

function keptLabel(token: string): string | null {
  switch (token) {
    case "source_format":
      return "Original format";
    case "dimensions":
    case "image_dimensions":
    case "video_dimensions":
      return "Frame size";
    case "aspect_ratio":
      return "Shape";
    case "alpha":
      return "Transparency";
    case "all_streams_copied":
    case "all streams copied":
      return "Picture and sound (copied, not re-encoded)";
    case "audio_stream_copied":
    case "audio stream copied":
      return "Sound";
    case "video_pixel_format":
    case "video pixel format":
      return "Color sampling";
    case "video_color_metadata":
    case "video color metadata":
      return "Color metadata";
    default:
      return token.replace(/_/g, " ");
  }
}

export function describeKept(plan: PlanReport | null, extra: string[] = []): string[] {
  const tokens = [
    ...(plan?.plan.target.preservation ?? []),
    ...extra,
  ];
  const seen = new Set<string>();
  const kept: string[] = [];
  for (const token of tokens) {
    const label = keptLabel(token);
    if (!label || seen.has(label)) continue;
    seen.add(label);
    kept.push(label);
  }
  if (plan?.plan.target.metadata === "keep" || plan?.plan.target.metadata === "preserve_unchanged") kept.push("Metadata");
  return kept;
}

export function describeActions(plan: PlanReport, completed = false): string[] {
  if (plan.report.compatible && plan.plan.noop) return ["Nothing. This file already fits."];
  const actions: string[] = [];
  if (plan.plan.source_format !== plan.plan.target.format) {
    actions.push(completed ? `Converted to ${formatLabel(plan.plan.target.format)}` : `The destination needs ${formatLabel(plan.plan.target.format)}`);
  }
  if (plan.plan.target.crop.required) {
    actions.push(completed ? "Cropped using the framing you approved" : "Crop to the required shape. Choose framing");
  }
  if (plan.plan.target.first_frame?.required) {
    actions.push(completed ? "Kept the first frame or page with your approval" : "Keep only the first frame or page. Extra frames are discarded");
  }
  if (plan.plan.source_width !== plan.plan.target.width || plan.plan.source_height !== plan.plan.target.height) {
    actions.push(`${completed ? "Resized" : "Resize"} to ${plan.plan.target.width}×${plan.plan.target.height}`);
  }
  const sizeFails = plan.report.checks.some((check) => check.field === "file.bytes" && check.result === "fail");
  if (!completed && sizeFails && plan.plan.target.max_bytes && (plan.plan.target.quality_warnings.length || plan.plan.target.proportional_reduction_allowed)) {
    actions.push("Reduce quality only as much as required to stay under the size limit");
  }
  if (
    plan.plan.target.metadata === "strip" ||
    plan.plan.target.metadata === "normalize_orientation_and_strip"
  ) {
    actions.push("Metadata is removed during re-encoding");
  }
  return actions;
}

export function describeMediaActions(steps: Array<{ operation: string; reasons?: Array<{ message: string }>; warnings?: string[] }> | undefined): string[] {
  if (!steps?.length) return [];
  return steps.map((step) => {
    const operation = step.operation.toLowerCase();
    if (operation.includes("remux")) return "Change the wrapper. Keep the same picture and sound";
    if (operation.includes("transcode_audio") || operation.includes("transcode-audio")) {
      const reason = step.reasons?.[0]?.message?.toLowerCase() ?? "";
      if (reason.includes("byte") || reason.includes("size")) {
        return "Re-encode so the file fits the size limit";
      }
      if (reason.includes("picture") || reason.includes("video")) {
        return "Change the sound format. Keep the picture";
      }
      return "Change the sound format";
    }
    if (operation.includes("transcode")) {
      const reason = step.reasons?.[0]?.message?.toLowerCase() ?? "";
      if (reason.includes("byte") || reason.includes("size")) {
        return "Re-encode so the file fits the size limit";
      }
      if (reason.includes("width") || reason.includes("height") || reason.includes("dimension")) {
        return "Scale the frame so it fits";
      }
      return "Re-encode the video so it plays there";
    }
    return step.reasons?.[0]?.message ?? operation.replace(/[._]/g, " ");
  });
}

export function describeVerified(opts: {
  compatible: boolean;
  adapted: boolean;
  checks: ConstraintCheck[];
}): string | null {
  if (!opts.checks.length) {
    if (opts.compatible) return "Verified: this file already matches the destination. Nothing was changed.";
    if (opts.adapted) return "Verified: the new file matches the destination rules.";
    return null;
  }
  const unresolved = opts.checks.filter((check) => check.result !== "pass");
  if (unresolved.length) return null;
  if (opts.compatible && !opts.adapted) {
    return "Verified: this file already matches the destination. Nothing was changed.";
  }
  if (opts.adapted) return "Verified: the new file matches the destination rules.";
  return null;
}
