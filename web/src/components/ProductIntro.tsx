import { DISCORD_CAPS, type DiscordCap } from "../lib/destinations";
import { desktopGuideHref } from "../lib/docs-url";

export type SampleScenario = "inspect" | "unchanged" | "format" | "crop";

const RELEASES_URL = "https://github.com/mattbaconz/fitifact/releases";

export function ProductIntro() {
  return (
    <div className="intro-copy">
      <h1>
        Make your file
        <br className="desktop-break" /> work there.
      </h1>
      <p className="intro-description">
        Say where it’s going. Fitifact makes the smallest change that fits, then
        checks it. Already fits? Nothing changes.
      </p>
      <p className="local-note">
        <span aria-hidden="true">↳</span> Your file stays on this device.
      </p>
    </div>
  );
}

export function DiscordIntro({
  cap,
  onCap,
}: {
  cap: DiscordCap;
  onCap: (cap: DiscordCap) => void;
}) {
  return (
    <div className="intro-copy">
      <h1>Fit a video for Discord</h1>
      <p className="intro-description">
        Drop a clip. Fitifact fits it under your upload limit in this browser,
        then checks the result. A clip that already fits stays as it is.
      </p>
      <fieldset className="tier-picker">
        <legend>Your Discord account</legend>
        {(Object.keys(DISCORD_CAPS) as DiscordCap[]).map((option) => (
          <label key={option}>
            <input
              type="radio"
              name="discord-tier"
              checked={cap === option}
              onChange={() => onCap(option)}
            />
            {DISCORD_CAPS[option].label} · {DISCORD_CAPS[option].short}
          </label>
        ))}
      </fieldset>
      <p className="local-note">
        <span aria-hidden="true">↳</span> Your file stays on this device.
      </p>
    </div>
  );
}

export function ProductExamples({
  onSample,
  disabled,
}: {
  onSample: (scenario: SampleScenario) => void;
  disabled: boolean;
}) {
  return (
    <section
      className="how-it-works"
      id="how-it-works"
      aria-labelledby="how-title"
    >
      <h2 id="how-title" className="section-title">
        What changes, and what doesn’t.
      </h2>
      <div className="row-list">
        <button
          disabled={disabled}
          className="row"
          onClick={() => onSample("unchanged")}
        >
          <span className="row-tag">PNG = PNG</span>
          <strong className="row-title">Leave a good file alone</strong>
          <span className="row-action">
            Try an unchanged file <span aria-hidden="true">→</span>
          </span>
        </button>
        <button
          disabled={disabled}
          className="row"
          onClick={() => onSample("format")}
        >
          <span className="row-tag">PNG → JPEG</span>
          <strong className="row-title">Change only what’s needed</strong>
          <span className="row-action">
            Try a format change <span aria-hidden="true">→</span>
          </span>
        </button>
        <button
          disabled={disabled}
          className="row"
          onClick={() => onSample("crop")}
        >
          <span className="row-tag">4:3 → 1:1</span>
          <strong className="row-title">You choose the crop</strong>
          <span className="row-action">
            Try a crop <span aria-hidden="true">→</span>
          </span>
        </button>
      </div>
    </section>
  );
}

export function ProductProof() {
  return (
    <section className="how-it-works" id="proof" aria-labelledby="proof-title">
      <h2 id="proof-title" className="section-title">
        Check it yourself.
      </h2>
      <ul className="claims">
        <li>
          <strong>Works offline once loaded</strong>
          <span>Wait a few seconds, go offline, drop a file.</span>
        </li>
        <li>
          <strong>Nothing is uploaded</strong>
          <span>Open developer tools, watch Network, adapt a file.</span>
        </li>
        <li>
          <strong>Every output is re-checked</strong>
          <span>Each result lists every rule, before and after.</span>
        </li>
      </ul>
      <p className="claims-note">
        Data Saver skips the offline warm-up, and HEIC photos load their
        decoder on first use.
      </p>
    </section>
  );
}

export function DesktopDownload() {
  return (
    <section
      className="how-it-works beyond"
      id="download"
      aria-labelledby="download-title"
    >
      <h2 id="download-title" className="section-title">
        Audio and HDR video? Use the desktop app.
      </h2>
      <p>
        Windows, with FFmpeg built in. Release candidates aren’t code-signed
        yet, so Windows may warn you first.
      </p>
      <p className="row-links">
        <a href={RELEASES_URL}>Downloads on GitHub</a>
        <a href={desktopGuideHref()}>Desktop guide</a>
        <a href={`${import.meta.env.BASE_URL}docs/cli/`}>Command line</a>
      </p>
    </section>
  );
}
