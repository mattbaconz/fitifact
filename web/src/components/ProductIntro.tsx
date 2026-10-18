export type SampleScenario = "inspect" | "unchanged" | "format" | "crop";

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
