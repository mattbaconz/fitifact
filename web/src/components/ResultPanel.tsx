import {
  CheckFailIcon,
  CheckPassIcon,
  CheckUnknownIcon,
  DownloadIcon,
} from "../icons";
import { cropAxis } from "../lib/crop";
import { fileNameFromPath } from "../lib/desktop";
import { checkLabel, formatCheckValue } from "../lib/explain";
import type { FitifactSession } from "../useFitifactSession";
export function ResultPanel({
  session,
  changeRequirements,
}: {
  session: FitifactSession;
  changeRequirements: () => void;
}) {
  const {
    status,
    state,
    resultReady,
    statusTitle,
    outputFacts,
    verifiedLine,
    downloadUrl,
    download,
    mediaArtifact,
    outputBuffer,
    mediaAdapt,
    revealOutput,
    progress,
    mediaPath,
    nativeJobRef,
    cancel,
    error,
    retry,
    doctorUnhealthy,
    refreshDoctor,
    plan,
    mediaOutcome,
    problems,
    actions,
    kept,
    warnings,
    sourceUrl,
    crop,
    sourceFile,
    cropPosition,
    setCropPosition,
    cropConsent,
    setCropConsent,
    firstFrameConsent,
    setFirstFrameConsent,
    showWork,
    targetDirty,
    adaptImage,
    needsApproval,
    checklist,
    reportUrl,
    constraintsUrl,
    leftover,
  } = session;
  return (
    <section
      className={`card status-card ${status.tone}`}
      aria-labelledby="status-title"
      aria-busy={state === "processing"}
    >
      <div className="result-heading">
        <p className="eyebrow">
          {resultReady ? "The result" : "Your compatibility plan"}
        </p>
        <h2 id="status-title" className="status-title" tabIndex={-1} aria-live="polite">
          {statusTitle}
        </h2>
      </div>
      {resultReady && leftover ? (
        <div className="notice warning" role="note">
          <h3>Some text wasn’t used</h3>
          <p>{leftover}</p>
          <p>The result only verifies the understood requirements.</p>
        </div>
      ) : null}
      {resultReady ? (
        <div className="result-actions">
          <p className="output-facts numeric">{outputFacts}</p>
          <p className="intel-verified">{verifiedLine}</p>
          {downloadUrl && download && !mediaArtifact ? (
            <a
              className="download-button"
              href={downloadUrl}
              download={download.name}
            >
              <DownloadIcon />
              {state === "compatible" && !outputBuffer
                ? "Use original"
                : `Download ${download.extension.toUpperCase()}`}
            </a>
          ) : null}
          {mediaArtifact ? (
            <>
              <p className="helper">
                {mediaAdapt?.output
                  ? `Saved as ${fileNameFromPath(mediaAdapt.output)}`
                  : "The original file is ready to use."}
              </p>
              <button onClick={() => void revealOutput()}>
                Show in folder
              </button>
            </>
          ) : null}
          <button className="text-button" onClick={changeRequirements}>
            Change requirements
          </button>
        </div>
      ) : null}
      {state === "processing" ? (
        <div className="processing-state" role="status">
          <p>{progress?.stage ?? "Inspecting and planning"}</p>
          <progress aria-label="Processing file" />
          <p className="helper">Your file stays on this device.</p>
          {!mediaPath || nativeJobRef.current ? (
            <button className="secondary" onClick={() => void cancel()}>
              Cancel processing
            </button>
          ) : null}
        </div>
      ) : null}
      {error ? (
        <div className="error-state">
          {mediaAdapt?.output ? (
            <p className="helper">
              An output was preserved at{" "}
              <span className="numeric">{mediaAdapt.output}</span>. It is not
              offered as a validated result.
              <button
                className="text-button"
                onClick={() => void revealOutput()}
              >
                Show in folder
              </button>
            </p>
          ) : null}
          <p className="error-copy" role="alert">
            {error.message}
          </p>
          {state !== "processing" ? (
            <div className="action-row">
              <button className="secondary" onClick={() => void retry()}>
                Retry
              </button>
              <button className="text-button" onClick={changeRequirements}>
                Change requirements
              </button>
            </div>
          ) : null}
        </div>
      ) : null}
      {doctorUnhealthy ? (
        <div className="notice">
          <h3>Video tools need attention</h3>
          <p>
            Restore the bundled FFmpeg tools by reinstalling the desktop build.
            Photos still work.
          </p>
          <button className="secondary" onClick={() => void refreshDoctor()}>
            Check again
          </button>
        </div>
      ) : null}
      {(plan || mediaOutcome) && state !== "compatible" ? (
        <div className="plan-summary intel">
          {problems.length ? (
            <div>
              <h3>{resultReady ? "Why it changed" : "Needs to change"}</h3>
              <ul>
                {problems.map((item) => (
                  <li key={item}>{item}</li>
                ))}
              </ul>
            </div>
          ) : null}
          {actions.length ? (
            <div>
              <h3>{resultReady ? "What changed" : "The minimum change"}</h3>
              <ul>
                {actions.map((item) => (
                  <li key={item}>{item}</li>
                ))}
              </ul>
            </div>
          ) : null}
          {kept.length ? (
            <div className="preserved">
              <h3>{resultReady ? "Preserved" : "Will be preserved"}</h3>
              <ul>
                {kept.map((item) => (
                  <li key={item}>{item}</li>
                ))}
              </ul>
            </div>
          ) : null}
        </div>
      ) : !resultReady && !error && state !== "processing" ? (
        <div className="plan-empty">
          <span aria-hidden="true">→</span>
          <h3>Your file is inspected.</h3>
          <p>
            Choose a destination or enter its requirements. You’ll see exactly
            what needs to change before adapting.
          </p>
        </div>
      ) : null}
      {warnings.length ? (
        <div className="notice warning">
          <h3>{resultReady ? "Output notes" : "Warnings"}</h3>
          <ul>
            {warnings.map((warning) => (
              <li key={warning}>{warning}</li>
            ))}
          </ul>
        </div>
      ) : null}
      {plan?.plan.target.crop.required && state !== "adapted" ? (
        <div className="crop-editor" aria-labelledby="crop-title">
          <h3 id="crop-title">Choose the crop</h3>
          {sourceUrl && crop ? (
            <div className="crop-stage">
              <img
                src={sourceUrl}
                alt={`Crop preview of ${sourceFile?.name ?? "selected image"}`}
              />
              <span
                className="crop-mask"
                aria-hidden="true"
                style={{
                  left: `${crop.x * 100}%`,
                  top: `${crop.y * 100}%`,
                  width: `${crop.width * 100}%`,
                  height: `${crop.height * 100}%`,
                }}
              />
            </div>
          ) : null}
          <label htmlFor="crop-position">
            {cropAxis(
              plan.plan.source_width,
              plan.plan.source_height,
              plan.plan.target.width,
              plan.plan.target.height,
            ) === "horizontal"
              ? "Horizontal"
              : "Vertical"}{" "}
            crop position: <span className="numeric">{cropPosition}%</span>
          </label>
          <input
            id="crop-position"
            type="range"
            min="0"
            max="100"
            value={cropPosition}
            disabled={state === "processing"}
            onChange={(event) => setCropPosition(Number(event.target.value))}
          />
          <label className="check-label" htmlFor="crop-consent">
            <input
              id="crop-consent"
              type="checkbox"
              checked={cropConsent}
              disabled={state === "processing"}
              onChange={(event) => setCropConsent(event.target.checked)}
            />{" "}
            I approve removing the shaded edges to match{" "}
            {plan.plan.target.width} × {plan.plan.target.height}.
          </label>
        </div>
      ) : null}

      {plan?.plan.target.first_frame?.required && state !== "adapted" ? (
        <label className="check-label" htmlFor="first-frame-consent">
          <input
            id="first-frame-consent"
            type="checkbox"
            checked={firstFrameConsent}
            disabled={state === "processing"}
            onChange={(event) => setFirstFrameConsent(event.target.checked)}
          />
          I approve keeping only the first frame or page. Extra frames will be
          discarded.
        </label>
      ) : null}

      {showWork &&
      !targetDirty &&
      state !== "adapted" &&
      state !== "compatible" &&
      state !== "processing" ? (
        <button
          type="button"
          onClick={() => void adaptImage()}
          disabled={
            (state !== "planned" && state !== "crop_approval_required") ||
            needsApproval
          }
        >
          Adapt file
        </button>
      ) : null}

      {checklist.length ? (
        <details className="technical-details" open={!resultReady}>
          <summary>
            {resultReady
              ? "Validation details"
              : "Current compatibility checks"}
            <span>
              {checklist.filter((check) => check.result === "pass").length} /{" "}
              {checklist.length} passed
            </span>
          </summary>
          {checklist.length ? (
            <div className="checklist">
              <h3>Checked against</h3>
              <ul>
                {checklist.map((check) => (
                  <li key={check.constraint_id} className={check.result}>
                    <span className="check-mark" aria-hidden="true">
                      {check.result === "pass" ? (
                        <CheckPassIcon />
                      ) : check.result === "fail" ? (
                        <CheckFailIcon />
                      ) : (
                        <CheckUnknownIcon />
                      )}
                    </span>
                    <span>
                      <strong>{checkLabel(check.field)}</strong>
                      <br />
                      {formatCheckValue(check.field, check.actual)} / needs{" "}
                      {formatCheckValue(check.field, check.required)}
                    </span>
                    <span className="sr-result">{check.result}</span>
                  </li>
                ))}
              </ul>
            </div>
          ) : null}
        </details>
      ) : null}
      {resultReady ? (
        <>
          <p className="validation-boundary">
            Verified against these requirements. That app may still have extra
            undocumented rules.
          </p>
          <div className="report-links">
            {reportUrl ? (
              <a href={reportUrl} download="fitifact-report.json">
                Download report JSON
              </a>
            ) : null}
            {constraintsUrl ? (
              <a href={constraintsUrl} download="fitifact-constraints.json">
                Download constraints JSON
              </a>
            ) : null}
          </div>
        </>
      ) : null}
      {state === "adapted" && (sourceUrl || downloadUrl) && !mediaArtifact ? (
        <div className="before-after">
          {sourceUrl ? (
            <figure>
              <img
                src={sourceUrl}
                alt={`Before: ${sourceFile?.name ?? "original"}`}
              />
              <figcaption>Before</figcaption>
            </figure>
          ) : null}
          {downloadUrl ? (
            <figure>
              <img
                src={downloadUrl}
                alt={`After: ${download?.name ?? "adapted image"}`}
              />
              <figcaption>After</figcaption>
            </figure>
          ) : null}
        </div>
      ) : null}

      {resultReady ? (
        <button
          className="text-button recovery-link"
          onClick={changeRequirements}
        >
          Destination still said no? Paste the new message.
        </button>
      ) : null}
    </section>
  );
}
