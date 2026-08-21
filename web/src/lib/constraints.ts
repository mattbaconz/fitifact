import type { Constraint, ConstraintSet, EditableTarget } from "../types";

export const EMPTY_TARGET: EditableTarget = {
  format: "jpeg",
  maxBytes: "",
  width: "",
  widthOp: "lte",
  height: "",
  heightOp: "lte",
};

function numericConstraint(
  constraints: Constraint[],
  field: Constraint["field"],
  fallback: Pick<EditableTarget, "widthOp" | "heightOp">["widthOp"],
): { value: string; op: "eq" | "lte" | "gte" } {
  const found = constraints.find((constraint) => constraint.field === field);
  return {
    value: found && typeof found.value === "number" ? String(found.value) : "",
    op: found && ["eq", "lte", "gte"].includes(found.op) ? (found.op as "eq" | "lte" | "gte") : fallback,
  };
}

export function editableTargetFromConstraints(constraints: ConstraintSet): EditableTarget {
  const formatConstraint = constraints.hard.find((constraint) => constraint.field === "image.format");
  const formats = Array.isArray(formatConstraint?.value) ? formatConstraint.value : [];
  const width = numericConstraint(constraints.hard, "image.width", "lte");
  const height = numericConstraint(constraints.hard, "image.height", "lte");
  const maxBytes = constraints.hard.find((constraint) => constraint.field === "file.bytes");
  return {
    format: formats.includes("png") && !formats.includes("jpeg") ? "png" : "jpeg",
    maxBytes: maxBytes && typeof maxBytes.value === "number" ? String(maxBytes.value) : "",
    width: width.value,
    widthOp: width.op,
    height: height.value,
    heightOp: height.op,
  };
}

function positiveInteger(value: string, label: string): number | null {
  if (!value.trim()) return null;
  if (!/^\d+$/.test(value.trim()) || Number(value) < 1 || !Number.isSafeInteger(Number(value))) {
    throw new Error(`${label} must be a positive whole number.`);
  }
  return Number(value);
}

export function constraintSetFromEditable(target: EditableTarget): ConstraintSet {
  const hard: Constraint[] = [
    { id: "image-format", field: "image.format", op: "in", value: [target.format] },
  ];
  const maxBytes = positiveInteger(target.maxBytes, "Maximum bytes");
  const width = positiveInteger(target.width, "Width");
  const height = positiveInteger(target.height, "Height");
  if (maxBytes !== null) hard.push({ id: "max-bytes", field: "file.bytes", op: "lte", value: maxBytes });
  if (width !== null) hard.push({ id: "image-width", field: "image.width", op: target.widthOp, value: width });
  if (height !== null) hard.push({ id: "image-height", field: "image.height", op: target.heightOp, value: height });
  return {
    schema: "fitifact.constraints/v1",
    hard,
    preferences: { preserve_audio: true, preserve_resolution: true },
  };
}

export function formatBytes(bytes: number): string {
  if (bytes < 1_000) return `${bytes} B`;
  if (bytes < 1_000_000) return `${(bytes / 1_000).toFixed(1)} KB`;
  return `${(bytes / 1_000_000).toFixed(2)} MB`;
}
