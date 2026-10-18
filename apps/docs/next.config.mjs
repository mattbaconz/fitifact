import { createMDX } from "fumadocs-mdx/next";
const withMDX = createMDX();
const appBase = (process.env.FITIFACT_BASE_PATH || "/").replace(/\/$/, "");
export default withMDX({
  output: "export",
  trailingSlash: true,
  basePath: `${appBase}/docs`,
  images: { unoptimized: true },
  env: {
    NEXT_PUBLIC_APP_BASE: `${appBase}/`,
    NEXT_PUBLIC_DOCS_BASE: `${appBase}/docs`,
  },
  experimental: { cpus: 2 },
});
