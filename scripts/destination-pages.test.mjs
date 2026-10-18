import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import { mdText, pageFiles, renderPage, ruleText, sourceName } from './destination-pages.mjs';

const profile = {
  id: 'discord/video-upload-nitro',
  name: 'Discord Nitro video upload',
  revision: 2,
  last_verified: '2026-10-17',
  trust: 'documented-with-rules',
  rules: [
    { id: 'container', field: 'media.container', op: 'in', value: ['mp4'], source_type: 'official-doc', source_url: 'https://example.test/faq' },
    { id: 'bytes', field: 'file.bytes', op: 'lte', value: 1_000_000_000, source_type: 'official-doc', source_url: 'https://example.test/faq' },
    { id: 'codec', field: 'media.video.codec', op: 'in', value: ['h264'], source_type: 'fitifact-rule', source_url: null },
  ],
  sources: [
    { id: 'faq', type: 'official-doc', url: 'https://example.test/faq', observed_at: '2026-10-17', note: '"Up to 1GB" for <Nitro> {members}.' },
    { id: 'rule', type: 'fitifact-rule', url: null, observed_at: '2026-10-17', note: 'Fitifact rule: H.264 plays everywhere.' },
  ],
  history: [
    { revision: 2, date: '2026-10-17', change: 'Raised the size limit from 500 MB to 1 GB.' },
    { revision: 1, date: '2026-08-29', change: 'First published.' },
  ],
};

test('text is escaped so MDX shows it literally', () => {
  assert.equal(mdText('<b> {x} | a_b *c* `d` & #e'), '\\<b\\> \\{x\\} \\| a\\_b \\*c\\* \\`d\\` \\& \\#e');
  assert.equal(mdText('two\nlines'), 'two lines');
});

test('sources are named by host and page', () => {
  assert.equal(sourceName('https://support.discord.com/hc/en-us/articles/25444343291031-File-Attachments-FAQ'), 'support.discord.com: File Attachments FAQ');
  assert.equal(sourceName('https://support.google.com/mail/answer/6584'), 'support.google.com');
  assert.equal(sourceName('https://example.test/'), 'example.test');
});

test('rules read in plain words', () => {
  assert.deepEqual(ruleText({ field: 'file.bytes', op: 'lte', value: 20_000_000 }), ['File size', 'At most 20 MB (20,000,000 bytes)']);
  assert.deepEqual(ruleText({ field: 'file.bytes', op: 'lte', value: 2_000_000_000 }), ['File size', 'At most 2 GB (2,000,000,000 bytes)']);
  assert.deepEqual(ruleText({ field: 'file.bytes', op: 'lte', value: 7_500_001 }), ['File size', 'At most 7,500,001 bytes']);
  assert.deepEqual(ruleText({ field: 'file.base64_bytes', op: 'lte', value: 10_000_000 }), [
    'Base64 size',
    'At most 10,000,000 characters once base64-encoded, so files up to 7,500,000 bytes',
  ]);
  assert.deepEqual(ruleText({ field: 'image.format', op: 'in', value: ['jpeg', 'png', 'gif', 'webp'] }), ['Format', 'JPEG, PNG, GIF, or WebP']);
  assert.deepEqual(ruleText({ field: 'media.video.hdr', op: 'in', value: ['sdr', 'unknown'] }), ['Dynamic range', 'SDR or untagged']);
  assert.deepEqual(ruleText({ field: 'image.width', op: 'lte', value: 8000 }), ['Width', 'At most 8,000 pixels']);
  assert.deepEqual(ruleText({ field: 'media.video.bit_depth', op: 'lte', value: 8 }), ['Bit depth', 'At most 8-bit']);
});

test('a page lists rules with sources, notes, history, and how to use it', () => {
  const page = renderPage(profile, '2026-10-18');
  assert.match(page, /^---\ntitle: "Discord Nitro video upload"\n/);
  assert.match(page, /- \*\*Profile:\*\* `discord\/video-upload-nitro`\n- \*\*Revision:\*\* 2, last verified 2026-10-17\n- \*\*Trust:\*\* Documented, plus Fitifact rules\. Documented limits/);
  assert.match(page, /\| File size \| At most 1 GB \(1,000,000,000 bytes\) \| \[Official help page\]\(https:\/\/example\.test\/faq\) \|/);
  assert.match(page, /\| Video \| H\.264 \| Fitifact rule \|/);
  assert.match(page, /- \*\*Official help page\*\*, checked 2026-10-17, \[example\.test: faq\]\(https:\/\/example\.test\/faq\)\. "Up to 1GB" for \\<Nitro\\> \\\{members\\\}\./);
  assert.match(page, /- \*\*Fitifact rule\*\*, recorded 2026-10-17\. Fitifact rule: H\.264 plays everywhere\./);
  assert.match(page, /\| 2 \| 2026-10-17 \| Raised the size limit from 500 MB to 1 GB\. \|\n\| 1 \| 2026-08-29 \| First published\. \|/);
  assert.match(page, /fitifact check clip\.mov --for discord\/video-upload-nitro/);
  assert.match(page, /choose Discord as the destination and set your Nitro plan in Setup/);
  assert.doesNotMatch(page, /Not re-checked/);
});

test('stale profiles warn, AI profiles skip the app hint, and history is required', () => {
  const ai = { ...profile, id: 'anthropic/vision', rules: [{ field: 'image.format', op: 'in', value: ['png'] }] };
  const page = renderPage(ai, '2026-12-01');
  assert.match(page, /<Callout type="warn" title="Not re-checked in over 30 days">/);
  assert.match(page, /fitifact check photo\.png --for anthropic\/vision/);
  assert.doesNotMatch(page, /browser and desktop apps/);
  assert.throws(() => renderPage({ ...profile, history: [] }, '2026-10-18'), /no change history/);
});

test('every shipped profile gets a page at its id, grouped by vendor', () => {
  const registry = JSON.parse(readFileSync(new URL('../web/public/registry.json', import.meta.url), 'utf8'));
  const files = new Map(pageFiles(registry, '2026-10-18'));
  for (const { id } of registry.profiles) assert.ok(files.has(`${id}.mdx`), id);
  const vendors = JSON.parse(files.get('meta.json')).pages;
  assert.deepEqual(vendors, [...vendors].sort((a, b) => a.localeCompare(b)));
  assert.deepEqual(JSON.parse(files.get('discord/meta.json')).pages.slice(0, 3), [
    'video-upload',
    'video-upload-nitro-basic',
    'video-upload-nitro',
  ]);
  assert.match(files.get('index.mdx'), /\| \[Claude API image\]\(anthropic\/vision\/\) \| `anthropic\/vision` \|/);
});
