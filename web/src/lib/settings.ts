const SETTINGS_KEY = "fitifact.settings.v1";
const LAST_TARGET_KEY = "fitifact.last-target.v1";

export interface AppSettings {
  fitifactSuffix: boolean;
  firstFrameConsentDefault: boolean;
}

export interface LastTarget {
  requirements: string;
  profile?: string;
  constraintsJson?: string;
  savedAt?: string;
}

export const DEFAULT_SETTINGS: AppSettings = {
  fitifactSuffix: true,
  firstFrameConsentDefault: false,
};

function readJson(key: string): unknown {
  try {
    const raw = localStorage.getItem(key);
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

export function loadSettings(): AppSettings {
  const parsed = readJson(SETTINGS_KEY);
  if (!parsed || typeof parsed !== "object") return { ...DEFAULT_SETTINGS };
  const candidate = parsed as Partial<AppSettings>;
  return {
    fitifactSuffix: candidate.fitifactSuffix !== false,
    firstFrameConsentDefault: candidate.firstFrameConsentDefault === true,
  };
}

export function saveSettings(settings: AppSettings) {
  localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
}

export function loadLastTarget(): LastTarget | null {
  const parsed = readJson(LAST_TARGET_KEY);
  if (!parsed || typeof parsed !== "object") return null;
  const candidate = parsed as Partial<LastTarget>;
  if (typeof candidate.requirements !== "string") return null;
  const profile = typeof candidate.profile === "string" && candidate.profile.includes("/")
    ? candidate.profile
    : undefined;
  const constraintsJson = typeof candidate.constraintsJson === "string" ? candidate.constraintsJson : undefined;
  if (!profile && !constraintsJson?.trim()) return null;
  return {
    requirements: candidate.requirements,
    profile,
    constraintsJson: constraintsJson?.trim() ? constraintsJson : undefined,
    savedAt: typeof candidate.savedAt === "string" ? candidate.savedAt : undefined,
  };
}

export function saveLastTarget(target: LastTarget) {
  localStorage.setItem(LAST_TARGET_KEY, JSON.stringify(target));
}

export function clearLastTarget() {
  localStorage.removeItem(LAST_TARGET_KEY);
}
