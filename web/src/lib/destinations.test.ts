import { describe, expect, it } from "vitest";
import {
  destinationChips,
  discordUsingCopy,
  isImageCapableProfile,
  profileForFamily,
  resolveProfile,
  sameAsLastTimeCopy,
  usingDestinationCopy,
} from "./destinations";

describe("destination catalog", () => {
  it("maps Discord cap and file kind onto shipped profile ids", () => {
    expect(profileForFamily("discord", "free", "video")).toBe("discord/video-upload");
    expect(profileForFamily("discord", "nitro-basic", "video")).toBe(
      "discord/video-upload-nitro-basic",
    );
    expect(profileForFamily("discord", "nitro", "image")).toBe("discord/image-upload-nitro");
    expect(profileForFamily("jpeg", "free", "video")).toBeNull();
    expect(profileForFamily("generic-video", "free", "image")).toBeNull();
    expect(profileForFamily("github", "free", "image")).toBe("github/comment-image");
    expect(profileForFamily("whatsapp", "free", "image")).toBe("whatsapp/photo");
    expect(profileForFamily("x", "free", "image")).toBe("x/image");
    expect(profileForFamily("slack", "free", "image")).toBe("slack/file-image");
    expect(profileForFamily("github", "free", "video")).toBeNull();
    expect(isImageCapableProfile("github/comment-image")).toBe(true);
    expect(isImageCapableProfile("generic/video-upload")).toBe(false);
  });

  it("resolves the first enabled family that matches the file kind", () => {
    expect(
      resolveProfile({ families: ["discord", "gmail"], discordCap: "nitro-basic" }, "video"),
    ).toBe("discord/video-upload-nitro-basic");
    expect(
      resolveProfile({ families: ["jpeg", "generic-video"], discordCap: "free" }, "image"),
    ).toBe("jpeg/photo-upload");
    expect(
      resolveProfile({ families: ["jpeg"], discordCap: "free" }, "video"),
    ).toBeNull();
  });

  it("never claims Nitro was detected", () => {
    expect(discordUsingCopy("free")).toBe("Using Discord free upload (the cap you set).");
    expect(discordUsingCopy("nitro")).toBe("Using Discord Nitro upload (the cap you set).");
    expect(usingDestinationCopy("gmail", "free")).toBe("Using Gmail attachment.");
    expect(usingDestinationCopy("github", "free")).toBe("Using GitHub comment image.");
    expect(usingDestinationCopy("whatsapp", "free")).toBe("Using WhatsApp photo.");
    expect(usingDestinationCopy("x", "free")).toBe("Using X image.");
    expect(usingDestinationCopy("slack", "free")).toBe("Using Slack file image.");
    expect(sameAsLastTimeCopy("discord", "free")).toBe(
      "Same as last time: Discord free (the cap you set).",
    );
  });
});
