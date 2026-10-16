import type { Metadata } from "next";
import { DocsLayout } from "fumadocs-ui/layouts/docs";
import { source } from "@/lib/source";
import { Providers, Appearance } from "./providers";
import "./styles.css";

export const metadata: Metadata = {
  title: { default: "Fitifact docs", template: "%s · Fitifact" },
  description:
    "Make files meet their destination’s requirements with the minimum necessary transformation. Local processing, inspectable plans, validated results.",
};

export default function Layout({ children }: { children: React.ReactNode }) {
  const base = process.env.NEXT_PUBLIC_DOCS_BASE!;
  return (
    <html lang="en" data-theme="dark" suppressHydrationWarning>
      <head>
        <link rel="stylesheet" href={`${base}/brand/tokens.css`} />
        <link rel="icon" href={`${base}/brand/mark.svg`} type="image/svg+xml" />
        <script src={`${base}/brand/theme.js`} />
      </head>
      <body>
        <Providers>
          <a className="fitifact-skip" href="#nd-page">
            Skip to documentation
          </a>
          <DocsLayout
            tree={source.pageTree}
            nav={{
              title: (
                <>
                  <span className="fitifact-mark">ft.</span> Fitifact{" "}
                  <span className="fitifact-docs-label">/ Docs</span>
                </>
              ),
            }}
            themeSwitch={{ enabled: false }}
            links={[
              {
                type: "custom",
                children: (
                  <a
                    className="fitifact-app-link"
                    href={process.env.NEXT_PUBLIC_APP_BASE}
                  >
                    Open Fitifact ↗
                  </a>
                ),
              },
              { type: "custom", secondary: true, children: <Appearance /> },
            ]}
          >
            {children}
          </DocsLayout>
        </Providers>
      </body>
    </html>
  );
}
