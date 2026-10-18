import { afterEach, expect, it, vi } from "vitest";
import { readStored, writeStored, removeStored } from "./storage";

afterEach(() => vi.unstubAllGlobals());

it("keeps failed writes and removals readable for this session even when reads work", () => {
  vi.stubGlobal("localStorage", {
    getItem: () => "stale",
    setItem: () => {
      throw new Error("quota");
    },
    removeItem: () => {
      throw new Error("denied");
    },
  });
  writeStored("volatile-target", "new requirements");
  expect(readStored("volatile-target")).toBe("new requirements");
  removeStored("volatile-target");
  expect(readStored("volatile-target")).toBeNull();
});

it("returns to persistent storage when a later write succeeds", () => {
  const memory = new Map<string, string>();
  vi.stubGlobal("localStorage", {
    getItem: (key: string) => memory.get(key) ?? null,
    setItem: (key: string, value: string) => memory.set(key, value),
  });
  writeStored("volatile-target", "persisted");
  expect(readStored("volatile-target")).toBe("persisted");
  memory.set("volatile-target", "another tab");
  expect(readStored("volatile-target")).toBe("another tab");
});
