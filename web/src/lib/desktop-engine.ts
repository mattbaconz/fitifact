import type { CompatibilityReport, ErrorReport, ProductState } from "../types";
import { isErrorReport } from "../types";
import type { DesktopTarget } from "./desktop";
import { WorkerFailure } from "../worker/client";

export interface ProgressPayload {
  stage: string;
  percent: number;
}

export interface DoctorTool {
  name: string;
  available: boolean;
  version: string | null;
  detail: string | null;
}

export interface DoctorReport {
  schema: "fitifact.doctor/v1";
  healthy: boolean;
  tools: DoctorTool[];
  capabilities: Array<{ name: string; available: boolean; detail: string }>;
  warnings: string[];
}

export interface DesktopArtifact {
  schema: string;
  path?: string | null;
  family: string;
  byte_length: number;
  container?: string | { unknown?: string } | null;
  duration_ms?: number | null;
  streams?: Array<{
    type?: string;
    codec?: string | { unknown?: string };
    width?: number;
    height?: number;
  }>;
  image?: {
    format?: string | null;
    width?: number | null;
    height?: number | null;
  } | null;
}

export interface DesktopPlanOutcome {
  kind: "compatible" | "planned" | "cannot_satisfy";
  planner_version?: string;
  plan?: {
    steps: Array<{
      operation: string;
      reasons?: Array<{ message: string }>;
      warnings?: string[];
    }>;
    warnings?: string[];
  };
  blocking?: Array<{ message: string; code?: string }>;
  warnings?: string[];
}

export interface DesktopAdaptResult {
  status: "compatible" | "adapted" | "cannot_satisfy" | "failed";
  output?: string | null;
  explanation?: { summary: string; details: string[] };
  report?: CompatibilityReport;
  error?: ErrorReport | { code?: string; message?: string };
}

async function invokeJson<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  try {
    return await invoke<T>(command, args);
  } catch (caught) {
    throw desktopFailure(caught);
  }
}

function desktopFailure(caught: unknown): WorkerFailure {
  if (caught instanceof WorkerFailure) return caught;
  if (isErrorReport(caught)) {
    return new WorkerFailure(stateForCode(caught.code), caught);
  }
  if (caught && typeof caught === "object" && "message" in caught) {
    const record = caught as { code?: string; message?: string; schema?: string };
    const code = record.code ?? "EXECUTION_FAILED";
    return new WorkerFailure(stateForCode(code), {
      schema: "fitifact.error/v1",
      code,
      message: record.message ?? "Desktop command failed.",
    });
  }
  const message = typeof caught === "string" ? caught : caught instanceof Error ? caught.message : "Desktop command failed.";
  const codeMatch = message.match(/^([A-Z_]+): /);
  const code = codeMatch?.[1] ?? "EXECUTION_FAILED";
  return new WorkerFailure(stateForCode(code), {
    schema: "fitifact.error/v1",
    code,
    message: codeMatch ? message.slice(codeMatch[0].length) : message,
  });
}

function stateForCode(code: string): ProductState {
  switch (code) {
    case "PROVIDER_MISSING":
      return "error";
    case "NO_VALID_PLAN":
      return "cannot_satisfy";
    case "VALIDATION_FAILED":
      return "validation_failure";
    case "INSPECTION_LIMIT":
    case "EXECUTION_LIMIT":
      return "resource_limit";
    default:
      return "error";
  }
}

function targetArgs(target: DesktopTarget) {
  return {
    profile: target.profile ?? null,
    constraintsJson: target.constraintsJson ?? null,
    constraintsYaml: target.constraintsYaml ?? null,
  };
}

export async function desktopInspect(path: string) {
  return invokeJson<DesktopArtifact>("inspect", { path });
}

export async function desktopCheck(path: string, target: DesktopTarget) {
  return invokeJson<CompatibilityReport>("check", { path, target: targetArgs(target) });
}

export async function desktopPlan(path: string, target: DesktopTarget) {
  return invokeJson<DesktopPlanOutcome>("plan", { path, target: targetArgs(target) });
}

export async function desktopAdapt(path: string, target: DesktopTarget) {
  return invokeJson<DesktopAdaptResult>("adapt", { path, target: targetArgs(target) });
}

export async function desktopDoctor() {
  return invokeJson<DoctorReport>("doctor");
}

export async function desktopReadHeader(path: string) {
  const bytes = await invokeJson<number[]>("read_header", { path });
  return Uint8Array.from(bytes);
}

export async function desktopReadImage(path: string) {
  const { invoke } = await import("@tauri-apps/api/core");
  try {
    const payload = await invoke<ArrayBuffer | number[] | Uint8Array>("read_limited_file", { path });
    if (payload instanceof ArrayBuffer) return new Uint8Array(payload);
    if (payload instanceof Uint8Array) return payload;
    return Uint8Array.from(payload);
  } catch (caught) {
    throw desktopFailure(caught);
  }
}

export async function desktopOpenDialog(): Promise<string | null> {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({
    multiple: false,
    title: "Choose a file",
    filters: [
      {
        name: "Fitifact",
        extensions: ["jpg", "jpeg", "png", "webp", "heic", "heif", "tif", "tiff", "bmp", "gif", "mp4", "mov", "m4v"],
      },
    ],
  });
  return typeof selected === "string" ? selected : null;
}

export function codecLabel(value: unknown): string | null {
  if (typeof value === "string") return value.toUpperCase();
  if (value && typeof value === "object" && "unknown" in value) {
    const unknown = (value as { unknown?: string }).unknown;
    return unknown ? unknown.toUpperCase() : null;
  }
  return null;
}

export function containerLabel(value: unknown): string | null {
  if (typeof value === "string") return value.toUpperCase();
  if (value && typeof value === "object" && "unknown" in value) {
    const unknown = (value as { unknown?: string }).unknown;
    return unknown ? unknown.toUpperCase() : null;
  }
  return null;
}

export function inspectMediaLine(artifact: DesktopArtifact, bytesLabel: string): string {
  const video = artifact.streams?.find((stream) => stream.type === "video");
  const audio = artifact.streams?.find((stream) => stream.type === "audio");
  const parts = [
    containerLabel(artifact.container) ?? "MEDIA",
    codecLabel(video?.codec),
    codecLabel(audio?.codec),
    bytesLabel,
  ].filter(Boolean);
  if (video?.width && video.height) parts.push(`${video.width}×${video.height}`);
  return parts.join(" · ");
}

export const FFMPEG_INSTALL_COMMANDS = [
  { label: "Ubuntu/Debian", command: "sudo apt update && sudo apt install ffmpeg" },
  { label: "macOS (Homebrew)", command: "brew install ffmpeg" },
  { label: "Windows (WinGet)", command: "winget install --id Gyan.FFmpeg -e" },
] as const;

export const FFMPEG_INSTALL_COPY = [
  "Install a current FFmpeg build with libx264 and MP4 support, add it to PATH, then retry.",
  ...FFMPEG_INSTALL_COMMANDS.map((item) => `${item.label}: ${item.command}`),
  "Fitifact does not bundle FFmpeg.",
].join("\n");
