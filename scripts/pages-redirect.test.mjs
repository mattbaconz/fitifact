// The GitHub Pages stub sends every old /fitifact/ URL to the same place on fitifact.com (D-046).
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const html = readFileSync(new URL('./pages-redirect.html', import.meta.url), 'utf8');
const script = html.match(/<script>([^<]*)<\/script>/)[1];

function redirect(href) {
  const url = new URL(href);
  let target;
  const location = { pathname: url.pathname, search: url.search, hash: url.hash, replace: (to) => { target = to; } };
  new Function('location', script)(location);
  return target;
}

test('the content security policy allows exactly the inline redirect script', () => {
  const hash = createHash('sha256').update(script).digest('base64');
  assert.match(html, new RegExp(`script-src 'sha256-${hash.replace(/[+/=]/g, '\\$&')}'`));
});

test('old Pages paths keep their path, query, and hash on fitifact.com', () => {
  const base = 'https://mattbaconz.github.io';
  assert.equal(redirect(`${base}/fitifact/`), 'https://fitifact.com/');
  assert.equal(redirect(`${base}/fitifact`), 'https://fitifact.com/');
  assert.equal(redirect(`${base}/fitifact/?for=discord`), 'https://fitifact.com/?for=discord');
  assert.equal(redirect(`${base}/fitifact/docs/cli/#install`), 'https://fitifact.com/docs/cli/#install');
  assert.equal(redirect(`${base}/fitifactory/`), 'https://fitifact.com/fitifactory/');
});
