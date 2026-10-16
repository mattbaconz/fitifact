import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { deleteSavedTarget, listSavedTargets, saveTarget } from "./saved-targets";

beforeEach(() => {
  const values = new Map<string, string>();
  vi.stubGlobal("localStorage", {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => values.set(key, value),
  });
});
afterEach(() => vi.unstubAllGlobals());

const target = { name: "Application photo", requirements: "JPEG", constraintsJson: '{}' };
test("round-trips a named target without storing file bytes", () => {
  const saved = saveTarget({
    name: "School portal",
    requirements: "JPG, max 500KB, 600×600",
    constraintsJson: '{"schema":"fitifact.constraints/v1"}',
  });
  expect(saved.name).toBe("School portal");
  expect(listSavedTargets()).toEqual([saved]);
  expect(localStorage.getItem("fitifact.saved-targets.v1")).not.toContain("blob");
  deleteSavedTarget(saved.id);
  expect(listSavedTargets()).toEqual([]);
});

test("duplicate names cannot silently replace another saved target", () => {
  const saved = saveTarget(target);
  expect(() => saveTarget({ ...target, name: "APPLICATION PHOTO" })).toThrow(/already in use/);
  expect(listSavedTargets()).toEqual([saved]);
  saveTarget({ ...saved, name: "Portrait" });
  expect(listSavedTargets().map(item => item.name)).toEqual(["Portrait"]);
});

test("a full library refuses a new target without discarding the oldest", () => {
  for (let i = 0; i < 20; i++) saveTarget({ ...target, name: `Target ${i}` });
  const before = listSavedTargets();
  expect(() => saveTarget(target)).toThrow(/20 saved targets/);
  expect(listSavedTargets()).toEqual(before);
});
