import { useState } from "react";
import { FFMPEG_INSTALL_COMMANDS } from "./lib/desktop-engine";

export function FfmpegInstall() {
  const [copied, setCopied] = useState<string | null>(null);
  const [failure, setFailure] = useState(false);

  async function copyCommand(label: string, command: string) {
    try {
      await navigator.clipboard.writeText(command);
      setCopied(label);
      setFailure(false);
    } catch {
      setCopied(null);
      setFailure(true);
    }
  }

  return (
    <div className="ffmpeg-install">
      <p>
        The desktop app already includes FFmpeg. Install it only for the
        command-line tool.
      </p>
      <div className="ffmpeg-cmds">
        {FFMPEG_INSTALL_COMMANDS.map((item) => (
          <div key={item.label}>
            <code>{item.command}</code>
            <button
              key={item.label}
              className="secondary"
              type="button"
              onClick={() => void copyCommand(item.label, item.command)}
            >
              Copy {item.label} command
              {copied === item.label ? (
                <span className="ffmpeg-copied">Copied</span>
              ) : null}
            </button>
          </div>
        ))}
      </div>
      <p role="status">
        {failure
          ? "Copy unavailable. Select the command above and copy it manually."
          : copied
            ? `${copied} command copied.`
            : ""}
      </p>
    </div>
  );
}
