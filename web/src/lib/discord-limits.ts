// Builds the /discord/ page's limits table from the shipped registry at build time, so the
// page states exactly the byte caps the engine checks against and can't drift from them.

interface RegistryRule {
  field: string;
  value: unknown;
  source_url: string | null;
}

interface RegistryProfile {
  id: string;
  last_verified: string;
  rules: RegistryRule[];
}

const TIERS = [
  ["discord/video-upload", "Free"],
  ["discord/video-upload-nitro-basic", "Nitro Basic"],
  ["discord/video-upload-nitro", "Nitro"],
] as const;

function escape(text: string): string {
  return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

function sizeLabel(bytes: number): string {
  return bytes >= 1e9 ? `${bytes / 1e9} GB` : `${bytes / 1e6} MB`;
}

function sourceLabel(url: string): string {
  const last = decodeURIComponent(new URL(url).pathname.split("/").filter(Boolean).at(-1) ?? "");
  return last.replace(/^[\d-]+/, "").replace(/[-_]+/g, " ").trim() || new URL(url).host;
}

function dateLabel(iso: string): string {
  return new Date(`${iso}T00:00:00Z`).toLocaleDateString("en-US", {
    timeZone: "UTC",
    year: "numeric",
    month: "long",
    day: "numeric",
  });
}

export function discordLimitsHtml(registry: { profiles: RegistryProfile[] }): string {
  const rows = TIERS.map(([id, tier]) => {
    const profile = registry.profiles.find((candidate) => candidate.id === id);
    const rule = profile?.rules.find((candidate) => candidate.field === "file.bytes");
    if (!profile || typeof rule?.value !== "number" || !rule.source_url) {
      throw new Error(`registry.json has no file.bytes rule with a source for ${id}`);
    }
    return { tier, bytes: rule.value, url: rule.source_url, checked: profile.last_verified };
  });
  const checked = rows.map((row) => row.checked).sort()[0];
  const body = rows
    .map(
      (row) =>
        `<tr><th scope="row">${row.tier}</th><td>${sizeLabel(row.bytes)}</td>` +
        `<td>${row.bytes.toLocaleString("en-US")} bytes</td>` +
        `<td><a href="${escape(row.url)}">${escape(sourceLabel(row.url))}</a></td></tr>`,
    )
    .join("");
  return (
    `<table class="limits-table"><caption>Discord's limits, checked ` +
    `<time datetime="${checked}">${dateLabel(checked)}</time></caption>` +
    `<thead><tr><th scope="col">Account</th><th scope="col">Upload limit</th>` +
    `<th scope="col">Fitifact aims at or under</th><th scope="col">Discord's page</th></tr></thead>` +
    `<tbody>${body}</tbody></table>`
  );
}
