import { defineConfig, loadEnv } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), "");
  return {
    plugins: [react()],
    define: {
      __FITIFACT_HEIC_APPROVED__: JSON.stringify(env.FITIFACT_HEIC_APPROVED === "true"),
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
