import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";
import { discordLimitsHtml } from "./src/lib/discord-limits";

export default defineConfig(({ mode }) => {
  const env = { ...process.env, ...loadEnv(mode, process.cwd(), "") };
  return {
    base: env.FITIFACT_BASE_PATH || "/",
    plugins: [
      react(),
      {
        // The web build connects only to itself. The desktop webview also talks to
        // its host over Tauri IPC, and the dev server needs inline styles and HMR.
        name: "fitifact-csp",
        transformIndexHtml(html, ctx) {
          const connect = ["'self'"];
          if (env.TAURI_ENV_PLATFORM) connect.push("ipc:", "http://ipc.localhost");
          if (ctx.server) connect.push("ws:", "wss:");
          const out = html.replace("connect-src 'self'", `connect-src ${connect.join(" ")}`);
          return ctx.server
            ? out.replace("style-src 'self'", "style-src 'self' 'unsafe-inline'")
            : out;
        },
      },
      {
        // /discord/ states the caps the engine checks, read from the shipped registry.
        name: "fitifact-discord-limits",
        transformIndexHtml(html) {
          if (!html.includes("<!--discord-limits-->")) return html;
          const registry = JSON.parse(readFileSync(new URL("./public/registry.json", import.meta.url), "utf8"));
          return html.replace("<!--discord-limits-->", discordLimitsHtml(registry));
        },
      },
    ],
    define: {
      __FITIFACT_HEIC_APPROVED__: JSON.stringify(env.FITIFACT_HEIC_APPROVED !== "false"),
      __FITIFACT_VERSION__: JSON.stringify(
        JSON.parse(readFileSync(new URL("./package.json", import.meta.url), "utf8")).version,
      ),
    },
    optimizeDeps: {
      exclude: ["@tauri-apps/api", "@tauri-apps/plugin-dialog"],
    },
    server: {
      port: 5173,
      strictPort: true,
    },
    build: {
      target: "es2022",
      sourcemap: true,
      rolldownOptions: {
        input: {
          main: fileURLToPath(new URL("./index.html", import.meta.url)),
          discord: fileURLToPath(new URL("./discord/index.html", import.meta.url)),
        },
      },
    },
    worker: {
      format: "es",
    },
    test: {
      environment: "node",
      include: ["src/**/*.test.ts"],
    },
  };
});
