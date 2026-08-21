import { describe, expect, it, vi } from "vitest";
import { StaleConstraints, withMatchingConstraints } from "./snapshot";

describe("worker constraint snapshot", () => {
  it("refuses stale replan/adapt requests before their operation", () => {
    const operation = vi.fn(() => "entered");
    expect(() => withMatchingConstraints("canonical-a", "canonical-b", operation)).toThrow(StaleConstraints);
    expect(operation).not.toHaveBeenCalled();
    expect(withMatchingConstraints("canonical-a", "canonical-a", operation)).toBe("entered");
  });
});
