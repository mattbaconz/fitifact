import { afterEach, describe, expect, it } from "vitest";
import { deleteSavedTarget, listSavedTargets, saveTarget } from "./saved-targets";

const KEY = "fitifact.saved-targets.v1";
const memory = new Map<string, string>();
Object.defineProperty(globalThis, "localStorage", {
  configurable: true,
  value: {
    getItem(key: string) { return memory.get(key) ?? null; },
    setItem(key: string, value: string) { memory.set(key, value); },
    removeItem(key: string) { memory.delete(key); },
    clear() { memory.clear(); },
  },
});

afterEach(() => {
  localStorage.clear();
});

describe("local saved targets", () => {
  it("round-trips a named target without storing file bytes", () => {
    const saved = saveTarget({
      name: "School portal",
      requirements: "JPG, max 500KB, 600×600",
      constraintsJson: '{"schema":"fitifact.constraints/v1"}',
    });
    expect(saved.name).toBe("School portal");
    expect(listSavedTargets()).toEqual([saved]);
    expect(localStorage.getItem(KEY)).not.toContain("blob");
    deleteSavedTarget(saved.id);
    expect(listSavedTargets()).toEqual([]);
  });

  it("replaces a target with the same name instead of growing forever", () => {
    saveTarget({ name: "CMS", requirements: "JPEG", constraintsJson: "a" });
    saveTarget({ name: "CMS", requirements: "PNG", constraintsJson: "b" });
    const listed = listSavedTargets();
    expect(listed).toHaveLength(1);
    expect(listed[0].requirements).toBe("PNG");
  });
});
