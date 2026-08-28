import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig(({ mode }) => {
  const env = { ...process.env, ...loadEnv(mode, process.cwd(), "") };
  return {
    base: env.FITIFACT_BASE_PATH || "/",
    plugins: [
      react(),
      {
        name: "fitifact-dev-csp",
        transformIndexHtml(html, ctx) {
          if (!ctx.server) return html;
          return html
            .replace("style-src 'self'", "style-src 'self' 'unsafe-inline'")
            .replace(
              "connect-src 'self'",
              "connect-src 'self' ipc: http://ipc.localhost ws: wss:",
            );
        },
      },
    ],
    define: {
      __FITIFACT_HEIC_APPROVED__: JSON.stringify(env.FITIFACT_HEIC_APPROVED !== "false"),
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
