import { describe, expect, it } from "vitest";
import {
  ageLabel,
  byteCapLabel,
  capLabel,
  daysSince,
  describeRule,
  isStale,
  registryProfile,
} from "./registry";

describe("profile registry", () => {
  it("ages profiles by their check date and flags stale ones after 30 days", () => {
    expect(daysSince("2026-10-17", new Date("2026-11-20T12:00:00Z"))).toBe(34);
    expect(ageLabel(0)).toBe("today");
    expect(ageLabel(1)).toBe("1 day ago");
    const gmail = registryProfile("gmail/attachment")!;
    expect(isStale(gmail, new Date("2026-11-16T00:00:00Z"))).toBe(false);
    expect(isStale(gmail, new Date("2026-11-17T00:00:00Z"))).toBe(true);
    expect(isStale(registryProfile("whatsapp/photo")!, new Date("2026-10-17T00:00:00Z"))).toBe(true);
  });

  it("reads caps and per-rule provenance from the shipped registry", () => {
    expect(capLabel(1_000_000_000)).toBe("1 GB");
    expect(byteCapLabel("discord/file-upload-nitro")).toBe("1 GB");
    expect(registryProfile("x/audio")).toBeNull();
    const rule = registryProfile("discord/video-upload")!.rules.find((item) => item.id === "bit-depth")!;
    expect(describeRule(rule)).toBe("Bit depth: at most 8");
    expect(rule.source_type).toBe("fitifact-rule");
  });
});
