// Generates one documentation page per destination profile in web/public/registry.json: each
// requirement with its source, the source notes, and the profile's change history, plus an
// index. The pages are written to apps/docs/content/docs/destinations/, which is gitignored
// and rebuilt before every docs build, so they always match the shipped profiles.
//
//   node scripts/destination-pages.mjs

import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

import { STALE_AFTER_DAYS, daysBetween } from './check-source-freshness.mjs';

// Labels match the app's wording in web/src/lib/explain.ts and web/src/lib/registry.ts.
const FIELD_LABEL = {
  'image.format': 'Format',
  'image.width': 'Width',
  'image.height': 'Height',
  'file.bytes': 'File size',
  'file.base64_bytes': 'Base64 size',
  'media.container': 'File type',
  'media.video.codec': 'Video',
  'media.audio.codec': 'Sound',
  'media.video.width': 'Width',
  'media.video.height': 'Height',
  'media.video.pixel_format': 'Picture format',
  'media.video.bit_depth': 'Bit depth',
  'media.video.hdr': 'Dynamic range',
};
const VALUE_LABEL = {
  jpeg: 'JPEG',
  png: 'PNG',
  gif: 'GIF',
  webp: 'WebP',
  mp4: 'MP4',
  mov: 'MOV',
  webm: 'WebM',
  mkv: 'MKV',
  mp3: 'MP3',
  wav: 'WAV',
  h264: 'H.264',
  aac: 'AAC',
  yuv420p: '4:2:0 (yuv420p)',
  yuvj420p: '4:2:0 full range (yuvj420p)',
  sdr: 'SDR',
  unknown: 'untagged',
};
const PIXEL_FIELDS = new Set(['image.width', 'image.height', 'media.video.width', 'media.video.height']);
const BOUND = { lte: 'At most', gte: 'At least', eq: 'Exactly' };
const SOURCE_LABEL = {
  'official-doc': 'Official help page',
  'official-api': 'Official API docs',
  'fitifact-rule': 'Fitifact rule',
  'user-config': 'Starter value',
  'observed-page': 'Observed page',
  'acceptance-test': 'Acceptance test',
  community: 'Community report',
};
const TRUST = {
  documented: ['Documented', "Every rule comes from the destination's own documentation."],
  'documented-with-rules': [
    'Documented, plus Fitifact rules',
    'Documented limits plus Fitifact rules that keep the file playable everywhere.',
  ],
  starter: ['Starter', "A starter target, not a named destination's published rules."],
  observed: ['Partly observed', 'Some rules come from observation, not documentation.'],
};
const VENDOR_LABEL = {
  anthropic: 'Anthropic',
  discord: 'Discord',
  generic: 'Generic',
  github: 'GitHub',
  gmail: 'Gmail',
  jpeg: 'JPEG',
  openai: 'OpenAI',
  slack: 'Slack',
  whatsapp: 'WhatsApp',
  x: 'X',
};
// How the apps pick each destination: the Setup label and the `?for=` value
// (web/src/lib/destinations.ts). Keyed by profile id first, then by vendor.
const APP_DESTINATION = {
  discord: ['Discord', 'discord'],
  gmail: ['Gmail', 'gmail'],
  github: ['GitHub', 'github'],
  slack: ['Slack', 'slack'],
  whatsapp: ['WhatsApp', 'whatsapp'],
  x: ['X', 'x'],
  'generic/web-image': ['Web image', 'web-image'],
  'generic/video-upload': ['Web video', 'web-video'],
};

/** Text that MDX shows literally: Markdown and JSX characters are escaped. */
export function mdText(text) {
  return String(text).replace(/\s+/g, ' ').replace(/[\\`*_{}[\]<>|~&#]/g, '\\$&');
}

const safeUrl = (url) => url.replace(/[ ()]/g, (character) => encodeURIComponent(character));
const number = (value) => value.toLocaleString('en-US');
const vendorOf = (id) => id.split('/')[0];

function orList(items) {
  if (items.length < 3) return items.join(' or ');
  return `${items.slice(0, -1).join(', ')}, or ${items.at(-1)}`;
}

function bytesLabel(bytes) {
  if (bytes % 1_000_000_000 === 0) return `${bytes / 1_000_000_000} GB (${number(bytes)} bytes)`;
  if (bytes % 1_000_000 === 0) return `${bytes / 1_000_000} MB (${number(bytes)} bytes)`;
  return `${number(bytes)} bytes`;
}

/** A rule in plain words: [requirement, rule]. */
export function ruleText(rule) {
  const label = FIELD_LABEL[rule.field] ?? rule.field;
  if (Array.isArray(rule.value)) return [label, orList(rule.value.map((value) => VALUE_LABEL[value] ?? value))];
  const bound = BOUND[rule.op] ?? rule.op;
  if (rule.field === 'file.bytes') return [label, `${bound} ${bytesLabel(rule.value)}`];
  if (rule.field === 'file.base64_bytes') {
    // A ceiling of N characters admits files of up to 3 * floor(N / 4) bytes.
    const raw = Math.floor(rule.value / 4) * 3;
    return [label, `${bound} ${number(rule.value)} characters once base64-encoded, so files up to ${number(raw)} bytes`];
  }
  if (rule.field === 'media.video.bit_depth') return [label, `${bound} ${rule.value}-bit`];
  if (PIXEL_FIELDS.has(rule.field)) return [label, `${bound} ${number(rule.value)} pixels`];
  return [label, `${bound} ${number(rule.value)}`];
}

function sourceCell(rule) {
  const label = SOURCE_LABEL[rule.source_type] ?? 'Unrecorded source';
  return rule.source_url ? `[${label}](${safeUrl(rule.source_url)})` : label;
}

/** "support.discord.com: File Attachments FAQ", from the page's last path segment. */
export function sourceName(url) {
  const { host, pathname } = new URL(url);
  const last = decodeURIComponent(pathname.split('/').filter(Boolean).at(-1) ?? '');
  const slug = last.replace(/^[\d-]+/, '').replace(/[-_]+/g, ' ').trim();
  return slug ? `${host}: ${slug}` : host;
}

function exampleFile(profile) {
  const fields = profile.rules.map((rule) => rule.field);
  if (fields.includes('image.format')) return 'photo.png';
  if (fields.some((field) => field.startsWith('media.video.'))) return 'clip.mov';
  return 'voice.m4a';
}

export function renderPage(profile, today) {
  if (!profile.history?.length) throw new Error(`${profile.id} has no change history`);
  const [trustName, trustText] = TRUST[profile.trust] ?? [profile.trust, ''];
  const lines = [
    '---',
    `title: ${JSON.stringify(profile.name)}`,
    `description: ${JSON.stringify(`What Fitifact checks for ${profile.id}, where each rule comes from, and how it changed.`)}`,
    '---',
    '',
  ];
  if (!(daysBetween(profile.last_verified, today) <= STALE_AFTER_DAYS)) {
    lines.push(
      `<Callout type="warn" title="Not re-checked in over ${STALE_AFTER_DAYS} days">Verify these limits against the sources below before relying on them.</Callout>`,
      '',
    );
  }
  lines.push(
    `- **Profile:** \`${profile.id}\``,
    `- **Revision:** ${profile.revision}, last verified ${profile.last_verified}`,
    `- **Trust:** ${mdText(trustName)}. ${mdText(trustText)}`,
    '',
    '## Requirements',
    '',
    '| Requirement | Rule | Source |',
    '| --- | --- | --- |',
  );
  for (const rule of profile.rules) {
    const [label, text] = ruleText(rule);
    lines.push(`| ${mdText(label)} | ${mdText(text)} | ${sourceCell(rule)} |`);
  }
  lines.push('', '## Sources', '');
  for (const source of profile.sources) {
    const label = SOURCE_LABEL[source.type] ?? 'Unrecorded source';
    const where = source.url
      ? `checked ${source.observed_at}, [${mdText(sourceName(source.url))}](${safeUrl(source.url)})`
      : `recorded ${source.observed_at}`;
    lines.push(`- **${label}**, ${where}.${source.note ? ` ${mdText(source.note)}` : ''}`);
  }
  lines.push('', '## Change history', '', '| Revision | Date | Change |', '| --- | --- | --- |');
  for (const entry of profile.history) lines.push(`| ${entry.revision} | ${entry.date} | ${mdText(entry.change)} |`);
  const file = exampleFile(profile);
  lines.push(
    '',
    '## Use it',
    '',
    '```sh',
    `fitifact check ${file} --for ${profile.id}`,
    `fitifact adapt ${file} --for ${profile.id}`,
    '```',
    '',
    `An AI assistant connected through [AI tools](../../../ai-tools/) passes \`${profile.id}\` as the \`destination\` of \`check\` or \`fit\`.`,
  );
  const app = APP_DESTINATION[profile.id] ?? APP_DESTINATION[vendorOf(profile.id)];
  if (app) {
    const plan = profile.id.includes('nitro') ? ' and set your Nitro plan in Setup' : '';
    lines.push(
      '',
      `In the browser and desktop apps, choose ${app[0]} as the destination${plan}. A link to the browser app can preselect it with \`?for=${app[1]}\`.`,
    );
  }
  lines.push('');
  return lines.join('\n');
}

function byVendor(profiles) {
  const groups = new Map();
  for (const profile of profiles) {
    const vendor = vendorOf(profile.id);
    groups.set(vendor, [...(groups.get(vendor) ?? []), profile]);
  }
  const label = (vendor) => VENDOR_LABEL[vendor] ?? vendor;
  return [...groups].sort(([a], [b]) => label(a).localeCompare(label(b)));
}

export function renderIndex(profiles) {
  const lines = [
    '---',
    'title: Destinations',
    'description: Every destination Fitifact checks, with its rules, their sources, and their change history.',
    '---',
    '',
    'Each destination is a profile: a short list of requirements, each tied to the page it comes from. Open one to see its rules, sources, and change history.',
    '',
  ];
  for (const [vendor, group] of byVendor(profiles)) {
    lines.push(`## ${VENDOR_LABEL[vendor] ?? vendor}`, '', '| Destination | Profile | Last verified |', '| --- | --- | --- |');
    for (const profile of group) {
      lines.push(`| [${mdText(profile.name)}](${profile.id}/) | \`${profile.id}\` | ${profile.last_verified} |`);
    }
    lines.push('');
  }
  lines.push(
    '## How the rules stay current',
    '',
    `Each source note quotes the sentences its rules rest on. Every week an automated check fetches each source page and looks for those sentences. A missing sentence, a page that is gone, or a profile not re-verified in ${STALE_AFTER_DAYS} days opens a tracking issue. A changed limit becomes a new profile revision with an entry in its change history, and these pages are rebuilt from the profiles whenever the site is built.`,
    '',
    `Some help centers block automated requests or show their text only in a browser. Those sources are re-checked by hand, and the ${STALE_AFTER_DAYS}-day limit still applies. Found a limit that changed? [Open an issue](https://github.com/mattbaconz/fitifact/issues/new).`,
    '',
  );
  return lines.join('\n');
}

/** Every generated file, as [path relative to the destinations folder, contents]. */
export function pageFiles(registry, today) {
  const groups = byVendor(registry.profiles);
  const files = [
    ['index.mdx', renderIndex(registry.profiles)],
    ['meta.json', `${JSON.stringify({ title: 'Destinations', pages: groups.map(([vendor]) => vendor) }, null, 2)}\n`],
  ];
  for (const [vendor, group] of groups) {
    const pages = group.map((profile) => profile.id.slice(vendor.length + 1));
    files.push([`${vendor}/meta.json`, `${JSON.stringify({ title: VENDOR_LABEL[vendor] ?? vendor, pages }, null, 2)}\n`]);
    for (const profile of group) files.push([`${profile.id}.mdx`, renderPage(profile, today)]);
  }
  return files;
}

async function main() {
  const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
  const registry = JSON.parse(await readFile(path.join(root, 'web/public/registry.json'), 'utf8'));
  const out = path.join(root, 'apps/docs/content/docs/destinations');
  await rm(out, { recursive: true, force: true });
  const files = pageFiles(registry, new Date().toISOString().slice(0, 10));
  for (const [relative, contents] of files) {
    await mkdir(path.dirname(path.join(out, relative)), { recursive: true });
    await writeFile(path.join(out, relative), contents);
  }
  console.log(`Destination pages: ${registry.profiles.length} profiles`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main();
}
