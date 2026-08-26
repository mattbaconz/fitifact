import { formatBytes } from "./constraints";
import type { EditableTarget, OutputFormat } from "../types";

function formatName(format: OutputFormat): string {
  if (format === "jpeg") return "JPG";
  if (format === "webp") return "WebP";
  return "PNG";
}

function dimensionSummary(target: EditableTarget): string | null {
  if (target.widthExact && target.heightExact) return `${target.widthExact}×${target.heightExact}`;
  const width = target.widthExact
    ? target.widthExact
    : [target.widthMin && `≥${target.widthMin}`, target.widthMax && `≤${target.widthMax}`].filter(Boolean).join(" ");
  const height = target.heightExact
    ? target.heightExact
    : [target.heightMin && `≥${target.heightMin}`, target.heightMax && `≤${target.heightMax}`].filter(Boolean).join(" ");
  if (!width && !height) return null;
  if (width && height) return `${width} × ${height}`;
  return width ? `W ${width}` : `H ${height}`;
}

export function summarizeTarget(target: EditableTarget): string {
  const parts = [target.formats.map(formatName).join(" or ") || "Any format"];
  if (target.maxBytes.trim()) {
    const bytes = Number(target.maxBytes);
    parts.push(Number.isFinite(bytes) && bytes > 0 ? `≤${formatBytes(bytes)}` : `≤${target.maxBytes} B`);
  }
  const dimensions = dimensionSummary(target);
  if (dimensions) parts.push(dimensions);
  return parts.join(" · ");
}
