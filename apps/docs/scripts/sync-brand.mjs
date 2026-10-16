import { cp, mkdir, copyFile } from 'node:fs/promises';
await mkdir('public/brand', { recursive: true });
await cp('../../web/public/brand', 'public/brand', { recursive: true });
await copyFile('../../web/public/ft-mark.svg', 'public/brand/ft-mark.svg');
