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
        Tell Fitifact where it needs to work. It finds the smallest change, then
        checks the result. Already compatible? Nothing changes.
      </p>
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
          <span className="row-body">
            <strong>Leave a good file alone</strong>
            <span>Already inside the limits, so you get the original back.</span>
          </span>
          <span className="row-action">
            Try an unchanged file <span aria-hidden="true">↗</span>
          </span>
        </button>
        <button
          disabled={disabled}
          className="row"
          onClick={() => onSample("format")}
        >
          <span className="row-tag">PNG → JPEG</span>
          <span className="row-body">
            <strong>Change only what’s needed</strong>
            <span>A different format, with the same dimensions and shape.</span>
          </span>
          <span className="row-action">
            Try a format change <span aria-hidden="true">↗</span>
          </span>
        </button>
        <button
          disabled={disabled}
          className="row"
          onClick={() => onSample("crop")}
        >
          <span className="row-tag">4:3 → 1:1</span>
          <span className="row-body">
            <strong>You choose what gets cropped</strong>
            <span>Review the framing and approve it before anything changes.</span>
          </span>
          <span className="row-action">
            Try a crop <span aria-hidden="true">↗</span>
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
      <div className="row-list">
        <div className="row">
          <span className="row-tag">Loaded → Offline</span>
          <span className="row-body">
            <strong>Works offline once loaded</strong>
            <span>
              The engine and video tools load in the background, so the network
              can drop. Data Saver skips this, and HEIC photos load their
              decoder on first use.
            </span>
            <span className="row-check">
              Wait a few seconds, go offline, drop a file.
            </span>
          </span>
        </div>
        <div className="row">
          <span className="row-tag">connect-src 'self'</span>
          <span className="row-body">
            <strong>Nothing is uploaded</strong>
            <span>
              Your file is read and changed in this tab, and the page’s
              security policy blocks connections to other sites.
            </span>
            <span className="row-check">
              Open developer tools, watch Network, adapt a file.
            </span>
          </span>
        </div>
        <div className="row">
          <span className="row-tag">Output vs Rules</span>
          <span className="row-body">
            <strong>Every output is re-checked</strong>
            <span>
              The new file is measured against the same rules it was planned
              with. A failed rule is an error, not a download.
            </span>
            <span className="row-check">
              Every result lists each rule, before and after.
            </span>
          </span>
        </div>
      </div>
    </section>
  );
}

export function DesktopDownload() {
  return (
    <section
      className="how-it-works"
      id="download"
      aria-labelledby="download-title"
    >
      <h2 id="download-title" className="section-title">
        Beyond the browser.
      </h2>
      <div className="row-list">
        <div className="row">
          <span className="row-tag">Windows</span>
          <span className="row-body">
            <strong>Desktop app</strong>
            <span>
              Audio and HDR video, with FFmpeg built in. Release candidates are
              not code-signed yet, so Windows may warn you before the installer
              runs.
            </span>
          </span>
          <span className="row-links">
            <a href={RELEASES_URL}>Downloads on GitHub</a>
            <a href={desktopGuideHref()}>Desktop guide</a>
          </span>
        </div>
        <div className="row">
          <span className="row-tag">Terminal</span>
          <span className="row-body">
            <strong>Command line</strong>
            <span>
              Inspect a file, preview a plan, and get a machine-readable report.
            </span>
          </span>
          <span className="row-links">
            <a href={`${import.meta.env.BASE_URL}docs/cli/`}>CLI docs</a>
          </span>
        </div>
      </div>
    </section>
  );
}
