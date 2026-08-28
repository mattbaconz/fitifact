import {
  DESTINATION_FAMILIES,
  type DestinationFamily,
  type DiscordCap,
} from "./destinations";

const SETUP_KEY = "fitifact.setup.v1";

export interface SetupState {
  completed: boolean;
  families: DestinationFamily[];
  discordCap: DiscordCap;
  updatedAt: string;
}

export const DEFAULT_SETUP: SetupState = {
  completed: false,
  families: [...DESTINATION_FAMILIES],
  discordCap: "free",
  updatedAt: "",
};

function isFamily(value: unknown): value is DestinationFamily {
  return DESTINATION_FAMILIES.includes(value as DestinationFamily);
}

function isCap(value: unknown): value is DiscordCap {
  return value === "free" || value === "nitro-basic" || value === "nitro";
}

function readJson(key: string): unknown {
  try {
    const raw = localStorage.getItem(key);
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

export function loadSetup(): SetupState {
  const parsed = readJson(SETUP_KEY);
  if (!parsed || typeof parsed !== "object") return { ...DEFAULT_SETUP, families: [...DEFAULT_SETUP.families] };
  const candidate = parsed as Partial<SetupState>;
  const families = Array.isArray(candidate.families)
    ? candidate.families.filter(isFamily)
    : [...DEFAULT_SETUP.families];
  return {
    completed: candidate.completed === true,
    families: families.length ? families : [...DEFAULT_SETUP.families],
    discordCap: isCap(candidate.discordCap) ? candidate.discordCap : "free",
    updatedAt: typeof candidate.updatedAt === "string" ? candidate.updatedAt : "",
  };
}

export function saveSetup(setup: SetupState) {
  localStorage.setItem(SETUP_KEY, JSON.stringify(setup));
}

export function declareSetup(partial: Partial<SetupState>, previous = loadSetup()): SetupState {
  const next: SetupState = {
    completed: partial.completed ?? previous.completed,
    families: partial.families ? [...partial.families] : [...previous.families],
    discordCap: partial.discordCap ?? previous.discordCap,
    updatedAt: new Date().toISOString(),
  };
  saveSetup(next);
  return next;
}

export function setupIsNewer(setup: SetupState, savedAt: string | undefined): boolean {
  if (!setup.updatedAt || !savedAt) return Boolean(setup.completed && setup.updatedAt && !savedAt);
  return setup.updatedAt > savedAt;
}
