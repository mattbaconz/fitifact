import { describe, expect, it } from "vitest";
import { constraintSetFromEditable, editableTargetFromConstraints } from "./constraints";

describe("parser-facing editable target", () => {
  it("preserves normalized core values while presenting editable fields", () => {
    const editable = editableTargetFromConstraints({
      schema: "fitifact.constraints/v1",
      hard: [
        { id: "format", field: "image.format", op: "in", value: ["png"] },
        { id: "bytes", field: "file.bytes", op: "lte", value: 2_000_000 },
        { id: "width", field: "image.width", op: "eq", value: 1200 },
        { id: "height", field: "image.height", op: "gte", value: 630 },
      ],
      preferences: { preserve_audio: true, preserve_resolution: true },
    });
    expect(editable).toEqual({
      format: "png",
      maxBytes: "2000000",
      width: "1200",
      widthOp: "eq",
      height: "630",
      heightOp: "gte",
    });
    expect(constraintSetFromEditable(editable).hard).toEqual([
      { id: "image-format", field: "image.format", op: "in", value: ["png"] },
      { id: "max-bytes", field: "file.bytes", op: "lte", value: 2_000_000 },
      { id: "image-width", field: "image.width", op: "eq", value: 1200 },
      { id: "image-height", field: "image.height", op: "gte", value: 630 },
    ]);
  });

  it("refuses non-integer form values before asking the core to compile", () => {
    expect(() =>
      constraintSetFromEditable({
        format: "jpeg",
        maxBytes: "2.5",
        width: "",
        widthOp: "lte",
        height: "",
        heightOp: "lte",
      }),
    ).toThrow("Maximum bytes must be a positive whole number");
  });
});
