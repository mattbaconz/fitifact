import { describe, expect, it } from "vitest";
import { constraintSetFromEditable, editableTargetFromConstraints } from "./constraints";

describe("parser-facing editable target", () => {
  it("preserves every bound and both allowed formats through the editable round-trip", () => {
    const editable = editableTargetFromConstraints({
      schema: "fitifact.constraints/v1",
      hard: [
        { id: "format", field: "image.format", op: "in", value: ["jpeg", "png"] },
        { id: "bytes", field: "file.bytes", op: "lte", value: 2_000_000 },
        { id: "width-min", field: "image.width", op: "gte", value: 640 },
        { id: "height-min", field: "image.height", op: "gte", value: 480 },
        { id: "width-max", field: "image.width", op: "lte", value: 1920 },
        { id: "height-max", field: "image.height", op: "lte", value: 1080 },
      ],
      preferences: { preserve_audio: true, preserve_resolution: true },
    });
    expect(editable).toEqual({
      formats: ["jpeg", "png"], maxBytes: "2000000",
      widthExact: "", widthMin: "640", widthMax: "1920",
      heightExact: "", heightMin: "480", heightMax: "1080",
    });
    expect(constraintSetFromEditable(editable).hard).toEqual([
      { id: "image-format", field: "image.format", op: "in", value: ["jpeg", "png"] },
      { id: "max-bytes", field: "file.bytes", op: "lte", value: 2_000_000 },
      { id: "width-minimum", field: "image.width", op: "gte", value: 640 },
      { id: "width-maximum", field: "image.width", op: "lte", value: 1920 },
      { id: "height-minimum", field: "image.height", op: "gte", value: 480 },
      { id: "height-maximum", field: "image.height", op: "lte", value: 1080 },
    ]);
  });

  it("intersects repeated constraints and retains exact plus range bounds", () => {
    const editable = editableTargetFromConstraints({
      schema: "fitifact.constraints/v1",
      hard: [
        { id: "format-a", field: "image.format", op: "in", value: ["jpeg", "png"] },
        { id: "format-b", field: "image.format", op: "in", value: ["png"] },
        { id: "width-exact", field: "image.width", op: "eq", value: 800 },
        { id: "width-min", field: "image.width", op: "gte", value: 640 },
        { id: "width-max-a", field: "image.width", op: "lte", value: 1920 },
        { id: "width-max-b", field: "image.width", op: "lte", value: 1080 },
      ],
      preferences: { preserve_audio: true, preserve_resolution: true },
    });
    expect(editable).toMatchObject({ formats: ["png"], widthExact: "800", widthMin: "640", widthMax: "1080" });
    expect(constraintSetFromEditable(editable).hard).toEqual(expect.arrayContaining([
      { id: "width-exact", field: "image.width", op: "eq", value: 800 },
      { id: "width-minimum", field: "image.width", op: "gte", value: 640 },
      { id: "width-maximum", field: "image.width", op: "lte", value: 1080 },
    ]));
  });

  it("refuses unrepresentable normalized constraints", () => {
    expect(() => editableTargetFromConstraints({
      schema: "fitifact.constraints/v1",
      hard: [{ id: "family", field: "file.family", op: "eq", value: "image" }],
      preferences: { preserve_audio: true, preserve_resolution: true },
    })).toThrow("cannot be edited safely");

    expect(() => editableTargetFromConstraints({
      schema: "fitifact.constraints/v1",
      hard: [{ id: "format", field: "image.format", op: "in", value: ["jpeg", "avif"] }],
      preferences: { preserve_audio: true, preserve_resolution: true },
    })).toThrow("unsupported alternative");

    expect(() => editableTargetFromConstraints({
      schema: "fitifact.constraints/v1",
      hard: [
        { id: "width-exact", field: "image.width", op: "eq", value: 320 },
        { id: "width-min", field: "image.width", op: "gte", value: 640 },
      ],
      preferences: { preserve_audio: true, preserve_resolution: true },
    })).toThrow("outside its bounds");
  });
});
