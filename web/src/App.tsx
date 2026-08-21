import { useEffect, useMemo, useRef, useState, type DragEvent } from "react";
import { constraintSetFromEditable, editableTargetFromConstraints, formatBytes } from "./lib/constraints";
import { cropAxis, cropForAspect } from "./lib/crop";
import type {
  AdaptReport,
  EditableTarget,
  ErrorReport,
  PlanReport,
  ProductState,
  RequirementParse,
} from "./types";
import { ImageWorkerClient, WorkerFailure, type ProgressUpdate } from "./worker/client";

const DEFAULT_REQUIREMENTS = "JPEG, max 2 MB, max 2000 x 2000";

const STATE_COPY: Record<ProductState, { title: string; tone: string }> = {
  idle: { title: "Describe the upload requirement", tone: "neutral" },
  requirements_ready: { title: "Requirements ready", tone: "neutral" },
  processing: { title: "Working locally", tone: "neutral" },
  planned: { title: "Minimum changes ready", tone: "neutral" },
  compatible: { title: "Already compatible", tone: "success" },
  crop_approval_required: { title: "Crop approval required", tone: "warning" },
  adapted: { title: "Image adapted and validated", tone: "success" },
  cannot_satisfy: { title: "Cannot satisfy these requirements", tone: "danger" },
  validation_failure: { title: "Validation failed", tone: "danger" },
  resource_limit: { title: "Resource limit reached", tone: "danger" },
  cancelled: { title: "Processing cancelled", tone: "warning" },
  unsupported_heic: { title: "HEIC is unsupported in this build", tone: "warning" },
  error: { title: "Could not process this image", tone: "danger" },
};

function useObjectUrl(blob: Blob | null) {
  const [url, setUrl] = useState<string | null>(null);
  useEffect(() => {
    if (!blob) {
      setUrl(null);
      return;
    }
    const next = URL.createObjectURL(blob);
    setUrl(next);
    return () => URL.revokeObjectURL(next);
  }, [blob]);
  return url;
}

function outputDetails(format: "jpeg" | "png", originalName: string) {
  const extension = format === "jpeg" ? "jpg" : "png";
  const mime = format === "jpeg" ? "image/jpeg" : "image/png";
  const stem = originalName.replace(/\.[^.]*$/, "").replace(/[^a-zA-Z0-9._-]+/g, "-") || "image";
  return { extension, mime, name: `${stem}.fitifact.${extension}` };
}

export function App() {
  const clientRef = useRef<ImageWorkerClient | null>(null);
  if (!clientRef.current) clientRef.current = new ImageWorkerClient();
  const client = clientRef.current;
  const [requirements, setRequirements] = useState(DEFAULT_REQUIREMENTS);
  const [parsed, setParsed] = useState<RequirementParse | null>(null);
  const [target, setTarget] = useState<EditableTarget | null>(null);
  const [state, setState] = useState<ProductState>("idle");
  const [progress, setProgress] = useState<ProgressUpdate | null>(null);
  const [error, setError] = useState<ErrorReport | null>(null);
  const [sourceFile, setSourceFile] = useState<File | null>(null);
  const [plan, setPlan] = useState<PlanReport | null>(null);
  const [adapted, setAdapted] = useState<AdaptReport | null>(null);
  const [outputBuffer, setOutputBuffer] = useState<ArrayBuffer | null>(null);
  const [cropPosition, setCropPosition] = useState(50);
  const [cropConsent, setCropConsent] = useState(false);
  const [dragging, setDragging] = useState(false);

  useEffect(() => () => client.dispose(), [client]);

  const inspectedSafeImage = Boolean(
    sourceFile &&
      plan?.inspection.image?.format &&
      ["image/jpeg", "image/png"].includes(sourceFile.type),
  );
  const sourceUrl = useObjectUrl(inspectedSafeImage ? sourceFile : null);
  const outputFormat = adapted?.output_artifact.image?.format ?? plan?.plan.target.format ?? null;
  const download = outputFormat && sourceFile ? outputDetails(outputFormat, sourceFile.name) : null;
  const outputBlob = useMemo(
    () => (outputBuffer && download ? new Blob([outputBuffer], { type: download.mime }) : null),
    [outputBuffer, download],
  );
  const originalBlob = state === "compatible" && sourceFile && download
    ? new Blob([sourceFile], { type: download.mime })
    : null;
  const downloadUrl = useObjectUrl(outputBlob ?? originalBlob);

  const crop = useMemo(() => {
    if (!plan?.plan.target.crop.required) return null;
    return cropForAspect(
      plan.plan.source_width,
      plan.plan.source_height,
      plan.plan.target.width,
      plan.plan.target.height,
      cropPosition,
    );
  }, [plan, cropPosition]);

  function onProgress(next: ProgressUpdate) {
    setProgress(next);
  }

  function handleFailure(caught: unknown) {
    const failure = caught instanceof WorkerFailure ? caught : null;
    setState(failure?.state ?? "error");
    setError(
      failure?.report ?? {
        schema: "fitifact.error/v1",
        code: "EXECUTION_FAILED",
        message: caught instanceof Error ? caught.message : "Local processing failed.",
      },
    );
    setProgress(null);
  }

  async function parseRequirements() {
    setState("processing");
    setError(null);
    setPlan(null);
    setAdapted(null);
    setOutputBuffer(null);
    try {
      const { report } = await client.compile<RequirementParse>(requirements, onProgress);
      setParsed(report);
      if (report.ambiguities.length) {
        setState("error");
        setError({
          schema: "fitifact.error/v1",
          code: "REQUIREMENTS_AMBIGUOUS",
          message: report.ambiguities.map((item) => item.message).join(" "),
        });
        setTarget(null);
      } else if (!report.constraints) {
        setState("error");
        setError({
          schema: "fitifact.error/v1",
          code: "INPUT_INVALID",
          message: "No supported image format, size, or dimension requirement was found.",
        });
        setTarget(null);
      } else {
        setTarget(editableTargetFromConstraints(report.constraints));
        setState("requirements_ready");
      }
    } catch (caught) {
      handleFailure(caught);
    } finally {
      setProgress(null);
    }
  }

  function constraintsJson(): string {
    if (!target) throw new Error("Review a valid requirement first.");
    return JSON.stringify(constraintSetFromEditable(target));
  }

  async function analyzeFile(file: File) {
    if (!target) {
      setError({
        schema: "fitifact.error/v1",
        code: "INPUT_INVALID",
        message: "Review the upload requirements before choosing an image.",
      });
      setState("error");
      return;
    }
    setSourceFile(file);
    setPlan(null);
    setAdapted(null);
    setOutputBuffer(null);
    setCropConsent(false);
    setError(null);
    setState("processing");
    try {
      const buffer = await file.arrayBuffer();
      const { report } = await client.analyze<PlanReport>(file.name, buffer, constraintsJson(), onProgress);
      setPlan(report);
      if (report.report.compatible && report.plan.noop) setState("compatible");
      else if (report.plan.target.crop.required) setState("crop_approval_required");
      else setState("planned");
    } catch (caught) {
      handleFailure(caught);
    } finally {
      setProgress(null);
    }
  }

  async function replan() {
    if (!sourceFile) return;
    setState("processing");
    setError(null);
    try {
      const { report } = await client.replan<PlanReport>(constraintsJson(), onProgress);
      setPlan(report);
      setAdapted(null);
      setOutputBuffer(null);
      setCropConsent(false);
      if (report.report.compatible && report.plan.noop) setState("compatible");
      else if (report.plan.target.crop.required) setState("crop_approval_required");
      else setState("planned");
    } catch (caught) {
      handleFailure(caught);
    } finally {
      setProgress(null);
    }
  }

  async function adaptImage() {
    if (!plan) return;
    if (plan.plan.target.crop.required && (!crop || !cropConsent)) {
      setState("crop_approval_required");
      return;
    }
    setState("processing");
    setError(null);
    try {
      const { report, output } = await client.adapt<AdaptReport>(
        constraintsJson(),
        plan.plan.target.crop.required ? crop : null,
        onProgress,
      );
      setAdapted(report);
      setOutputBuffer(output ?? null);
      setState(report.status === "compatible" ? "compatible" : "adapted");
    } catch (caught) {
      handleFailure(caught);
    } finally {
      setProgress(null);
    }
  }

  function cancel() {
    client.cancel();
    setState("cancelled");
    setError({
      schema: "fitifact.error/v1",
      code: "EXECUTION_CANCELLED",
      message: "Local processing was cancelled. No output was saved.",
    });
    setProgress(null);
  }

  function onDrop(event: DragEvent<HTMLDivElement>) {
    event.preventDefault();
    setDragging(false);
    const file = event.dataTransfer.files.item(0);
    if (file) void analyzeFile(file);
  }

  const checklist = adapted?.report.checks ?? plan?.report.checks ?? [];
  const status = STATE_COPY[state];

  return (
    <div className="app-shell">
      <header className="site-header">
        <a className="wordmark" href="#top" aria-label="Fitifact home">Fitifact</a>
        <p className="privacy-line"><span aria-hidden="true">●</span> Your image stays on this device.</p>
      </header>

      <main id="top">
        <section className="hero" aria-labelledby="page-title">
          <p className="eyebrow">Local image compatibility</p>
          <h1 id="page-title">Make your image pass the upload</h1>
          <p className="lede">Paste what the upload form asks for. Fitifact finds the smallest change, shows it to you, then validates the result.</p>
          <p className="local-badge">Your image stays on this device. Uploads to Fitifact: 0 bytes.</p>
        </section>

        <div className="workflow-grid">
          <section className="card" aria-labelledby="requirements-title">
            <div className="step-heading"><span>1</span><h2 id="requirements-title">Paste the requirements</h2></div>
            <label htmlFor="requirements">Upload instructions</label>
            <textarea
              id="requirements"
              rows={4}
              value={requirements}
              onChange={(event) => setRequirements(event.target.value)}
              disabled={state === "processing"}
            />
            <button type="button" onClick={() => void parseRequirements()} disabled={state === "processing" || !requirements.trim()}>
              Review requirements
            </button>
            {parsed?.unresolved.length ? (
              <div className="notice" role="note">
                <strong>Not used:</strong> {parsed.unresolved.map((item) => item.text.trim()).filter(Boolean).join(" · ")}
              </div>
            ) : null}
          </section>

          <section className="card" aria-labelledby="target-title">
            <div className="step-heading"><span>2</span><h2 id="target-title">Review the target</h2></div>
            {target ? (
              <div className="target-form">
                <label>Format<select value={target.format} onChange={(event) => setTarget({ ...target, format: event.target.value as "jpeg" | "png" })}><option value="jpeg">JPEG</option><option value="png">PNG</option></select></label>
                <label>Maximum bytes<input inputMode="numeric" value={target.maxBytes} onChange={(event) => setTarget({ ...target, maxBytes: event.target.value })} placeholder="No limit" /></label>
                <fieldset><legend>Width</legend><select aria-label="Width rule" value={target.widthOp} onChange={(event) => setTarget({ ...target, widthOp: event.target.value as EditableTarget["widthOp"] })}><option value="eq">Exactly</option><option value="lte">At most</option><option value="gte">At least</option></select><input aria-label="Width in pixels" inputMode="numeric" value={target.width} onChange={(event) => setTarget({ ...target, width: event.target.value })} placeholder="Any" /></fieldset>
                <fieldset><legend>Height</legend><select aria-label="Height rule" value={target.heightOp} onChange={(event) => setTarget({ ...target, heightOp: event.target.value as EditableTarget["heightOp"] })}><option value="eq">Exactly</option><option value="lte">At most</option><option value="gte">At least</option></select><input aria-label="Height in pixels" inputMode="numeric" value={target.height} onChange={(event) => setTarget({ ...target, height: event.target.value })} placeholder="Any" /></fieldset>
                {sourceFile ? <button className="secondary" type="button" onClick={() => void replan()} disabled={state === "processing"}>Review target changes</button> : null}
              </div>
            ) : <p className="empty-copy">A normalized, editable target will appear here.</p>}
          </section>

          <section className="card upload-card" aria-labelledby="image-title">
            <div className="step-heading"><span>3</span><h2 id="image-title">Choose your image</h2></div>
            <div className={`drop-zone ${dragging ? "is-dragging" : ""}`} onDragOver={(event) => { event.preventDefault(); setDragging(true); }} onDragLeave={() => setDragging(false)} onDrop={onDrop}>
              <p><strong>Drop a JPEG or PNG here</strong></p>
              <p>HEIC is accepted only in an explicitly approved build.</p>
              <label className="button-label" htmlFor="image-file">Choose an image</label>
              <input id="image-file" className="visually-hidden" type="file" accept="image/jpeg,image/png,.heic,.heif" onChange={(event) => { const file = event.currentTarget.files?.item(0); if (file) void analyzeFile(file); event.currentTarget.value = ""; }} disabled={!target || state === "processing"} />
            </div>
            {sourceFile ? <p className="file-row"><span>{sourceFile.name}</span><span>{formatBytes(sourceFile.size)}</span></p> : null}
          </section>

          <section className={`card status-card ${status.tone}`} aria-labelledby="status-title">
            <div className="step-heading"><span>4</span><h2 id="status-title">{status.title}</h2></div>
            <div role="status" aria-live="polite" aria-atomic="true">
              {state === "processing" && progress ? <><p>{progress.stage}</p><progress max="100" value={progress.percent}>{progress.percent}%</progress><p className="privacy-reminder">Your image stays on this device.</p><button className="danger-link" type="button" onClick={cancel}>Cancel processing</button></> : null}
              {error ? <p className="error-copy"><strong>{error.code}</strong><br />{error.message}</p> : null}
              {plan ? <PlanSummary plan={plan} /> : <p className="empty-copy">The inspection, minimum changes, and validation checklist will appear here.</p>}
            </div>

            {plan?.plan.target.crop.required && state !== "adapted" ? (
              <div className="crop-editor" aria-labelledby="crop-title">
                <h3 id="crop-title">Choose the crop</h3>
                {sourceUrl && crop ? <div className="crop-stage"><img src={sourceUrl} alt={`Crop preview of ${sourceFile?.name ?? "selected image"}`} /><span className="crop-mask" aria-hidden="true" style={{ left: `${crop.x * 100}%`, top: `${crop.y * 100}%`, width: `${crop.width * 100}%`, height: `${crop.height * 100}%` }} /></div> : null}
                <label htmlFor="crop-position">{cropAxis(plan.plan.source_width, plan.plan.source_height, plan.plan.target.width, plan.plan.target.height) === "horizontal" ? "Horizontal" : "Vertical"} crop position: {cropPosition}%</label>
                <input id="crop-position" type="range" min="0" max="100" value={cropPosition} onChange={(event) => setCropPosition(Number(event.target.value))} />
                <label className="check-label"><input type="checkbox" checked={cropConsent} onChange={(event) => setCropConsent(event.target.checked)} /> I approve removing the shaded edges to match {plan.plan.target.width} × {plan.plan.target.height}.</label>
              </div>
            ) : null}

            {(state === "planned" || state === "crop_approval_required") ? <button type="button" onClick={() => void adaptImage()} disabled={Boolean(plan?.plan.target.crop.required && !cropConsent)}>Adapt and validate</button> : null}

            {checklist.length ? <div className="checklist"><h3>Requirement checklist</h3><ul>{checklist.map((check) => <li key={check.constraint_id} className={check.result}><span aria-hidden="true">{check.result === "pass" ? "✓" : check.result === "fail" ? "×" : "?"}</span><span><strong>{check.field}</strong><br />{check.actual ?? "Unknown"} / needs {check.required}</span><span className="sr-result">{check.result}</span></li>)}</ul></div> : null}

            {downloadUrl && download && (state === "adapted" || state === "compatible") ? <a className="download-button" href={downloadUrl} download={download.name}>{state === "compatible" ? "Use original image" : `Download ${download.extension.toUpperCase()}`}</a> : null}
          </section>
        </div>
      </main>

      <footer><p>Your image stays on this device. Fitifact has no upload or cloud fallback.</p><p>{__FITIFACT_HEIC_APPROVED__ ? <>Optional HEIC decoder approved for this build; see the <a href="/THIRD_PARTY_NOTICES.md">third-party notices</a>.</> : "HEIC decoder disabled in this build."}</p></footer>
    </div>
  );
}

function PlanSummary({ plan }: { plan: PlanReport }) {
  const image = plan.inspection.image;
  return (
    <div className="plan-summary">
      <p><strong>Source:</strong> {image?.format?.toUpperCase()} · {image?.width} × {image?.height} · {formatBytes(plan.inspection.byte_length)}</p>
      {plan.report.compatible ? <p>No changes are needed. The original bytes remain untouched.</p> : <><p><strong>Proposed:</strong> {plan.plan.target.format.toUpperCase()} · {plan.plan.target.width} × {plan.plan.target.height}{plan.plan.target.max_bytes ? ` · at most ${formatBytes(plan.plan.target.max_bytes)}` : ""}</p><ul>{plan.plan.warnings.map((warning) => <li key={warning}>{warning}</li>)}</ul></>}
    </div>
  );
}
