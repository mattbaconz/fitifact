# Fitifact for Chrome

Attach a video on Discord, Gmail, or WhatsApp Web. If it's over that
destination's limit, Fitifact holds it before the site sees it, shows what
fails and what it would change, and fits it on your device. Only an output
that passes the same checks again reaches the page. A video under the limit is
never touched, and **Use original** hands over the file as it was.

- Runs only on `discord.com`, `mail.google.com`, and `web.whatsapp.com`. The
  one permission is `storage`, for the Discord plan you declare (Free, Nitro
  Basic, or Nitro). Fitifact never detects it.
- Video only, with the web app's engine and the browser's WebCodecs encoders
  (D-047). Images, sound, and other files pass straight through. On WhatsApp,
  a file sent as a document passes through too.
- Nothing is uploaded or sent anywhere. The extension's CSP allows no
  connection beyond its own files, and there is no telemetry.
- The toolbar popup sets your Discord plan and opens a tab where you can fit a
  video yourself and download it.

The limits come from the destination profiles bundled at build time
(`web/public/registry.json` and the WASM engine built from `profiles/`).

## Build

The extension reuses the web app's dependencies and engine, so build those
first:

```bash
npm ci --prefix web
npm run build:wasm --prefix web
npm ci --prefix apps/extension
npm run build --prefix apps/extension
```

Then open `chrome://extensions`, turn on Developer mode, choose
**Load unpacked**, and pick `apps/extension/dist`.

## Test

```bash
npm run test:e2e --prefix apps/extension
```

Playwright loads the unpacked build in Chromium and routes the three sites to
a local page, so nothing reaches them. The suite fails if any request leaves
the extension or that page. Firefox isn't supported yet.
