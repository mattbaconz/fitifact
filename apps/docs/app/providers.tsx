"use client";
import { RootProvider } from "fumadocs-ui/provider/next";
import { useEffect, useState } from "react";

export function Providers({ children }: { children: React.ReactNode }) {
  return (
    <RootProvider
      theme={{ enabled: false }}
      search={{
        options: {
          type: "static",
          api: `${process.env.NEXT_PUBLIC_DOCS_BASE}/search-index.json`,
        },
      }}
    >
      {children}
    </RootProvider>
  );
}

export function Appearance() {
  const [theme, setTheme] = useState("dark");
  useEffect(() => {
    setTheme(document.documentElement.dataset.theme || "dark");
  }, []);
  return (
    <label className="fitifact-appearance">
      Appearance
      <select
        value={theme}
        onChange={(event) => {
          const value = event.target.value;
          setTheme(value);
          document.documentElement.dataset.theme = value;
          try {
            localStorage.setItem("fitifact.theme", value);
          } catch {
            /* The current tab still changes. */
          }
        }}
      >
        <option value="dark">Dark</option>
        <option value="light">Light</option>
        <option value="system">System</option>
      </select>
    </label>
  );
}
