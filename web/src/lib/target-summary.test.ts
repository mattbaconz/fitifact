import { describe, expect, it } from "vitest";
import { summarizeTarget } from "./target-summary";
import { EMPTY_TARGET } from "./constraints";

describe("target summary", () => {
  it("hides the schema behind a one-line consumer reading", () => {
    expect(summarizeTarget({
      ...EMPTY_TARGET,
      formats: ["jpeg"],
      maxBytes: "2000000",
      widthMax: "2000",
      heightMax: "2000",
    })).toBe("JPG · ≤2.00 MB · ≤2000 × ≤2000");
    expect(summarizeTarget({
      ...EMPTY_TARGET,
      formats: ["jpeg", "png"],
      widthExact: "600",
      heightExact: "600",
    })).toBe("JPG or PNG · 600×600");
  });
});
