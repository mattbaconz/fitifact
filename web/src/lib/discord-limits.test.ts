import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { discordLimitsHtml } from "./discord-limits";

const registry = JSON.parse(readFileSync(new URL("../../public/registry.json", import.meta.url), "utf8"));

describe("discordLimitsHtml", () => {
  it("states each tier's cap from the shipped profiles", () => {
    const html = discordLimitsHtml(registry);
    for (const [tier, cap, bytes] of [
      ["Free", "20 MB", "20,000,000"],
      ["Nitro Basic", "50 MB", "50,000,000"],
      ["Nitro", "1 GB", "1,000,000,000"],
    ]) {
      expect(html).toContain(`<th scope="row">${tier}</th><td>${cap}</td><td>${bytes} bytes</td>`);
    }
    expect(html).toMatch(/checked <time datetime="\d{4}-\d{2}-\d{2}">/);
    expect(html).toContain('href="https://support.discord.com/');
  });

  it("refuses to build when a profile loses its byte rule", () => {
    const broken = {
      profiles: registry.profiles.filter((profile: { id: string }) => profile.id !== "discord/video-upload-nitro"),
    };
    expect(() => discordLimitsHtml(broken)).toThrow(/discord\/video-upload-nitro/);
  });
});
