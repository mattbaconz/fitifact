import { afterEach, describe, expect, it } from "vitest";
import { clearLastTarget, loadLastTarget, loadSettings, saveLastTarget, saveSettings } from "./settings";

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

describe("web session settings", () => {
  it("defaults the .fitifact suffix on and first-frame consent off", () => {
    expect(loadSettings()).toEqual({
      fitifactSuffix: true,
      firstFrameConsentDefault: false,
    });
    saveSettings({ fitifactSuffix: false, firstFrameConsentDefault: true });
    expect(loadSettings()).toEqual({
      fitifactSuffix: false,
      firstFrameConsentDefault: true,
    });
  });

  it("round-trips the last confirmed target without storing file bytes", () => {
    saveLastTarget({
      requirements: "JPG, PNG, or WebP, max 2 MB",
      constraintsJson: '{"schema":"fitifact.constraints/v1"}',
    });
    expect(loadLastTarget()?.requirements).toContain("WebP");
    expect(localStorage.getItem("fitifact.last-target.v1")).not.toContain("blob");
    clearLastTarget();
    expect(loadLastTarget()).toBeNull();
  });

  it("restores a profile id without requiring a ConstraintSet snapshot", () => {
    saveLastTarget({
      requirements: "discord/image-upload",
      profile: "discord/image-upload",
      savedAt: "2026-08-27T00:00:00.000Z",
    });
    expect(loadLastTarget()).toMatchObject({
      profile: "discord/image-upload",
      requirements: "discord/image-upload",
    });
  });

  it("keeps a chip job when the paste box is empty", () => {
    saveLastTarget({
      requirements: "",
      profile: "gmail/attachment",
      savedAt: "2026-08-27T00:00:00.000Z",
    });
    expect(loadLastTarget()).toMatchObject({
      profile: "gmail/attachment",
      requirements: "",
    });
  });
});
