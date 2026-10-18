import { useState } from "react";
import { readStored, writeStored } from "../lib/storage";

export function ThemeControl() {
  const [theme, setTheme] = useState(
    () => readStored("fitifact.theme") ?? "system",
  );
  return (
    <label className="theme-control">
      Appearance
      <select className="ft-field"
        value={theme}
        onChange={(event) => {
          const value = event.target.value;
          setTheme(value);
          writeStored("fitifact.theme", value);
          document.documentElement.dataset.theme = value;
        }}
      >
        <option value="dark">Dark</option>
        <option value="light">Light</option>
        <option value="system">System</option>
      </select>
    </label>
  );
}
