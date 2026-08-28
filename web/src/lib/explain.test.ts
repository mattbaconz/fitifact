import { describe, expect, it } from "vitest";
import { checkLabel, describeActions, describeProblems, formatCheckValue, formatLabel, inspectLine, leftoverNote, understoodNote } from "./explain";
import type { PlanReport } from "../types";

function plan(overrides: Partial<PlanReport> & { checks?: PlanReport["report"]["checks"] }): PlanReport {
  return {
    schema: "fitifact.web-plan/v1",
    inspection: { schema: "fitifact.artifact/v1", byte_length: 4_800_000, family: "image", image: { format: "heif", width: 4032, height: 3024, alpha: false, animated: false } },
    report: {
      schema: "fitifact.check/v1",
      compatible: false,
      checks: overrides.checks ?? [
        { constraint_id: "format", field: "image.format", actual: "heif", required: "jpeg", result: "fail" },
        { constraint_id: "bytes", field: "file.bytes", actual: "4800000", required: "<= 2000000", result: "fail" },
        { constraint_id: "width", field: "image.width", actual: "4032", required: "600", result: "fail" },
      ],
    },
    plan: {
      schema: "fitifact.image-adapt-plan/v1",
      plan: { schema: "fitifact.plan/v1", planner_version: "test", steps: [{ operation: "image.adapt" }] },
      noop: false,
      source_format: "png",
      source_width: 4032,
      source_height: 3024,
      target: {
        format: "jpeg",
        width: 600,
        height: 600,
        max_bytes: 2_000_000,
        preservation: [],
        metadata: "strip",
        crop: { required: true, explicit_consent_required: true, target_aspect_width: 600, target_aspect_height: 600 },
        first_frame: { required: false, explicit_consent_required: false },
        quality_warnings: ["lossy"],
        upscale_warnings: [],
        proportional_reduction_allowed: true,
      },
      warnings: [],
    },
    ...overrides,
  };
}

describe("human plan copy", () => {
  it("names the four consumer problems without codec jargon", () => {
    expect(formatLabel("heic")).toBe("HEIC");
    expect(inspectLine("heic", 4032, 3024, 4_800_000)).toContain("HEIC");
    expect(describeProblems(plan({}))).toEqual([
      "HEIC isn't accepted",
      "The file is too large (4.80 MB vs ≤2.00 MB)",
      "The dimensions don't match",
    ]);
    expect(checkLabel("image.format")).toBe("Format");
    expect(checkLabel("file.bytes")).toBe("File size");
    expect(formatCheckValue("file.bytes", "2000000")).toBe("2.00 MB");
    expect(leftoverNote(["File must be"])).toBe("Not used: File must be.");
    expect(
      understoodNote({
        schema: "fitifact.requirements/v1",
        constraints: {
          schema: "fitifact.constraints/v1",
          hard: [
            { id: "format", field: "image.format", op: "in", value: ["jpeg"] },
            { id: "bytes", field: "file.bytes", op: "lte", value: 2_000_000 },
          ],
          preferences: { preserve_audio: true, preserve_resolution: true },
        },
        source_spans: [],
        ambiguities: [],
        unresolved: [{ start: 0, end: 12, text: "please use a recent photo" }],
      }),
    ).toBe("I took: JPEG, max 2.00 MB.");
  });

  it("lists only the mutations Fitifact will actually perform", () => {
    expect(describeActions(plan({}))).toEqual([
      "The destination needs JPEG",
      "Crop to the required shape — choose framing",
      "Resize to 600×600",
      "Reduce quality only as much as required to stay under the size limit",
    ]);
  });
});
