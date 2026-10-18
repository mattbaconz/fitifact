import { test } from 'node:test';
import assert from 'node:assert/strict';

import {
  classify,
  needsReview,
  pageText,
  quotedPhrases,
  renderReport,
  sourcesToCheck,
  staleProfiles,
} from './check-source-freshness.mjs';

const filler = '<p>' + 'Other help text. '.repeat(40) + '</p>';

const registry = {
  profiles: [
    {
      id: 'example/video',
      last_verified: '2026-10-01',
      sources: [
        { id: 'docs', url: 'https://example.test/limits', note: '"Videos can be up to 25 MB." Fitifact uses 25000000.' },
        { id: 'rule', url: null, note: 'Fitifact rule.' },
      ],
    },
    {
      id: 'example/image',
      last_verified: '2026-08-01',
      sources: [{ id: 'docs', url: 'https://example.test/limits', note: 'Images: "Supported formats are `png`, `jpeg`."' }],
    },
  ],
};

test('page text drops markup, scripts, and entities', () => {
  const html = '<script>var x = "Videos can be up to 25 MB.";</script><p>Videos&nbsp;can be up to <b>25&#160;MB</b>.</p>';
  assert.equal(pageText(html), 'videos can be up to 25 mb.');
  assert.equal(pageText('<p>Formats are <code>mp3</code>, <code>wav</code>.</p>'), 'formats are mp3, wav.');
  assert.equal(pageText('<p>It’s “fine” — really</p>'), 'it\'s "fine" - really');
});

test('quoted phrases come from notes, normalized and without trailing punctuation', () => {
  assert.deepEqual(quotedPhrases('"Videos can be up to 25 MB." Fitifact uses 25000000.'), ['videos can be up to 25 mb']);
  assert.deepEqual(quotedPhrases('No quotes here, and "short" ones are skipped.'), []);
  assert.deepEqual(quotedPhrases(null), []);
});

test('sources are grouped by URL with every citing profile', () => {
  const [entry] = sourcesToCheck(registry);
  assert.equal(entry.url, 'https://example.test/limits');
  assert.deepEqual(entry.citedBy, ['example/video (docs)', 'example/image (docs)']);
  assert.deepEqual(entry.phrases, ['videos can be up to 25 mb', 'supported formats are `png`, `jpeg`']);
});

test('a source is changed only when a quoted sentence is missing from a readable page', () => {
  const entry = { url: 'https://example.test/limits', citedBy: ['example/video (docs)'], phrases: ['videos can be up to 25 mb'] };
  assert.equal(classify(entry, { status: 200, html: `${filler}<p>Videos can be up to 25 MB.</p>` }).state, 'ok');
  const changed = classify(entry, { status: 200, html: `${filler}<p>Videos can be up to 50 MB.</p>` });
  assert.equal(changed.state, 'changed');
  assert.deepEqual(changed.missing, ['videos can be up to 25 mb']);
  assert.equal(classify(entry, { status: 404, html: '' }).state, 'gone');
  assert.equal(classify(entry, { status: 403, html: 'blocked' }).state, 'unchecked');
  assert.equal(classify(entry, { error: 'timeout' }).state, 'unchecked');
  assert.equal(classify(entry, { status: 200, html: '<div id="root"></div>' }).state, 'unchecked');
  assert.equal(classify({ ...entry, phrases: [] }, { status: 200, html: filler }).state, 'unwatched');
});

test('stale profiles and changed sources need review; unchecked ones alone do not', () => {
  const stale = staleProfiles(registry, '2026-10-18');
  assert.deepEqual(stale.map((profile) => profile.id), ['example/image']);
  assert.equal(needsReview([{ state: 'unchecked' }, { state: 'ok' }], []), false);
  assert.equal(needsReview([{ state: 'changed' }], []), true);
  assert.equal(needsReview([{ state: 'gone' }], []), true);
  assert.equal(needsReview([], stale), true);
  assert.equal(staleProfiles({ profiles: [{ id: 'x/y', last_verified: 'not a date' }] }, '2026-10-18').length, 1);
});

test('the report names what changed, who cites it, and what to do', () => {
  const results = [
    { url: 'https://example.test/limits', citedBy: ['example/video (docs)'], state: 'changed', detail: 'quoted text is no longer on the page', missing: ['videos can be up to 25 mb'] },
    { url: 'https://example.test/blocked', citedBy: ['example/image (docs)'], state: 'unchecked', detail: 'HTTP 403' },
  ];
  const report = renderReport(results, staleProfiles(registry, '2026-10-18'), '2026-10-18');
  assert.match(report, /## Sources that changed/);
  assert.match(report, /Not found: "videos can be up to 25 mb"/);
  assert.match(report, /Cited by example\/video \(docs\)/);
  assert.match(report, /example\/image, last verified 2026-08-01 \(78 days\)/);
  assert.match(report, /https:\/\/example\.test\/blocked \(HTTP 403\)/);
  assert.match(report, /FITIFACT_BLESS=1 cargo test -p fitifact --lib registry/);
});
