// Every declared version must match the Cargo workspace version (D-041).
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');
const cargoVersion = (path) => read(path).match(/^version = "([^"]+)"/m)?.[1];
const pin = (path) => read(path).match(/^fitifact = \{ version = "=([^"]+)"/m)?.[1];
const json = (path) => JSON.parse(read(path));

test('web, desktop, docs, and internal dependencies match the workspace version', () => {
  const version = cargoVersion('Cargo.toml');
  assert.match(version ?? '', /^\d+\.\d+\.\d+/);
  const declared = {
    'crates/fitifact-cli/Cargo.toml': pin('crates/fitifact-cli/Cargo.toml'),
    'crates/fitifact-wasm/Cargo.toml': pin('crates/fitifact-wasm/Cargo.toml'),
    'apps/desktop/src-tauri/Cargo.toml': cargoVersion('apps/desktop/src-tauri/Cargo.toml'),
    'apps/desktop/src-tauri/tauri.conf.json': json('apps/desktop/src-tauri/tauri.conf.json').version,
  };
  for (const app of ['web', 'apps/desktop', 'apps/docs']) {
    declared[`${app}/package.json`] = json(`${app}/package.json`).version;
    const lock = json(`${app}/package-lock.json`);
    declared[`${app}/package-lock.json`] = lock.version;
    declared[`${app}/package-lock.json packages[""]`] = lock.packages[''].version;
  }
  for (const [path, value] of Object.entries(declared)) assert.equal(value, version, path);
});
