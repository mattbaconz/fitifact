import { spawnSync } from 'node:child_process';
import { cp, mkdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
function run(directory, script) {
  const result = spawnSync(process.platform === 'win32' ? 'npm.cmd' : 'npm', ['run', script], { cwd: path.join(root, directory), stdio: 'inherit', shell: process.platform === 'win32', env: { ...process.env, NEXT_TELEMETRY_DISABLED: '1' } });
  if (result.status !== 0) process.exit(result.status ?? 1);
}
run('web', 'build:app');
run('apps/docs', 'build');
await mkdir(path.join(root, 'web/dist/docs'), { recursive: true });
await cp(path.join(root, 'apps/docs/out'), path.join(root, 'web/dist/docs'), { recursive: true });
await writeFile(path.join(root, 'web/dist/.nojekyll'), '');
console.log('Combined static app and documentation: web/dist');
