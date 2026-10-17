// Serve the exported artifact without SPA fallbacks or framework rewrites.
// This makes broken direct links and missing assets fail the same way they do
// on a static host. For local verification only.
import { createServer } from 'node:http';
import { createReadStream } from 'node:fs';
import { stat } from 'node:fs/promises';
import { createGzip } from 'node:zlib';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(process.env.FITIFACT_PREVIEW_DIR || fileURLToPath(new URL('../web/dist', import.meta.url)));
const base = (process.env.FITIFACT_BASE_PATH || '/').replace(/\/$/, '');
const port = Number(process.env.FITIFACT_PREVIEW_PORT || 4173);
// GitHub Pages gzips text responses; serving them the same way keeps local measurements honest.
const compressible = new Set(['.html', '.js', '.css', '.json', '.svg', '.wasm', '.txt', '.md']);
const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.json': 'application/json', '.wasm': 'application/wasm', '.woff2': 'font/woff2', '.svg': 'image/svg+xml', '.png': 'image/png', '.jpg': 'image/jpeg', '.txt': 'text/plain', '.md': 'text/plain' };

createServer(async (request, response) => {
  try {
    if (!['GET', 'HEAD'].includes(request.method)) {
      response.writeHead(405).end();
      return;
    }
    const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
    if (base && !pathname.startsWith(`${base}/`)) {
      if (pathname === base) response.writeHead(308, { Location: `${base}/` }).end();
      else response.writeHead(404).end('Not found');
      return;
    }
    let file = path.resolve(root, `.${pathname.slice(base.length)}`);
    if (file !== root && !file.startsWith(root + path.sep)) {
      response.writeHead(403).end();
      return;
    }
    let info = await stat(file);
    if (info.isDirectory()) {
      if (!pathname.endsWith('/')) {
        response.writeHead(308, { Location: `${pathname}/` }).end();
        return;
      }
      file = path.join(file, 'index.html');
      info = await stat(file);
    }
    if (!info.isFile()) throw new Error('Not a file');
    const gzip = compressible.has(path.extname(file)) && (request.headers['accept-encoding'] ?? '').includes('gzip');
    response.writeHead(200, {
      'Content-Type': types[path.extname(file)] || 'application/octet-stream',
      ...(gzip ? { 'Content-Encoding': 'gzip', Vary: 'Accept-Encoding' } : { 'Content-Length': info.size }),
      'Cache-Control': 'no-store',
    });
    if (request.method === 'HEAD') response.end();
    else {
      const stream = createReadStream(file).on('error', () => response.destroy());
      (gzip ? stream.pipe(createGzip()) : stream).pipe(response);
    }
  } catch {
    response.writeHead(404).end('Not found');
  }
}).listen(port, '127.0.0.1', () => {
  console.log(`Static artifact: http://127.0.0.1:${port}${base}/`);
});
