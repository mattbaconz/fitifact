import { describe, expect, it } from "vitest";
import { constraintsLookLikeImage, constraintsLookLikeMedia, desktopTargetFromText, fileNameFromPath, isDesktop } from "./desktop";
import { FFMPEG_INSTALL_COMMANDS, FFMPEG_INSTALL_COPY } from "./desktop-engine";

describe("desktop helpers", () => {
  it("is not a Tauri host in unit tests", () => {
    expect(isDesktop()).toBe(false);
  });

  it("parses profile ids, --for, JSON, and YAML", () => {
    expect(desktopTargetFromText("discord/video-upload-nitro-basic")).toEqual({
      profile: "discord/video-upload-nitro-basic",
    });
    expect(desktopTargetFromText("--for generic/video-upload")).toEqual({ profile: "generic/video-upload" });
    expect(desktopTargetFromText('{"schema":"fitifact.constraints/v1","hard":[]}')).toEqual({
      constraintsJson: '{"schema":"fitifact.constraints/v1","hard":[]}',
    });
    expect(desktopTargetFromText("schema: fitifact.constraints/v1\nhard: []")).toEqual({
      constraintsYaml: "schema: fitifact.constraints/v1\nhard: []",
    });
    expect(desktopTargetFromText("JPEG, max 2 MB")).toBeNull();
  });

  it("distinguishes image and media constraint snapshots", () => {
    expect(constraintsLookLikeImage('{"hard":[{"field":"image.format"}]}')).toBe(true);
    expect(constraintsLookLikeMedia('{"hard":[{"field":"media.container"}]}')).toBe(true);
    expect(fileNameFromPath("C:\\\\temp\\\\clip.mov")).toBe("clip.mov");
  });
});

describe("FFmpeg install copy", () => {
  it("keeps CLI PATH recipes and does not tell desktop users to install FFmpeg", () => {
    expect(FFMPEG_INSTALL_COMMANDS.map((item) => item.command)).toEqual([
      "sudo apt update && sudo apt install ffmpeg",
      "brew install ffmpeg",
      "winget install --id Gyan.FFmpeg -e",
    ]);
    expect(FFMPEG_INSTALL_COPY).toContain("desktop app ships FFmpeg");
    expect(FFMPEG_INSTALL_COPY).toContain("winget install --id Gyan.FFmpeg -e");
    expect(FFMPEG_INSTALL_COPY).not.toContain("Fitifact does not bundle FFmpeg.");
  });
});
