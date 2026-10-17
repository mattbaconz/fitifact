import type { Constraint, ConstraintSet, EditableTarget, OutputFormat } from "../types";

export const PRODUCIBLE_FORMATS: OutputFormat[] = ["jpeg", "png", "webp"];
const DEFAULT_FORMATS: OutputFormat[] = ["jpeg", "png"];
const KNOWN_FORMATS = new Set(["jpeg", "png", "webp", "gif", "tiff", "bmp", "heif", "heic"]);

export const EMPTY_TARGET: EditableTarget = {
  formats: [...DEFAULT_FORMATS], maxBytes: "", minBytes: "",
  widthExact: "", widthMin: "", widthMax: "",
  heightExact: "", heightMin: "", heightMax: "",
};

function numberValue(constraint: Constraint, label: string): number {
  if (
    typeof constraint.value !== "number" ||
    !Number.isSafeInteger(constraint.value) ||
    constraint.value < 1
  ) {
    throw new Error(`${label} has an unsupported non-positive or non-integer value.`);
  }
  return constraint.value;
}

export function editableTargetFromConstraints(constraints: ConstraintSet): EditableTarget {
  let allowed = new Set<OutputFormat>(PRODUCIBLE_FORMATS);
  let sawFormat = false;
  let maxBytes: number | null = null;
  let minBytes: number | null = null;
  const dimensions = {
    width: { exact: null as number | null, min: null as number | null, max: null as number | null },
    height: { exact: null as number | null, min: null as number | null, max: null as number | null },
  };

  for (const constraint of constraints.hard) {
    if (constraint.field === "image.format") {
      if (constraint.op !== "in" || !Array.isArray(constraint.value)) {
        throw new Error("The normalized image-format intersection cannot be edited safely.");
      }
      sawFormat = true;
      if (constraint.value.some((value) => typeof value !== "string" || !KNOWN_FORMATS.has(value))) {
        throw new Error("The normalized image-format intersection contains an unsupported alternative.");
      }
      const next = new Set(
        (constraint.value as string[]).filter((value): value is OutputFormat =>
          PRODUCIBLE_FORMATS.includes(value as OutputFormat),
        ),
      );
      allowed = new Set([...allowed].filter((format) => next.has(format)));
    } else if (constraint.field === "file.bytes") {
      if (constraint.op === "gte") {
        const value = numberValue(constraint, "Minimum bytes");
        minBytes = minBytes === null ? value : Math.max(minBytes, value);
      } else if (constraint.op === "lte") {
        const value = numberValue(constraint, "Maximum bytes");
        maxBytes = maxBytes === null ? value : Math.min(maxBytes, value);
      } else {
        throw new Error("Only a byte floor or ceiling can be edited safely.");
      }
    } else if (constraint.field === "image.width" || constraint.field === "image.height") {
      if (!(["eq", "gte", "lte"] as const).includes(constraint.op as "eq" | "gte" | "lte")) {
        throw new Error(`The ${constraint.field} operator cannot be edited safely.`);
      }
      const axis = constraint.field === "image.width" ? dimensions.width : dimensions.height;
      const value = numberValue(constraint, constraint.field);
      if (constraint.op === "eq") {
        if (axis.exact !== null && axis.exact !== value) {
          throw new Error(`The ${constraint.field} exact intersection is inconsistent.`);
        }
        axis.exact = value;
      } else if (constraint.op === "gte") {
        axis.min = axis.min === null ? value : Math.max(axis.min, value);
      } else {
        axis.max = axis.max === null ? value : Math.min(axis.max, value);
      }
    } else {
      throw new Error(`The normalized ${constraint.field} constraint cannot be edited safely.`);
    }
  }

  if (sawFormat && allowed.size === 0) throw new Error("The normalized image-format intersection is empty.");
  if (minBytes !== null && maxBytes !== null && minBytes > maxBytes) {
    throw new Error("The normalized byte range is inconsistent.");
  }
  for (const [label, axis] of Object.entries(dimensions)) {
    if (axis.min !== null && axis.max !== null && axis.min > axis.max) {
      throw new Error(`The normalized image ${label} intersection is inconsistent.`);
    }
    if (
      axis.exact !== null &&
      ((axis.min !== null && axis.exact < axis.min) ||
        (axis.max !== null && axis.exact > axis.max))
    ) {
      throw new Error(`The normalized exact image ${label} is outside its bounds.`);
    }
  }
  const formats = sawFormat
    ? PRODUCIBLE_FORMATS.filter((format) => allowed.has(format))
    : [...DEFAULT_FORMATS];
  return {
    formats: [...formats], maxBytes: maxBytes === null ? "" : String(maxBytes),
    minBytes: minBytes === null ? "" : String(minBytes),
    widthExact: dimensions.width.exact === null ? "" : String(dimensions.width.exact),
    widthMin: dimensions.width.min === null ? "" : String(dimensions.width.min),
    widthMax: dimensions.width.max === null ? "" : String(dimensions.width.max),
    heightExact: dimensions.height.exact === null ? "" : String(dimensions.height.exact),
    heightMin: dimensions.height.min === null ? "" : String(dimensions.height.min),
    heightMax: dimensions.height.max === null ? "" : String(dimensions.height.max),
  };
}

function positiveInteger(value: string, label: string): number | null {
  if (!value.trim()) return null;
  if (!/^\d+$/.test(value.trim()) || Number(value) < 1 || !Number.isSafeInteger(Number(value))) {
    throw new Error(`${label} must be a positive whole number.`);
  }
  return Number(value);
}

export function byteLimitFromText(value: string): number | null {
  const text = value.trim();
  if (!text) return null;
  const match = /^(\d{1,16})(?:\.(\d{1,12}))?\s*(B|KB|MB|GB|KiB|MiB|GiB)?$/i.exec(text);
  if (!match) throw new Error("Enter a size such as 2 MB, 1.5 MiB, or a whole number of bytes.");
  const scales: Record<string, bigint> = { b: 1n, kb: 1000n, mb: 1000000n, gb: 1000000000n, kib: 1024n, mib: 1048576n, gib: 1073741824n };
  const fraction = match[2] ?? "";
  const denominator = 10n ** BigInt(fraction.length);
  const numerator = BigInt(match[1] + fraction) * scales[(match[3] ?? "b").toLowerCase()];
  if (numerator % denominator !== 0n || numerator <= 0n || numerator / denominator > BigInt(Number.MAX_SAFE_INTEGER))
    throw new Error("The size must resolve to a positive, exact whole number of bytes.");
  return Number(numerator / denominator);
}

function addDimension(
  hard: Constraint[], field: "image.width" | "image.height", label: string,
  exactText: string, minText: string, maxText: string,
) {
  const exact = positiveInteger(exactText, `Exact ${label}`);
  const min = positiveInteger(minText, `Minimum ${label}`);
  const max = positiveInteger(maxText, `Maximum ${label}`);
  if (min !== null && max !== null && min > max) throw new Error(`${label} minimum exceeds maximum.`);
  if (exact !== null && ((min !== null && exact < min) || (max !== null && exact > max))) {
    throw new Error(`Exact ${label} is outside its minimum/maximum bounds.`);
  }
  if (exact !== null) hard.push({ id: `${label}-exact`, field, op: "eq", value: exact });
  if (min !== null) hard.push({ id: `${label}-minimum`, field, op: "gte", value: min });
  if (max !== null) hard.push({ id: `${label}-maximum`, field, op: "lte", value: max });
}

export function constraintSetFromEditable(target: EditableTarget): ConstraintSet {
  if (target.formats.some((format) => !PRODUCIBLE_FORMATS.includes(format))) {
    throw new Error("The target contains an unsupported image format.");
  }
  const formats = PRODUCIBLE_FORMATS.filter((format) => target.formats.includes(format));
  if (formats.length === 0) throw new Error("Select at least one allowed image format.");
  const hard: Constraint[] = [{ id: "image-format", field: "image.format", op: "in", value: formats }];
  const minBytes = byteLimitFromText(target.minBytes);
  const maxBytes = byteLimitFromText(target.maxBytes);
  if (minBytes !== null && maxBytes !== null && minBytes > maxBytes) {
    throw new Error("Minimum size exceeds maximum size.");
  }
  if (minBytes !== null) hard.push({ id: "min-bytes", field: "file.bytes", op: "gte", value: minBytes });
  if (maxBytes !== null) hard.push({ id: "max-bytes", field: "file.bytes", op: "lte", value: maxBytes });
  addDimension(hard, "image.width", "width", target.widthExact, target.widthMin, target.widthMax);
  addDimension(hard, "image.height", "height", target.heightExact, target.heightMin, target.heightMax);
  return {
    schema: "fitifact.constraints/v1", hard,
    preferences: { preserve_audio: true, preserve_resolution: true },
  };
}

export function formatBytes(bytes: number): string {
  if (bytes < 1_000) return `${bytes} B`;
  if (bytes < 1_000_000) return `${(bytes / 1_000).toFixed(1)} KB`;
  return `${(bytes / 1_000_000).toFixed(2)} MB`;
}
