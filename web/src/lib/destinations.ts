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
export type FileKind = "image" | "video";

export const PROFILE_DOCUMENTED = "2026-08-27";
export const PROFILE_DOCUMENTED_NEW = "2026-08-28";

export const DISCORD_CAPS: Record<
  DiscordCap,
  { bytes: number; label: string; short: string }
> = {
  free: { bytes: 20_000_000, label: "Free", short: "20 MB" },
  "nitro-basic": { bytes: 50_000_000, label: "Nitro Basic", short: "50 MB" },
  nitro: { bytes: 500_000_000, label: "Nitro", short: "500 MB" },
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
  jpeg: "JPEG photo",
  "generic-video": "Generic video",
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
  if (family === "github") return "Using GitHub comment image.";
  if (family === "slack") return "Using Slack file image.";
  if (family === "whatsapp") return "Using WhatsApp photo.";
  if (family === "x") return "Using X image.";
  if (family === "jpeg") return "Using JPEG photo.";
  return "Using generic video.";
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
  if (family === "jpeg") return "Same as last time: JPEG photo.";
  return "Same as last time: Generic video.";
}

export function chipSubtitle(family: DestinationFamily, cap: DiscordCap): string {
  if (family === "discord") {
    return `Discord ${DISCORD_CAPS[cap].label.toLowerCase()} · ${DISCORD_CAPS[cap].short} · documented ${PROFILE_DOCUMENTED}`;
  }
  if (family === "gmail") return `Gmail · 25 MB · documented 2026-08-25`;
  if (family === "github") return `GitHub · 10 MB · documented ${PROFILE_DOCUMENTED_NEW}`;
  if (family === "slack") return `Slack · 1 GB · documented ${PROFILE_DOCUMENTED_NEW}`;
  if (family === "whatsapp") return `WhatsApp photo · 16 MB JPEG · documented ${PROFILE_DOCUMENTED_NEW}`;
  if (family === "x") return `X · 5 MB · documented ${PROFILE_DOCUMENTED_NEW}`;
  if (family === "jpeg") return "JPEG photo · 8 MB";
  return "Generic video · 25 MB MP4 H.264";
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
      return kind === "video" ? DISCORD_VIDEO[cap] : DISCORD_IMAGE[cap];
    case "gmail":
      return "gmail/attachment";
    case "github":
      return kind === "image" ? "github/comment-image" : null;
    case "slack":
      return kind === "image" ? "slack/file-image" : null;
    case "whatsapp":
      return kind === "image" ? "whatsapp/photo" : null;
    case "x":
      return kind === "image" ? "x/image" : null;
    case "jpeg":
      return kind === "image" ? "jpeg/photo-upload" : null;
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
  if (id === "github/comment-image") return "github";
  if (id === "slack/file-image") return "slack";
  if (id === "whatsapp/photo") return "whatsapp";
  if (id === "x/image") return "x";
  if (id === "jpeg/photo-upload") return "jpeg";
  if (id === "generic/video-upload") return "generic-video";
  return null;
}

export function isImageCapableProfile(id: string): boolean {
  const family = familyForProfile(id);
  return family !== null && family !== "generic-video";
}

export function isVideoProfile(id: string): boolean {
  return id.includes("/video-upload");
}
