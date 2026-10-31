import { desktopGuideHref } from "../lib/docs-url";

export type SampleScenario = "inspect" | "unchanged" | "format" | "crop";

const RELEASES_URL = "https://github.com/mattbaconz/fitifact/releases";

export function ProductIntro() {
  return (
    <div className="intro-copy">
      <p className="eyebrow">Files, fitted with care.</p>
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
      <div className="section-heading">
        <div>
          <p className="eyebrow">Less change. More certainty.</p>
          <h2 id="how-title">A requirement, not a recipe.</h2>
        </div>
        <p>
          The same destination can need a different answer for every file. Try
          it with a local sample.
        </p>
      </div>
      <div className="example-grid">
        <button
          disabled={disabled}
          className="example"
          onClick={() => onSample("unchanged")}
        >
          <span className="example-diagram">
            <span>PNG</span>
            <span className="example-connection">=</span>
            <span>PNG</span>
          </span>
          <strong>Leave a good file alone</strong>
          <span>
            Already inside the limits. Original bytes, original quality.
          </span>
          <span className="text-link">
            Try an unchanged file <span aria-hidden="true">↗</span>
          </span>
        </button>
        <button
          disabled={disabled}
          className="example"
          onClick={() => onSample("format")}
        >
          <span className="example-diagram">
            <span>PNG</span>
            <span className="example-connection">→</span>
            <span>JPEG</span>
          </span>
          <strong>Change only what’s needed</strong>
          <span>A different format, with the same dimensions and shape.</span>
          <span className="text-link">
            Try a format change <span aria-hidden="true">↗</span>
          </span>
        </button>
        <button
          disabled={disabled}
          className="example"
          onClick={() => onSample("crop")}
        >
          <span className="example-diagram">
            <span>4:3</span>
            <span className="example-connection">→</span>
            <span>1:1</span>
          </span>
          <strong>You choose what gets cropped</strong>
          <span>
            Review the framing and approve it before anything changes.
          </span>
          <span className="text-link">
            Try a crop <span aria-hidden="true">↗</span>
          </span>
        </button>
      </div>
      <div className="developer-note">
        <div>
          <h3>The same thinking, from your terminal.</h3>
          <p>
            Inspect, preview a plan, and get a machine-readable report with the
            local CLI.
          </p>
        </div>
        <a href={`${import.meta.env.BASE_URL}docs/cli/`}>
          Explore the CLI <span aria-hidden="true">↗</span>
        </a>
      </div>
    </section>
  );
}

export function ProductProof() {
  return (
    <section className="how-it-works" id="proof" aria-labelledby="proof-title">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Check it yourself</p>
          <h2 id="proof-title">Claims you can test in this tab.</h2>
        </div>
        <p>
          Each one is something this page does, not a promise in a policy.
        </p>
      </div>
      <div className="example-grid">
        <article className="proof">
          <span className="example-diagram">
            <span>Loaded</span>
            <span className="example-connection">→</span>
            <span>Offline</span>
          </span>
          <h3>Works offline once loaded</h3>
          <p>
            After the page loads, Fitifact fetches its engine and video tools
            in the background, so the network can drop and you keep working.
            Data Saver skips this, and HEIC photos load their decoder on first
            use.
          </p>
          <p className="proof-check">
            <span className="muted">Try it:</span>{" "}
            Wait a few seconds, go offline, drop a file.
          </p>
        </article>
        <article className="proof">
          <span className="example-diagram">
            <span>connect-src 'self'</span>
          </span>
          <h3>Nothing is uploaded</h3>
          <p>
            Your file is read and changed inside this tab. There is no server
            to send it to, and the page’s security policy blocks connections
            to any other site.
          </p>
          <p className="proof-check">
            <span className="muted">Try it:</span>{" "}
            Open developer tools, watch Network, adapt a file.
          </p>
        </article>
        <article className="proof">
          <span className="example-diagram">
            <span>Output</span>
            <span className="example-connection">vs</span>
            <span>Rules</span>
          </span>
          <h3>Every output is re-checked</h3>
          <p>
            Fitifact inspects the new file and measures it against the same
            destination rules it planned with. If a rule fails, you get an
            error, not a download.
          </p>
          <p className="proof-check">
            <span className="muted">Try it:</span>{" "}
            Every result lists each rule, before and after.
          </p>
        </article>
      </div>
    </section>
  );
}

export function DesktopDownload() {
  return (
    <section
      className="how-it-works download"
      id="download"
      aria-labelledby="download-title"
    >
      <div className="ft-card download-card">
        <div>
          <p className="eyebrow">Desktop app</p>
          <h2 id="download-title">Audio and HDR video, on your computer.</h2>
          <p>
            The same engine with FFmpeg built in, for Windows. Release
            candidates are not code-signed yet, so Windows may warn you before
            the installer runs.
          </p>
        </div>
        <div className="download-actions">
          <a className="ft-btn" href={RELEASES_URL}>
            Downloads on GitHub
          </a>
          <a className="ft-btn ft-btn-secondary" href={desktopGuideHref()}>
            Desktop guide
          </a>
        </div>
      </div>
    </section>
  );
}
