import { useState } from "react";
import type { DoctorReport } from "../lib/desktop-engine";
import { FfmpegInstall } from "../FfmpegInstall";

export function ToolDiagnostics({
  doctor,
  onRecheck,
}: {
  doctor: DoctorReport | null;
  onRecheck?: () => void | Promise<void>;
}) {
  const [checking, setChecking] = useState(false);
  return (
    <div className="tool-diagnostics">
      <p role="status">
        {checking
          ? "Checking local video tools…"
          : doctor?.healthy
            ? "Video and audio tools are ready."
            : doctor
              ? "Video tools need attention. Photos still work."
              : "Video tool status is not available yet."}
      </p>
      {doctor ? (
        <details>
          <summary>Tool versions and diagnostics</summary>
          <ul>
            {doctor.tools.map((tool) => (
              <li key={tool.name}>
                <strong>{tool.name}</strong> ·{" "}
                {tool.available ? (tool.version ?? "Available") : "Unavailable"}
                {tool.detail ? <p>{tool.detail}</p> : null}
              </li>
            ))}
          </ul>
          {doctor.warnings.map((warning) => (
            <p key={warning}>{warning}</p>
          ))}
        </details>
      ) : null}
      {doctor && !doctor.healthy ? (
        <>
          <p>
            Reinstall this desktop build to restore its bundled FFmpeg and
            ffprobe tools, then check again.
          </p>
          <details>
            <summary>Command-line and development setup</summary>
            <FfmpegInstall />
          </details>
        </>
      ) : null}
      {onRecheck ? (
        <button
          className="secondary"
          disabled={checking}
          onClick={async () => {
            setChecking(true);
            try {
              await onRecheck();
            } finally {
              setChecking(false);
            }
          }}
        >
          {checking ? "Checking…" : "Check again"}
        </button>
      ) : null}
    </div>
  );
}
