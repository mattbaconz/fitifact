import { useMemo, useState } from "react";
import {
  AlertIcon,
  CheckFailIcon,
  CheckPassIcon,
  CheckUnknownIcon,
  CopyIcon,
  DownloadIcon,
  PlanIcon,
  RefuseIcon,
  ShareIcon,
  WarningIcon,
} from "../icons";
import { cropAxis } from "../lib/crop";
import { fileNameFromPath } from "../lib/desktop";
import type { MeasuredRow } from "../lib/explain";
import { copyFile, fileActions, shareFile } from "../lib/share";
import type { FitifactSession, Tone } from "../useFitifactSession";

const TONE_ICON: Record<Tone, typeof PlanIcon> = {
  neutral: PlanIcon,
  info: PlanIcon,
  success: CheckPassIcon,
  warning: WarningIcon,
  danger: AlertIcon,
  refusal: RefuseIcon,
};

export function StatusIcon({ tone }: { tone: Tone }) {
  const Icon = TONE_ICON[tone];
  return (
    <span className="status-icon" aria-hidden="true">
      <Icon />
    </span>
  );
}

const RESULT_TEXT = { pass: "Pass", fail: "Fail", unknown: "Not measured" };

function ResultMark({ result }: { result: MeasuredRow["result"] }) {
  const Icon =
    result === "pass"
      ? CheckPassIcon
      : result === "fail"
        ? CheckFailIcon
        : CheckUnknownIcon;
  return (
    <span className={`check-mark ${result}`}>
      <Icon />
      <span className="visually-hidden">{RESULT_TEXT[result]}</span>
    </span>
  );
}

/** Measured values against each requirement; after adapting, before and after side by side. */
function MeasureTable({
  rows,
  compared,
  caption,
}: {
  rows: MeasuredRow[];
  compared: boolean;
  caption: string;
}) {
  const passed = rows.filter((row) => row.result === "pass").length;
  return (
    <table className="checklist">
      <caption>
        {caption}
        <span className="ft-numeric">
          {passed} / {rows.length} passed
        </span>
      </caption>
      <thead>
        <tr>
          <th scope="col">Requirement</th>
          <th scope="col">Needs</th>
          <th scope="col">{compared ? "Before" : "Your file"}</th>
          {compared ? <th scope="col">After</th> : null}
        </tr>
      </thead>
      <tbody>
        {rows.map((row) => (
          <tr key={row.id} className={row.result}>
            <th scope="row">{row.label}</th>
            <td className="ft-numeric">{row.needs}</td>
            <td className="ft-numeric">
              <ResultMark result={row.beforeResult} />
              {row.before}
            </td>
            {compared ? (
              <td className="ft-numeric">
                <ResultMark result={row.result} />
                {row.after}
              </td>
            ) : null}
          </tr>
        ))}
      </tbody>
    </table>
  );
}

/** Copy and share hand over the exact validated file, only where the platform can. */
function FileActions({ blob, name }: { blob: Blob | null; name: string }) {
  const file = useMemo(
    () => (blob ? new File([blob], name, { type: blob.type }) : null),
    [blob, name],
  );
  const available = useMemo(() => fileActions(file), [file]);
  const [feedback, setFeedback] = useState("");
  if (!file || (!available.copy && !available.share)) return null;
  return (
    <div className="file-actions">
      {available.copy ? (
        <button
          type="button"
          className="ft-btn ft-btn-secondary"
          onClick={async () => {
            try {
              await copyFile(file);
              setFeedback("Copied. Paste it where it needs to go.");
            } catch {
              setFeedback("Copy didn’t work here. Download the file instead.");
            }
          }}
        >
          <CopyIcon />
          Copy image
        </button>
      ) : null}
      {available.share ? (
        <button
          type="button"
          className="ft-btn ft-btn-secondary"
          onClick={async () => {
            try {
              await shareFile(file);
              setFeedback("");
            } catch (caught) {
              if (!(caught instanceof DOMException && caught.name === "AbortError"))
                setFeedback("Sharing didn’t work here. Download the file instead.");
            }
          }}
        >
          <ShareIcon />
          Share
        </button>
      ) : null}
      <p className="helper file-actions-status" role="status">
        {feedback}
      </p>
    </div>
  );
}

export function ResultPanel({
  session,
  changeRequirements,
}: {
  session: FitifactSession;
  changeRequirements: () => void;
}) {
  const {
    tone,
    state,
    resultReady,
    statusTitle,
    outputFacts,
    inspectFacts,
    verifiedLine,
    downloadUrl,
    downloadBlob,
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
    adaptLabel,
    needsApproval,
    measured,
    reportUrl,
    constraintsUrl,
    leftover,
    staleRules,
  } = session;
  const failing = measured.filter((row) => row.result === "fail").length;
  const consent = state === "crop_approval_required";
  const canAdapt = state === "planned" || consent;
  const lead =
    state === "processing"
      ? null
      : resultReady
        ? verifiedLine
        : consent
          ? "Nothing changes until you approve this."
          : state === "planned"
            ? failing
              ? `${failing === 1 ? "One requirement fails" : `${failing} requirements fail`}. Fitifact changes only what it must.`
              : "Review the change below."
            : state === "cannot_satisfy"
              ? "Fitifact won’t produce a file that breaks a requirement."
              : null;
  const showAdapt =
    showWork &&
    !targetDirty &&
    state !== "adapted" &&
    state !== "compatible" &&
    state !== "processing" &&
    state !== "cannot_satisfy" &&
    !error;
  const compared = state === "adapted" && measured.some((row) => row.after !== null);
  // Retrying cannot change a refusal or a resource limit; new requirements can.
  const retryable = state !== "cannot_satisfy" && state !== "resource_limit";
  return (
    <section
      className={`ft-card status-card tone-${tone}`}
      aria-labelledby="status-title"
      aria-busy={state === "processing"}
    >
      <div className="status-head">
        <StatusIcon tone={tone} />
        <div className="status-heading">
          <p className="ft-eyebrow">
            {resultReady ? "The result" : "Your compatibility plan"}
          </p>
          <h2
            id="status-title"
            className="status-title"
            tabIndex={-1}
            aria-live="polite"
          >
            {statusTitle}
          </h2>
          {lead ? <p className="status-lead intel-verified">{lead}</p> : null}
        </div>
      </div>

      {resultReady && (leftover || staleRules) ? (
        <div className="ft-notice ft-tone-warning result-caveat" role="note">
          <h3>Check before you rely on it</h3>
          {leftover ? (
            <p>
              {leftover} The result only verifies the requirements Fitifact
              understood.
            </p>
          ) : null}
          {staleRules ? (
            <p>
              These destination rules haven’t been re-checked in over 30 days.
              Confirm the limits for your account.
            </p>
          ) : null}
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
            crop position: <span className="ft-numeric">{cropPosition}%</span>
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
            />
            <span>
              I approve removing the shaded edges to match{" "}
              {plan.plan.target.width} × {plan.plan.target.height}.
            </span>
          </label>
        </div>
      ) : null}

      {plan?.plan.target.first_frame?.required && state !== "adapted" ? (
        <label className="check-label consent-frame" htmlFor="first-frame-consent">
          <input
            id="first-frame-consent"
            type="checkbox"
            checked={firstFrameConsent}
            disabled={state === "processing"}
            onChange={(event) => setFirstFrameConsent(event.target.checked)}
          />
          <span>
            I approve keeping only the first frame or page. Extra frames will be
            discarded.
          </span>
        </label>
      ) : null}

      {showAdapt ? (
        <div className="status-actions">
          <button
            type="button"
            className="ft-btn ft-btn-block adapt-button"
            onClick={() => void adaptImage(!mediaPath)}
            disabled={!canAdapt || needsApproval}
            aria-describedby={!canAdapt || needsApproval ? "adapt-help" : undefined}
          >
            {adaptLabel}
          </button>
          {!canAdapt || needsApproval ? (
            <p id="adapt-help" className="helper">
              {needsApproval
                ? "Approve the change above to continue."
                : "Choose where the file is going first."}
            </p>
          ) : null}
        </div>
      ) : null}

      {resultReady ? (
        <div className="status-actions result-actions">
          {downloadUrl && download && !mediaArtifact ? (
            <>
              {/* Safari leaves links out of the Tab order unless they set tabindex. */}
              <a
                className="ft-btn ft-btn-block download-button"
                href={downloadUrl}
                download={download.name}
                tabIndex={0}
              >
                <DownloadIcon />
                {state === "compatible" && !outputBuffer
                  ? "Use original"
                  : `Download ${download.extension.toUpperCase()}`}
              </a>
              <FileActions blob={downloadBlob} name={download.name} />
            </>
          ) : null}
          {mediaArtifact ? (
            <>
              <p className="helper">
                {mediaAdapt?.output
                  ? `Saved as ${fileNameFromPath(mediaAdapt.output)}`
                  : "The original file is ready to use."}
              </p>
              <button
                type="button"
                className="ft-btn ft-btn-secondary ft-btn-block"
                onClick={() => void revealOutput()}
              >
                Show in folder
              </button>
            </>
          ) : null}
          <button
            type="button"
            className="ft-btn ft-btn-text"
            onClick={changeRequirements}
          >
            Change requirements
          </button>
        </div>
      ) : null}

      {state === "processing" ? (
        <div className="processing-state" role="status">
          <p>{progress?.stage ?? "Inspecting and planning"}</p>
          <progress
            aria-label="Processing file"
            max={100}
            value={progress?.percent ? progress.percent : undefined}
          />
          <p className="helper">Your file stays on this device.</p>
          {!mediaPath || nativeJobRef.current ? (
            <button
              type="button"
              className="ft-btn ft-btn-secondary"
              onClick={() => void cancel()}
            >
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
              <span className="ft-numeric">{mediaAdapt.output}</span>. It is not
              offered as a validated result.{" "}
              <button
                type="button"
                className="ft-btn ft-btn-text inline-button"
                onClick={() => void revealOutput()}
              >
                Show in folder
              </button>
            </p>
          ) : null}
          <p className="ft-notice ft-tone-danger error-copy" role="alert">
            {error.message}
          </p>
          {state !== "cancelled" ? (
            <details className="error-details">
              <summary>Technical details</summary>
              <code>{error.code}</code>
            </details>
          ) : null}
          {state !== "processing" ? (
            <div className="action-row">
              {retryable ? (
                <button
                  type="button"
                  className="ft-btn ft-btn-secondary"
                  onClick={() => void retry()}
                >
                  Retry
                </button>
              ) : null}
              <button
                type="button"
                className={retryable ? "ft-btn ft-btn-text" : "ft-btn ft-btn-secondary"}
                onClick={changeRequirements}
              >
                Change requirements
              </button>
            </div>
          ) : null}
        </div>
      ) : state === "cannot_satisfy" ? (
        <div className="action-row">
          <button
            type="button"
            className="ft-btn ft-btn-secondary"
            onClick={changeRequirements}
          >
            Change requirements
          </button>
        </div>
      ) : null}

      {doctorUnhealthy ? (
        <div className="ft-notice ft-tone-warning">
          <h3>Video tools need attention</h3>
          <p>
            Restore the bundled FFmpeg tools by reinstalling the desktop build.
            Photos still work.
          </p>
          <button
            type="button"
            className="ft-btn ft-btn-secondary"
            onClick={() => void refreshDoctor()}
          >
            Check again
          </button>
        </div>
      ) : null}

      {measured.length ? (
        <MeasureTable
          rows={measured}
          compared={compared}
          caption={resultReady ? "Checked on the result" : "Checked against the requirements"}
        />
      ) : null}

      {compared && (sourceUrl || downloadUrl) && !mediaArtifact ? (
        <div className="before-after">
          {sourceUrl ? (
            <figure>
              <img
                src={sourceUrl}
                alt={`Before: ${sourceFile?.name ?? "original"}`}
              />
              <figcaption>
                <strong>Before</strong>
                <span className="ft-numeric">{inspectFacts}</span>
              </figcaption>
            </figure>
          ) : null}
          {downloadUrl ? (
            <figure>
              <img
                src={downloadUrl}
                alt={`After: ${download?.name ?? "adapted image"}`}
              />
              <figcaption>
                <strong>After</strong>
                <span className="ft-numeric output-facts">{outputFacts}</span>
              </figcaption>
            </figure>
          ) : null}
        </div>
      ) : null}

      {(plan || mediaOutcome) && state !== "compatible" ? (
        <div className="plan-summary">
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
          {actions.length && state !== "cannot_satisfy" ? (
            <div>
              <h3>{resultReady ? "What changed" : "The minimum change"}</h3>
              <ul>
                {actions.map((item) => (
                  <li key={item}>{item}</li>
                ))}
              </ul>
            </div>
          ) : null}
          {kept.length && state !== "cannot_satisfy" ? (
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
      ) : !resultReady && !error && state !== "processing" && state !== "cannot_satisfy" ? (
        <div className="plan-empty">
          <h3>Your file is inspected.</h3>
          <p>
            Choose a destination or enter its requirements. You’ll see exactly
            what needs to change before adapting.
          </p>
        </div>
      ) : null}

      {warnings.length ? (
        <div className="ft-notice output-notes">
          <h3>{resultReady ? "Output notes" : "Warnings"}</h3>
          <ul>
            {warnings.map((warning) => (
              <li key={warning}>{warning}</li>
            ))}
          </ul>
        </div>
      ) : null}

      {resultReady ? (
        <div className="result-footer">
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
          <button
            type="button"
            className="ft-btn ft-btn-text recovery-link"
            onClick={changeRequirements}
          >
            Destination still said no? Paste the new message.
          </button>
        </div>
      ) : null}
    </section>
  );
}
