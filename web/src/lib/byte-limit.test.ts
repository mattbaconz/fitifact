import { expect, test } from "vitest";
import { byteLimitFromText } from "./constraints";

test("size units preserve exact byte semantics", () => {
  expect(byteLimitFromText("2 MB")).toBe(2000000);
  expect(byteLimitFromText("2 MiB")).toBe(2097152);
  expect(byteLimitFromText("1.5 MiB")).toBe(1572864);
  expect(byteLimitFromText("0.1 MB")).toBe(100000);
  expect(byteLimitFromText("1999999")).toBe(1999999);
  expect(byteLimitFromText("")).toBeNull();
});
test("ambiguous units, fractional bytes, and unsafe sizes are refused", () => {
  for (const value of ["2 M", "1.1 MiB", "-2 MB", "0", "9007199254740992", "1e9", "2 MB extra"])
    expect(() => byteLimitFromText(value)).toThrow();
});
