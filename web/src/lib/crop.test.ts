import { describe, expect, it } from "vitest";
import { cropForAspect } from "./crop";

describe("interactive crop geometry", () => {
  it("moves a landscape crop without leaving normalized bounds", () => {
    const left = cropForAspect(1200, 800, 800, 800, 0);
    const right = cropForAspect(1200, 800, 800, 800, 100);
    expect(left).toEqual({ x: 0, y: 0, width: 2 / 3, height: 1 });
    expect(right.x).toBeCloseTo(1 / 3);
    expect(right).toMatchObject({ y: 0, width: 2 / 3, height: 1 });
  });

  it("moves a portrait crop vertically", () => {
    const crop = cropForAspect(800, 1200, 800, 800, 50);
    expect(crop.y).toBeCloseTo(1 / 6);
    expect(crop).toMatchObject({ x: 0, width: 1, height: 2 / 3 });
  });
});
