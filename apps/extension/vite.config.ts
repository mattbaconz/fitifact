import { readFileSync } from "node:fs";
import { defineConfig, type Plugin } from "vite";
import { contentSites, SITES } from "./src/sites";

const pkg = JSON.parse(readFileSync(new URL("./package.json", import.meta.url), "utf8")) as { version: string };
const hosts = Object.keys(SITES).map((host) => `https://${host}/*`);
const ICONS = ["32x32.png", "64x64.png", "128x128.png"];

// Nothing leaves the extension: no remote code, no requests beyond its own files.
const CSP = [
  "default-src 'self'",
  "script-src 'self' 'wasm-unsafe-eval'",
  "style-src 'self'",
  "img-src 'self' blob: data:",
  "media-src 'self' blob:",
  "worker-src 'self'",
  "connect-src 'self'",
  "object-src 'none'",
  "base-uri 'none'",
  "form-action 'none'",
  `frame-ancestors ${Object.keys(SITES).map((host) => `https://${host}`).join(" ")}`,
].join("; ");

function manifest(): Plugin {
  return {
    name: "fitifact-manifest",
    generateBundle() {
      const icons: Record<string, string> = {};
      for (const file of ICONS) {
        icons[file.split("x")[0]] = `icons/${file}`;
        this.emitFile({
          type: "asset",
          fileName: `icons/${file}`,
          source: readFileSync(new URL(`../desktop/src-tauri/icons/${file}`, import.meta.url)),
        });
      }
      this.emitFile({
        type: "asset",
        fileName: "manifest.json",
        source: JSON.stringify(
          {
            manifest_version: 3,
            name: "Fitifact",
            version: pkg.version.split("-")[0],
            version_name: pkg.version,
            description:
              "Fits a video to Discord, Gmail, or WhatsApp Web before it uploads. Everything runs on your device.",
            icons,
            action: { default_title: "Fitifact", default_popup: "popup.html", default_icon: icons },
            permissions: ["storage"],
            content_scripts: [{ matches: hosts, js: ["content.js"], run_at: "document_start" }],
            web_accessible_resources: [{ resources: ["panel.html"], matches: hosts }],
            content_security_policy: { extension_pages: CSP },
          },
          null,
          2,
        ),
      });
    },
  };
}

export default defineConfig(({ mode }) =>
  mode === "content"
    ? {
        // A content script can't be a module: one self-contained file, with the caps baked in.
        define: { __FITIFACT_SITES__: JSON.stringify(contentSites()) },
        build: {
          outDir: "dist",
          emptyOutDir: false,
          copyPublicDir: false,
          target: "es2022",
          lib: { entry: "src/content.ts", formats: ["iife"], name: "fitifactContent", fileName: () => "content.js" },
        },
      }
    : {
        publicDir: false,
        plugins: [manifest()],
        define: {
          // Video only (D-047): leave the lazy HEIC decoder out of the package.
          __FITIFACT_HEIC_APPROVED__: "false",
          __FITIFACT_VERSION__: JSON.stringify(pkg.version),
        },
        build: {
          outDir: "dist",
          emptyOutDir: true,
          target: "es2022",
          // Preloads made by a framed extension page go unused ("cross-world" mismatch); files are local anyway.
          modulePreload: false,
          rollupOptions: { input: { panel: "panel.html", popup: "popup.html" } },
        },
        worker: { format: "es" },
      },
);
