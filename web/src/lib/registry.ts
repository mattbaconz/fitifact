import registry from "../../public/registry.json";
import { formatBytes } from "./constraints";
import { checkLabel, formatCheckValue } from "./explain";

export interface RegistryRule {
  id: string;
  field: string;
  op: string;
  value: unknown;
  source: string | null;
  source_type: string | null;
  source_url: string | null;
  observed_at: string | null;
}

export interface RegistryProfile {
  id: string;
  name: string;
  revision: number;
  last_verified: string;
  trust: string;
  rules: RegistryRule[];
}

/** A profile not re-checked against its sources within this many days is shown as stale. */
export const STALE_AFTER_DAYS = 30;

const PROFILES = new Map(
  (registry.profiles as unknown as RegistryProfile[]).map((profile) => [profile.id, profile]),
);

export const SOURCE_LABEL: Record<string, string> = {
  "official-doc": "Official help page",
  "official-api": "Official API docs",
  "fitifact-rule": "Fitifact rule",
  "user-config": "Starter value",
  "observed-page": "Observed page",
  "acceptance-test": "Acceptance test",
  community: "Community report",
};

export const TRUST_LABEL: Record<string, string> = {
  documented: "Every rule comes from the destination's own documentation.",
  "documented-with-rules":
    "Documented limits plus Fitifact rules that keep the file playable everywhere.",
  "fitifact-rules":
    "None of these rules comes from the destination's documentation; each is Fitifact's own.",
  starter: "A starter target, not a named destination's published rules.",
  observed: "Some rules come from observation, not documentation.",
};

export function registryProfile(id: string | null | undefined): RegistryProfile | null {
  return (id && PROFILES.get(id)) || null;
}

export function daysSince(date: string, now = new Date()): number {
  const then = Date.parse(`${date}T00:00:00Z`);
  const today = Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate());
  return Number.isNaN(then) ? Number.POSITIVE_INFINITY : Math.max(0, Math.floor((today - then) / 86_400_000));
}

export function ageLabel(days: number): string {
  if (!Number.isFinite(days)) return "on an unknown date";
  if (days === 0) return "today";
  return days === 1 ? "1 day ago" : `${days} days ago`;
}

export function isStale(profile: RegistryProfile, now = new Date()): boolean {
  return daysSince(profile.last_verified, now) > STALE_AFTER_DAYS;
}

export function byteCap(id: string): number | null {
  const cap = PROFILES.get(id)?.rules.find((rule) => rule.field === "file.bytes" && rule.op === "lte")?.value;
  return typeof cap === "number" ? cap : null;
}

/** "20 MB", "1 GB": the published round numbers, not formatBytes' decimals. */
export function capLabel(bytes: number): string {
  return bytes >= 1_000_000_000 && bytes % 1_000_000_000 === 0
    ? `${bytes / 1_000_000_000} GB`
    : `${Math.round(bytes / 1_000_000)} MB`;
}

export function byteCapLabel(id: string): string {
  const cap = byteCap(id);
  return cap === null ? "no size limit" : capLabel(cap);
}

export function describeRule(rule: RegistryRule): string {
  const bound = rule.op === "lte" ? "at most " : rule.op === "gte" ? "at least " : "";
  const value =
    rule.field === "file.bytes"
      ? formatBytes(Number(rule.value))
      : formatCheckValue(rule.field, Array.isArray(rule.value) ? rule.value.join(", ") : String(rule.value));
  return `${checkLabel(rule.field)}: ${bound}${value}`;
}
