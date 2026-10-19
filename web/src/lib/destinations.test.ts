import { describe, expect, it } from "vitest";
import {
  chipSubtitle,
  discordUsingCopy,
  familyFromQuery,
  isImageCapableProfile,
  profileForFamily,
  profilePlainSummary,
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
    expect(profileForFamily("github", "free", "video")).toBe("github/comment-video");
    expect(profileForFamily("whatsapp", "free", "video")).toBe("whatsapp/video");
    expect(profileForFamily("x", "free", "video")).toBe("x/video");
    expect(profileForFamily("slack", "free", "video")).toBe("slack/file-video");
    expect(profileForFamily("jpeg", "free", "image")).toBe("generic/web-image");
    expect(profileForFamily("discord", "free", "audio")).toBe("discord/file-upload");
    expect(profileForFamily("gmail", "free", "audio")).toBe("gmail/attachment");
    expect(profileForFamily("slack", "free", "audio")).toBe("slack/file-upload");
    expect(profileForFamily("whatsapp", "free", "audio")).toBe("whatsapp/audio");
    expect(profileForFamily("x", "free", "audio")).toBeNull();
    expect(profileForFamily("github", "free", "audio")).toBeNull();
    expect(profileForFamily("jpeg", "free", "audio")).toBeNull();
    expect(profileForFamily("generic-video", "free", "audio")).toBeNull();
    expect(isImageCapableProfile("github/comment-image")).toBe(true);
    expect(isImageCapableProfile("generic/video-upload")).toBe(false);
  });

  it("resolves the first enabled family that matches the file kind", () => {
    expect(
      resolveProfile({ families: ["discord", "gmail"], discordCap: "nitro-basic" }, "video"),
    ).toBe("discord/video-upload-nitro-basic");
    expect(
      resolveProfile({ families: ["jpeg", "generic-video"], discordCap: "free" }, "image"),
    ).toBe("generic/web-image");
    expect(
      resolveProfile({ families: ["jpeg"], discordCap: "free" }, "video"),
    ).toBeNull();
  });

  it("never claims Nitro was detected", () => {
    expect(discordUsingCopy("free")).toBe("Using Discord free upload (the cap you set).");
    expect(discordUsingCopy("nitro")).toBe("Using Discord Nitro upload (the cap you set).");
    expect(usingDestinationCopy("gmail", "free")).toBe("Using Gmail attachment.");
    expect(usingDestinationCopy("github", "free")).toBe("Using GitHub comments.");
    expect(usingDestinationCopy("whatsapp", "free")).toBe("Using WhatsApp.");
    expect(usingDestinationCopy("x", "free")).toBe("Using X.");
    expect(usingDestinationCopy("slack", "free")).toBe("Using Slack files.");
    expect(sameAsLastTimeCopy("discord", "free")).toBe(
      "Same as last time: Discord free (the cap you set).",
    );
    expect(profilePlainSummary("whatsapp/video", "free")).toBe("WhatsApp video · 64 MB");
    expect(profilePlainSummary("whatsapp/audio", "free")).toBe("WhatsApp audio · 2 GB");
    expect(profilePlainSummary("discord/image-upload-nitro", "nitro")).toBe(
      "Discord Nitro image · 20 MB",
    );
    expect(profilePlainSummary("discord/file-upload-nitro", "nitro")).toBe(
      "Discord Nitro file · 1 GB",
    );
    expect(chipSubtitle("discord", "nitro")).toBe("1 GB (the cap you set)");
    expect(chipSubtitle("whatsapp", "free")).toBe("JPEG photo, 64 MB video");
    expect(profilePlainSummary("whatsapp/photo", "free")).toBe("WhatsApp photo · JPEG");
    expect(profilePlainSummary("discord/file-upload", "free")).toBe("Discord free file · 20 MB");
    expect(profilePlainSummary("slack/file-upload", "free")).toBe("Slack file · 1 GB");
    expect(profilePlainSummary("generic/web-image", "free")).toBe("Web image · JPEG PNG WebP · 8 MB");
  });

  it("reads a ?for= deep link and ignores unknown destinations", () => {
    expect(familyFromQuery("?for=discord")).toBe("discord");
    expect(familyFromQuery("?for=WhatsApp")).toBe("whatsapp");
    expect(familyFromQuery("?for=twitter")).toBe("x");
    expect(familyFromQuery("?for=web-image")).toBe("jpeg");
    expect(familyFromQuery("?for=myspace")).toBeNull();
    expect(familyFromQuery("")).toBeNull();
  });
});
