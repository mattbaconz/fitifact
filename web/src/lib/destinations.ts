import { byteCap, byteCapLabel, capLabel } from "./registry";

export type DestinationFamily =
  | "discord"
  | "gmail"
  | "github"
  | "slack"
  | "whatsapp"
  | "x"
  | "jpeg"
  | "generic-video";
export type DiscordCap = "free" | "nitro-basic" | "nitro";
export type FileKind = "image" | "video" | "audio";

function discordCap(id: string, label: string) {
  const bytes = byteCap(id) ?? 0;
  return { bytes, label, short: capLabel(bytes) };
}

/** Upload caps per tier, read from the shipped Discord file profiles. */
export const DISCORD_CAPS: Record<
  DiscordCap,
  { bytes: number; label: string; short: string }
> = {
  free: discordCap("discord/file-upload", "Free"),
  "nitro-basic": discordCap("discord/file-upload-nitro-basic", "Nitro Basic"),
  nitro: discordCap("discord/file-upload-nitro", "Nitro"),
};

const DISCORD_VIDEO: Record<DiscordCap, string> = {
  free: "discord/video-upload",
  "nitro-basic": "discord/video-upload-nitro-basic",
  nitro: "discord/video-upload-nitro",
};

const DISCORD_IMAGE: Record<DiscordCap, string> = {
  free: "discord/image-upload",
  "nitro-basic": "discord/image-upload-nitro-basic",
  nitro: "discord/image-upload-nitro",
};

const DISCORD_FILE: Record<DiscordCap, string> = {
  free: "discord/file-upload",
  "nitro-basic": "discord/file-upload-nitro-basic",
  nitro: "discord/file-upload-nitro",
};

export const DESTINATION_FAMILIES: DestinationFamily[] = [
  "discord",
  "gmail",
  "github",
  "whatsapp",
  "x",
  "slack",
  "jpeg",
  "generic-video",
];

export const FAMILY_ORDER: DestinationFamily[] = DESTINATION_FAMILIES;

export const FAMILY_LABEL: Record<DestinationFamily, string> = {
  discord: "Discord",
  gmail: "Gmail",
  github: "GitHub",
  slack: "Slack",
  whatsapp: "WhatsApp",
  x: "X",
  jpeg: "Web image",
  "generic-video": "Web video",
};

export interface DestinationChip {
  family: DestinationFamily;
  label: string;
  subtitle: string;
  videoOnly: boolean;
  imageCapable: boolean;
}

export function discordUsingCopy(cap: DiscordCap): string {
  const name = cap === "free" ? "free" : DISCORD_CAPS[cap].label;
  return `Using Discord ${name} upload (the cap you set).`;
}

export function usingDestinationCopy(family: DestinationFamily, cap: DiscordCap): string {
  if (family === "discord") return discordUsingCopy(cap);
  if (family === "gmail") return "Using Gmail attachment.";
  if (family === "github") return "Using GitHub comments.";
  if (family === "slack") return "Using Slack files.";
  if (family === "whatsapp") return "Using WhatsApp.";
  if (family === "x") return "Using X.";
  if (family === "jpeg") return "Using a typical web image upload.";
  return "Using a typical web video upload.";
}

export function sameAsLastTimeCopy(family: DestinationFamily, cap: DiscordCap): string {
  if (family === "discord") {
    const name = cap === "free" ? "free" : DISCORD_CAPS[cap].label;
    return `Same as last time: Discord ${name} (the cap you set).`;
  }
  if (family === "gmail") return "Same as last time: Gmail.";
  if (family === "github") return "Same as last time: GitHub.";
  if (family === "slack") return "Same as last time: Slack.";
  if (family === "whatsapp") return "Same as last time: WhatsApp.";
  if (family === "x") return "Same as last time: X.";
  if (family === "jpeg") return "Same as last time: Web image.";
  return "Same as last time: Web video.";
}

export function chipSubtitle(family: DestinationFamily, cap: DiscordCap): string {
  if (family === "discord") {
    return `${DISCORD_CAPS[cap].short} (the cap you set)`;
  }
  if (family === "gmail") return `${byteCapLabel("gmail/attachment")} attachment`;
  if (family === "github") return `${byteCapLabel("github/comment-image")} comment`;
  if (family === "slack") return `${byteCapLabel("slack/file-upload")} file`;
  if (family === "whatsapp") {
    const photo = byteCap("whatsapp/photo") === null ? "JPEG" : byteCapLabel("whatsapp/photo");
    return `${photo} photo, ${byteCapLabel("whatsapp/video")} video`;
  }
  if (family === "x") return `${byteCapLabel("x/image")} photo, ${byteCapLabel("x/video")} video`;
  if (family === "jpeg") return `JPEG PNG WebP · ${byteCapLabel("generic/web-image")}`;
  return `MP4 · ${byteCapLabel("generic/video-upload")}`;
}

const QUERY_ALIASES: Record<string, DestinationFamily> = {
  twitter: "x",
  "web-image": "jpeg",
  "web-video": "generic-video",
};

/** The destination named by a `?for=` deep link, or null when absent or unknown. */
export function familyFromQuery(search: string): DestinationFamily | null {
  const value = new URLSearchParams(search).get("for")?.trim().toLowerCase() ?? "";
  if ((DESTINATION_FAMILIES as string[]).includes(value)) return value as DestinationFamily;
  return QUERY_ALIASES[value] ?? null;
}

export function destinationChips(
  cap: DiscordCap,
  options: { includeVideo?: boolean } = {},
): DestinationChip[] {
  const includeVideo = options.includeVideo ?? true;
  return FAMILY_ORDER.filter((family) => includeVideo || family !== "generic-video").map((family) => ({
    family,
    label: FAMILY_LABEL[family],
    subtitle: chipSubtitle(family, cap),
    videoOnly: family === "generic-video",
    imageCapable: family !== "generic-video",
  }));
}

export function profileForFamily(
  family: DestinationFamily,
  cap: DiscordCap,
  kind: FileKind,
): string | null {
  switch (family) {
    case "discord":
      return kind === "video"
        ? DISCORD_VIDEO[cap]
        : kind === "audio"
          ? DISCORD_FILE[cap]
          : DISCORD_IMAGE[cap];
    case "gmail":
      return "gmail/attachment";
    case "github":
      return kind === "image" ? "github/comment-image" : kind === "video" ? "github/comment-video" : null;
    case "slack":
      return kind === "image" ? "slack/file-image" : kind === "audio" ? "slack/file-upload" : "slack/file-video";
    case "whatsapp":
      return kind === "image" ? "whatsapp/photo" : kind === "audio" ? "whatsapp/audio" : "whatsapp/video";
    case "x":
      return kind === "image" ? "x/image" : kind === "video" ? "x/video" : null;
    case "jpeg":
      return kind === "image" ? "generic/web-image" : null;
    case "generic-video":
      return kind === "video" ? "generic/video-upload" : null;
  }
}

export function resolveProfile(
  setup: { families: readonly DestinationFamily[]; discordCap: DiscordCap },
  kind: FileKind,
): string | null {
  for (const family of FAMILY_ORDER) {
    if (!setup.families.includes(family)) continue;
    const id = profileForFamily(family, setup.discordCap, kind);
    if (id) return id;
  }
  return null;
}

export function familyForProfile(id: string): DestinationFamily | null {
  if (id.startsWith("discord/")) return "discord";
  if (id === "gmail/attachment") return "gmail";
  if (id === "github/comment-image" || id === "github/comment-video") return "github";
  if (id === "slack/file-image" || id === "slack/file-video" || id === "slack/file-upload") return "slack";
  if (id === "whatsapp/photo" || id === "whatsapp/video" || id === "whatsapp/audio") return "whatsapp";
  if (id === "x/image" || id === "x/video") return "x";
  if (id === "jpeg/photo-upload" || id === "generic/web-image") return "jpeg";
  if (id === "generic/video-upload") return "generic-video";
  return null;
}

export function isImageCapableProfile(id: string): boolean {
  const family = familyForProfile(id);
  return family !== null && family !== "generic-video";
}

export function isVideoProfile(id: string): boolean {
  return id.includes("/video") && !id.includes("image");
}

export function profilePlainSummary(id: string, cap: DiscordCap): string {
  const family = familyForProfile(id);
  if (!family) return id;
  const size = byteCapLabel(id);
  if (family === "discord") {
    const name = cap === "free" ? "free" : DISCORD_CAPS[cap].label;
    if (isVideoProfile(id)) return `Discord ${name} video · ${size}`;
    if (id.includes("file-upload")) return `Discord ${name} file · ${size}`;
    return `Discord ${name} image · ${size}`;
  }
  if (family === "gmail") return `Gmail attachment · ${size}`;
  if (family === "github") {
    return `GitHub comment ${isVideoProfile(id) ? "video" : "image"} · ${size}`;
  }
  if (family === "slack") {
    if (isVideoProfile(id)) return `Slack file video · ${size}`;
    if (id.endsWith("file-upload")) return `Slack file · ${size}`;
    return `Slack file image · ${size}`;
  }
  if (family === "whatsapp") {
    if (isVideoProfile(id)) return `WhatsApp video · ${size}`;
    if (id.endsWith("/audio")) return `WhatsApp audio · ${size}`;
    return byteCap(id) === null ? "WhatsApp photo · JPEG" : `WhatsApp photo · ${size} JPEG`;
  }
  if (family === "x") return `X ${isVideoProfile(id) ? "video" : "image"} · ${size}`;
  if (family === "jpeg") return `Web image · JPEG PNG WebP · ${size}`;
  return `Web video · MP4 · ${size}`;
}
