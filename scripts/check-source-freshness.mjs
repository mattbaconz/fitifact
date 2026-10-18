// Checks every destination source cited in web/public/registry.json.
//
// A profile source's note quotes the sentences its rule rests on. This script fetches each
// source page and reports a source as changed when a quoted sentence is no longer on it, as
// gone when the page answers 404 or 410, and as unchecked when it cannot be fetched. Profiles
// whose rules were last verified more than 30 days ago are reported as stale. It writes a
// Markdown report and, in GitHub Actions, sets the `changed` output; it never edits files
// or opens issues itself.
//
//   node scripts/check-source-freshness.mjs [--report report.md] [--today YYYY-MM-DD]

import { readFile, writeFile, appendFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

export const STALE_AFTER_DAYS = 30;
const FETCH_TIMEOUT_MS = 20_000;
// Pages that render their text with JavaScript come back nearly empty; they need a person.
const MIN_PAGE_TEXT = 500;
const USER_AGENT = 'fitifact-source-freshness (+https://github.com/mattbaconz/fitifact)';
const ENTITIES = { amp: '&', lt: '<', gt: '>', quot: '"', apos: "'", nbsp: ' ', rsquo: "'", lsquo: "'", rdquo: '"', ldquo: '"', ndash: '-', mdash: '-', hellip: '...' };

/** Lowercased plain text with one kind of quote, dash, and space. */
export function normalize(text) {
  return text
    .normalize('NFKC')
    .replace(/[‘’‛′]/g, "'")
    .replace(/[“”‟″]/g, '"')
    .replace(/[‐-―−]/g, '-')
    .replace(/…/g, '...')
    .replace(/\s+/g, ' ')
    // Inline tags such as <code> leave spaces before punctuation; drop them.
    .replace(/ ([,.;:!?)\]])/g, '$1')
    .replace(/([(\[]) /g, '$1')
    .trim()
    .toLowerCase();
}

/** The visible text of an HTML page, without scripts, styles, or tags. */
export function pageText(html) {
  const withoutCode = html
    .replace(/<!--[\s\S]*?-->/g, ' ')
    .replace(/<(script|style|noscript|template)\b[\s\S]*?<\/\1\s*>/gi, ' ');
  const text = withoutCode
    .replace(/<[^>]+>/g, ' ')
    .replace(/&#x([0-9a-f]+);/gi, (_, hex) => String.fromCodePoint(Number.parseInt(hex, 16)))
    .replace(/&#(\d+);/g, (_, decimal) => String.fromCodePoint(Number(decimal)))
    .replace(/&([a-z]+);/gi, (match, name) => ENTITIES[name.toLowerCase()] ?? match);
  return normalize(text);
}

/** Sentences a note quotes from its source, the facts to look for on the page. */
export function quotedPhrases(note) {
  const phrases = [];
  for (const match of (note ?? '').matchAll(/"([^"]{8,})"/g)) {
    const phrase = normalize(match[1]).replace(/[.,;:]+$/, '');
    if (phrase.length >= 8) phrases.push(phrase);
  }
  return phrases;
}

/** One entry per source URL, with every profile that cites it and the phrases to find. */
export function sourcesToCheck(registry) {
  const byUrl = new Map();
  for (const profile of registry.profiles) {
    for (const source of profile.sources) {
      if (!source.url) continue;
      const entry = byUrl.get(source.url) ?? { url: source.url, citedBy: [], phrases: [] };
      entry.citedBy.push(`${profile.id} (${source.id})`);
      for (const phrase of quotedPhrases(source.note)) {
        if (!entry.phrases.includes(phrase)) entry.phrases.push(phrase);
      }
      byUrl.set(source.url, entry);
    }
  }
  return [...byUrl.values()];
}

/** Classifies one fetched source. `response` is { status, html } or { error }. */
export function classify(entry, response) {
  if (response.error) return { ...entry, state: 'unchecked', detail: response.error };
  if (response.status === 404 || response.status === 410) {
    return { ...entry, state: 'gone', detail: `HTTP ${response.status}` };
  }
  if (response.status < 200 || response.status >= 300) {
    return { ...entry, state: 'unchecked', detail: `HTTP ${response.status}` };
  }
  if (!entry.phrases.length) return { ...entry, state: 'unwatched', detail: 'no quoted sentence to look for' };
  const text = pageText(response.html);
  if (text.length < MIN_PAGE_TEXT) {
    return { ...entry, state: 'unchecked', detail: 'the page shows its text only in a browser' };
  }
  const missing = entry.phrases.filter((phrase) => !text.includes(phrase));
  return missing.length
    ? { ...entry, state: 'changed', detail: 'quoted text is no longer on the page', missing }
    : { ...entry, state: 'ok', detail: `${entry.phrases.length} quoted sentence(s) found` };
}

export function daysBetween(from, to) {
  return Math.floor((Date.parse(`${to}T00:00:00Z`) - Date.parse(`${from}T00:00:00Z`)) / 86_400_000);
}

export function staleProfiles(registry, today) {
  return registry.profiles
    .map((profile) => ({ id: profile.id, lastVerified: profile.last_verified, age: daysBetween(profile.last_verified, today) }))
    .filter((profile) => !(profile.age <= STALE_AFTER_DAYS));
}

/** True when something needs a person: a changed or gone source, or a stale profile. */
export function needsReview(results, stale) {
  return stale.length > 0 || results.some((result) => result.state === 'changed' || result.state === 'gone');
}

export function renderReport(results, stale, today) {
  const lines = [`# Destination source check, ${today}`, ''];
  const attention = results.filter((result) => result.state === 'changed' || result.state === 'gone');
  if (attention.length) {
    lines.push('## Sources that changed', '');
    for (const result of attention) {
      lines.push(`- ${result.url} (${result.state}: ${result.detail})`);
      lines.push(`  - Cited by ${result.citedBy.join(', ')}`);
      for (const phrase of result.missing ?? []) lines.push(`  - Not found: "${phrase}"`);
    }
    lines.push('');
  }
  if (stale.length) {
    lines.push(`## Profiles not verified in ${STALE_AFTER_DAYS} days`, '');
    for (const profile of stale) lines.push(`- ${profile.id}, last verified ${profile.lastVerified} (${profile.age} days)`);
    lines.push('');
  }
  if (!attention.length && !stale.length) lines.push('Every quoted source sentence was found, and no profile is stale.', '');
  const unchecked = results.filter((result) => result.state === 'unchecked' || result.state === 'unwatched');
  if (unchecked.length) {
    lines.push('## Not checked', '');
    for (const result of unchecked) lines.push(`- ${result.url} (${result.detail})`);
    lines.push('');
  }
  lines.push(
    '## What to do',
    '',
    'Re-read each changed source. If a limit moved, update the profile under `profiles/`, raise its `revision`, add a `history` entry, and set `last_verified` and `observed_at`. If only the wording moved, update the quoted sentence in the source `note`. Then regenerate `web/public/registry.json` with `FITIFACT_BLESS=1 cargo test -p fitifact --lib registry`.',
    '',
  );
  return lines.join('\n');
}

async function fetchSource(url) {
  try {
    const response = await fetch(url, {
      headers: { 'user-agent': USER_AGENT, accept: 'text/html,application/xhtml+xml', 'accept-language': 'en-US,en;q=0.9' },
      redirect: 'follow',
      signal: AbortSignal.timeout(FETCH_TIMEOUT_MS),
    });
    return { status: response.status, html: await response.text() };
  } catch (error) {
    return { error: error instanceof Error ? error.message : String(error) };
  }
}

async function main(argv) {
  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
  const option = (name) => {
    const index = argv.indexOf(name);
    return index === -1 ? null : argv[index + 1];
  };
  const today = option('--today') ?? new Date().toISOString().slice(0, 10);
  const registry = JSON.parse(await readFile(path.join(root, 'web/public/registry.json'), 'utf8'));
  const results = [];
  for (const entry of sourcesToCheck(registry)) results.push(classify(entry, await fetchSource(entry.url)));
  const stale = staleProfiles(registry, today);
  const report = renderReport(results, stale, today);
  const reportPath = option('--report');
  if (reportPath) await writeFile(reportPath, report);
  else process.stdout.write(report);
  const changed = needsReview(results, stale);
  if (process.env.GITHUB_OUTPUT) await appendFile(process.env.GITHUB_OUTPUT, `changed=${changed}\n`);
  if (process.env.GITHUB_STEP_SUMMARY) await appendFile(process.env.GITHUB_STEP_SUMMARY, report);
  for (const result of results) console.error(`${result.state.padEnd(9)} ${result.url}`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main(process.argv.slice(2));
}
