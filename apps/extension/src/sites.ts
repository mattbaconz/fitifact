import { DISCORD_CAPS, profileForFamily, type DestinationFamily, type DiscordCap } from "../../../web/src/lib/destinations";
import { byteCap, capLabel } from "../../../web/src/lib/registry";

export const PLANS: DiscordCap[] = ["free", "nitro-basic", "nitro"];

export interface Site {
  family: DestinationFamily;
  /** Only inputs whose `accept` names video carry the profile; others (a WhatsApp document) pass untouched. */
  videoInputsOnly?: boolean;
}

/** The upload pages Fitifact watches. Their rules come from the shipped profiles, never from here. */
export const SITES: Record<string, Site> = {
  "discord.com": { family: "discord" },
  "mail.google.com": { family: "gmail" },
  "web.whatsapp.com": { family: "whatsapp", videoInputsOnly: true },
};

export function videoProfile(family: DestinationFamily, plan: DiscordCap): string {
  const id = profileForFamily(family, plan, "video");
  if (!id) throw new Error(`No video profile for ${family}`);
  return id;
}

export function videoCap(family: DestinationFamily, plan: DiscordCap): number {
  const cap = byteCap(videoProfile(family, plan));
  if (cap === null) throw new Error(`${family} has no video size limit`);
  return cap;
}

/** What the content script needs to decide on the spot, built into it at build time. */
export function contentSites() {
  return Object.fromEntries(
    Object.entries(SITES).map(([host, site]) => [
      host,
      { ...site, caps: Object.fromEntries(PLANS.map((plan) => [plan, videoCap(site.family, plan)])) },
    ]),
  );
}

const isPlan = (value: unknown): value is DiscordCap => PLANS.includes(value as DiscordCap);

/** The Discord plan the person declared; never detected (D-032). */
export async function loadPlan(): Promise<DiscordCap> {
  const { discordPlan } = await chrome.storage.local.get("discordPlan");
  return isPlan(discordPlan) ? discordPlan : "free";
}

export function savePlan(plan: DiscordCap) {
  return chrome.storage.local.set({ discordPlan: plan });
}

/** Fills a plan select with each plan's video cap, set to the declared plan. */
export async function fillPlans(select: HTMLSelectElement) {
  select.replaceChildren(
    ...PLANS.map((plan) => new Option(`${DISCORD_CAPS[plan].label} · ${capLabel(videoCap("discord", plan))}`, plan)),
  );
  select.value = await loadPlan();
}
