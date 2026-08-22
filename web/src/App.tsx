import { useEffect, useMemo, useRef, useState, type DragEvent } from "react";
import { constraintSetFromEditable, editableTargetFromConstraints, formatBytes } from "./lib/constraints";
import { cropAxis, cropForAspect } from "./lib/crop";
import type {
  AdaptReport,
  ConstraintSet,
  EditableTarget,
  ErrorReport,
  PlanReport,
  ProductState,
  RequirementParse,
} from "./types";
import { ImageWorkerClient, WorkerFailure, type ProgressUpdate } from "./worker/client";

const DEFAULT_REQUIREMENTS = "JPEG, max 2 MB, max 2000 x 2000";

const STATE_COPY: Record<ProductState, { title: string; tone: string }> = {
  idle: { title: "Waiting for reviewed requirements", tone: "neutral" },
  requirements_ready: { title: "Ready for an image", tone: "neutral" },
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
  const [entry, setEntry] = useState<{ blob: Blob; url: string } | null>(null);
  useEffect(() => {
    if (!blob) {
      setEntry(null);
      return;
    }
    const url = URL.createObjectURL(blob);
    setEntry({ blob, url });
    return () => URL.revokeObjectURL(url);
  }, [blob]);
  return entry?.blob === blob ? entry.url : null;
}

function outputDetails(format: "jpeg" | "png", originalName: string) {
  const extension = format === "jpeg" ? "jpg" : "png";
  const mime = format === "jpeg" ? "image/jpeg" : "image/png";
  const stem = originalName.replace(/\.[^.]*$/, "").replace(/[^a-zA-Z0-9._-]+/g, "-") || "image";
  return { extension, mime, name: `${stem}.fitifact.${extension}` };
}

export function App() {
  const clientRef = useRef<ImageWorkerClient | null>(null);
  const operationRef = useRef(0);
  if (!clientRef.current) clientRef.current = new ImageWorkerClient();
  const client = clientRef.current;
  const [requirements, setRequirements] = useState(DEFAULT_REQUIREMENTS);
  const [parsed, setParsed] = useState<RequirementParse | null>(null);
  const [target, setTarget] = useState<EditableTarget | null>(null);
  const [confirmedConstraintsJson, setConfirmedConstraintsJson] = useState<string | null>(null);
  const [targetDirty, setTargetDirty] = useState(false);
  const [state, setState] = useState<ProductState>("idle");
  const [progress, setProgress] = useState<ProgressUpdate | null>(null);
  const [error, setError] = useState<ErrorReport | null>(null);
  const [sourceFile, setSourceFile] = useState<File | null>(null);
  const [plan, setPlan] = useState<PlanReport | null>(null);
  const [adapted, setAdapted] = useState<AdaptReport | null>(null);
  const [outputBuffer, setOutputBuffer] = useState<ArrayBuffer | null>(null);
  const [previewBuffer, setPreviewBuffer] = useState<ArrayBuffer | null>(null);
  const [cropPosition, setCropPosition] = useState(50);
  const [cropConsent, setCropConsent] = useState(false);
  const [dragging, setDragging] = useState(false);

  useEffect(() => () => client.dispose(), [client]);

  const inspectedSafeImage = Boolean(
    sourceFile &&
      plan?.inspection.image?.format &&
      ["image/jpeg", "image/png"].includes(sourceFile.type),
  );
  const previewBlob = useMemo(
    () => (previewBuffer ? new Blob([previewBuffer], { type: "image/png" }) : null),
    [previewBuffer],
  );
  const sourceUrl = useObjectUrl(previewBlob ?? (inspectedSafeImage ? sourceFile : null));
  const outputFormat = adapted?.output_artifact.image?.format ?? plan?.plan.target.format ?? null;
  const download = useMemo(
    () => (outputFormat && sourceFile ? outputDetails(outputFormat, sourceFile.name) : null),
    [outputFormat, sourceFile],
  );
  const outputBlob = useMemo(
    () => (outputBuffer && download ? new Blob([outputBuffer], { type: download.mime }) : null),
    [outputBuffer, download],
  );
  const originalBlob = useMemo(
    () => state === "compatible" && !outputBuffer && sourceFile && download
      ? new Blob([sourceFile], { type: download.mime })
      : null,
    [state, outputBuffer, sourceFile, download],
  );
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

  function beginOperation(replaceWorker = false) {
    const operation = ++operationRef.current;
    if (replaceWorker) client.cancel();
    return operation;
  }

  function onProgress(operation: number) {
    return (next: ProgressUpdate) => {
      if (operation === operationRef.current) setProgress(next);
    };
  }

  function handleFailure(caught: unknown, operation: number) {
    if (operation !== operationRef.current) return;
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

  function clearDerivedState(clearSource: boolean) {
    setPlan(null);
    setAdapted(null);
    setOutputBuffer(null);
    setPreviewBuffer(null);
    setCropConsent(false);
    setProgress(null);
    if (clearSource) setSourceFile(null);
  }

  function editRequirements(value: string) {
    ++operationRef.current;
    client.cancel();
    setRequirements(value);
    setParsed(null);
    setTarget(null);
    setConfirmedConstraintsJson(null);
    setTargetDirty(false);
    clearDerivedState(true);
    setError(null);
    setState("idle");
  }

  function editTarget(update: (current: EditableTarget) => EditableTarget) {
    if (!target) return;
    ++operationRef.current;
    setTarget((current) => current ? update(current) : current);
    setTargetDirty(true);
    clearDerivedState(false);
    setError(null);
    setState("requirements_ready");
  }

  async function parseRequirements() {
    const operation = beginOperation(true);
    setState("processing");
    setError(null);
    setTarget(null);
    setConfirmedConstraintsJson(null);
    setTargetDirty(false);
    clearDerivedState(true);
    try {
      const { report } = await client.compile<RequirementParse>(requirements, onProgress(operation));
      if (operation !== operationRef.current) return;
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
        try {
          setTarget(editableTargetFromConstraints(report.constraints));
          setConfirmedConstraintsJson(JSON.stringify(report.constraints));
          setState("requirements_ready");
        } catch (caught) {
          setTarget(null);
          setConfirmedConstraintsJson(null);
          handleFailure(caught, operation);
        }
      }
    } catch (caught) {
      handleFailure(caught, operation);
    } finally {
      if (operation === operationRef.current) setProgress(null);
    }
  }

  function draftConstraintsJson(): string {
    if (!target) throw new Error("Review a valid requirement first.");
    return JSON.stringify(constraintSetFromEditable(target));
  }

  async function analyzeFile(file: File) {
    if (state === "processing") return;
    if (!target || !confirmedConstraintsJson || targetDirty) {
      setError({
        schema: "fitifact.error/v1",
        code: "INPUT_INVALID",
        message: "Review the upload requirements before choosing an image.",
      });
      setState("error");
      return;
    }
    const operation = beginOperation(true);
    setSourceFile(file);
    setPlan(null);
    setAdapted(null);
    setOutputBuffer(null);
    setPreviewBuffer(null);
    setCropConsent(false);
    setError(null);
    setState("processing");
    try {
      const { report, preview, constraintsSnapshot } = await client.analyze<PlanReport>(
        file,
        confirmedConstraintsJson,
        onProgress(operation),
      );
      if (operation !== operationRef.current) return;
      setPlan(report);
      setConfirmedConstraintsJson(constraintsSnapshot ?? confirmedConstraintsJson);
      setPreviewBuffer(preview ?? null);
      if (report.report.compatible && report.plan.noop) {
        setOutputBuffer(preview ?? null);
        setState("compatible");
      }
      else if (report.plan.target.crop.required) setState("crop_approval_required");
      else setState("planned");
    } catch (caught) {
      handleFailure(caught, operation);
    } finally {
      if (operation === operationRef.current) setProgress(null);
    }
  }

  async function replan() {
    if (!sourceFile || !confirmedConstraintsJson || !target || state === "processing") return;
    const operation = beginOperation();
    setState("processing");
    setError(null);
    setPlan(null);
    setAdapted(null);
    setOutputBuffer(null);
    try {
      const draft = draftConstraintsJson();
      const { report, preview, constraintsSnapshot } = await client.replan<PlanReport>(
        confirmedConstraintsJson,
        draft,
        onProgress(operation),
      );
      if (operation !== operationRef.current) return;
      setPlan(report);
      setConfirmedConstraintsJson(constraintsSnapshot ?? draft);
      setTargetDirty(false);
      if (preview) setPreviewBuffer(preview);
      setCropConsent(false);
      if (report.report.compatible && report.plan.noop) {
        setOutputBuffer(preview ?? null);
        setState("compatible");
      }
      else if (report.plan.target.crop.required) setState("crop_approval_required");
      else setState("planned");
    } catch (caught) {
      handleFailure(caught, operation);
    } finally {
      if (operation === operationRef.current) setProgress(null);
    }
  }

  async function confirmTarget() {
    if (!target || sourceFile || state === "processing") return;
    const operation = beginOperation();
    setState("processing");
    setError(null);
    try {
      const { report } = await client.compileConstraints<ConstraintSet>(
        draftConstraintsJson(),
        onProgress(operation),
      );
      if (operation !== operationRef.current) return;
      setTarget(editableTargetFromConstraints(report));
      setConfirmedConstraintsJson(JSON.stringify(report));
      setTargetDirty(false);
      setState("requirements_ready");
    } catch (caught) {
      handleFailure(caught, operation);
    } finally {
      if (operation === operationRef.current) setProgress(null);
    }
  }

  async function adaptImage() {
    if (!plan || !confirmedConstraintsJson || targetDirty || state === "processing") return;
    if (plan.plan.target.crop.required && (!crop || !cropConsent)) {
      setState("crop_approval_required");
      return;
    }
    const operation = beginOperation();
    setState("processing");
    setError(null);
    try {
      const { report, output } = await client.adapt<AdaptReport>(
        confirmedConstraintsJson,
        plan.plan.target.crop.required ? crop : null,
        onProgress(operation),
      );
      if (operation !== operationRef.current) return;
      setAdapted(report);
      setOutputBuffer(output ?? null);
      setState(report.status === "compatible" ? "compatible" : "adapted");
    } catch (caught) {
      handleFailure(caught, operation);
    } finally {
      if (operation === operationRef.current) setProgress(null);
    }
  }

  function cancel() {
    ++operationRef.current;
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
    if (state === "processing" || !target || !confirmedConstraintsJson || targetDirty) return;
    const file = event.dataTransfer.files.item(0);
    if (file) void analyzeFile(file);
  }

  const checklist = adapted?.report.checks ?? plan?.report.checks ?? [];
  const status = STATE_COPY[state];

  return (
    <div className="app-shell">
      <header className="site-header">
        <a className="wordmark" href="#top" aria-label="Fitifact home">
          <span className="brand-mark" aria-hidden="true" />
          <span>Fitifact</span>
        </a>
        <p className="privacy-line"><span aria-hidden="true" /> Local processing · No uploads</p>
      </header>

      <main id="top">
        <section className="hero" aria-labelledby="page-title">
          <div className="hero-copy">
            <p className="eyebrow">Image compatibility, solved locally</p>
            <h1 id="page-title">Make your image<br />pass the upload</h1>
            <p className="lede">Paste the requirements. Fitifact makes only the changes needed, then checks the result against every rule you confirmed.</p>
          </div>
          <div className="trust-strip" aria-label="Privacy and processing details">
            <p><span>01</span> Your image stays on this device</p>
            <p><span>02</span> Nothing is uploaded to Fitifact</p>
            <p><span>03</span> The result is checked before download</p>
          </div>
        </section>

        <div className="workflow-grid">
          <section className="card requirements-card" aria-labelledby="requirements-title">
            <div className="step-heading"><span>1</span><h2 id="requirements-title">Paste the requirements</h2></div>
            <label htmlFor="requirements">Upload instructions</label>
            <textarea
              id="requirements"
              rows={4}
              value={requirements}
              onChange={(event) => editRequirements(event.target.value)}
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

          <section className="card target-card" aria-labelledby="target-title">
            <div className="step-heading"><span>2</span><h2 id="target-title">Review the target</h2></div>
            {target ? (
              <div className="target-form">
                <fieldset className="format-options" disabled={state === "processing"}><legend>Allowed formats</legend>{(["jpeg", "png"] as const).map((format) => <label key={format}><input type="checkbox" checked={target.formats.includes(format)} onChange={(event) => editTarget((current) => ({ ...current, formats: event.target.checked ? [...current.formats, format] : current.formats.filter((item) => item !== format) }))} /> {format.toUpperCase()}</label>)}</fieldset>
                <label>Maximum bytes<input inputMode="numeric" value={target.maxBytes} disabled={state === "processing"} onChange={(event) => editTarget((current) => ({ ...current, maxBytes: event.target.value }))} placeholder="No limit" /></label>
                <fieldset className="dimension-fields" disabled={state === "processing"}><legend>Width</legend><label>Exact<input aria-label="Exact width" inputMode="numeric" value={target.widthExact} onChange={(event) => editTarget((current) => ({ ...current, widthExact: event.target.value }))} placeholder="Any" /></label><label>Minimum<input aria-label="Minimum width" inputMode="numeric" value={target.widthMin} onChange={(event) => editTarget((current) => ({ ...current, widthMin: event.target.value }))} placeholder="None" /></label><label>Maximum<input aria-label="Maximum width" inputMode="numeric" value={target.widthMax} onChange={(event) => editTarget((current) => ({ ...current, widthMax: event.target.value }))} placeholder="None" /></label></fieldset>
                <fieldset className="dimension-fields" disabled={state === "processing"}><legend>Height</legend><label>Exact<input aria-label="Exact height" inputMode="numeric" value={target.heightExact} onChange={(event) => editTarget((current) => ({ ...current, heightExact: event.target.value }))} placeholder="Any" /></label><label>Minimum<input aria-label="Minimum height" inputMode="numeric" value={target.heightMin} onChange={(event) => editTarget((current) => ({ ...current, heightMin: event.target.value }))} placeholder="None" /></label><label>Maximum<input aria-label="Maximum height" inputMode="numeric" value={target.heightMax} onChange={(event) => editTarget((current) => ({ ...current, heightMax: event.target.value }))} placeholder="None" /></label></fieldset>
                {targetDirty ? <button className="secondary" type="button" onClick={() => void (sourceFile ? replan() : confirmTarget())} disabled={state === "processing"}>{sourceFile ? "Review target changes" : "Confirm target changes"}</button> : null}
              </div>
            ) : <p className="empty-copy">A normalized, editable target will appear here.</p>}
          </section>

          <section className="card upload-card" aria-labelledby="image-title">
            <div className="step-heading"><span>3</span><h2 id="image-title">Choose your image</h2></div>
            <div className={`drop-zone ${dragging ? "is-dragging" : ""} ${!confirmedConstraintsJson || targetDirty ? "is-disabled" : ""}`} aria-disabled={!confirmedConstraintsJson || targetDirty || state === "processing"} onDragOver={(event) => { event.preventDefault(); if (state !== "processing" && confirmedConstraintsJson && !targetDirty) setDragging(true); }} onDragLeave={() => setDragging(false)} onDrop={onDrop}>
              <p><strong>Drop a JPEG or PNG here</strong></p>
              <p id="image-help">HEIC is accepted only in an explicitly approved build.</p>
              {!confirmedConstraintsJson || targetDirty ? <p id="image-prerequisite" className="prerequisite">Review the requirements and target before choosing a file.</p> : null}
              <label className="button-label" htmlFor="image-file">Choose an image</label>
              <input id="image-file" className="visually-hidden" type="file" accept="image/jpeg,image/png,.heic,.heif" aria-describedby={!confirmedConstraintsJson || targetDirty ? "image-help image-prerequisite" : "image-help"} onChange={(event) => { const file = event.currentTarget.files?.item(0); if (file) void analyzeFile(file); event.currentTarget.value = ""; }} disabled={!confirmedConstraintsJson || targetDirty || state === "processing"} />
            </div>
            {sourceFile ? <p className="file-row"><span>{sourceFile.name}</span><span>{formatBytes(sourceFile.size)}</span></p> : null}
          </section>

          <section className={`card status-card ${status.tone}`} aria-labelledby="status-title">
            <div className="step-heading"><span>4</span><h2 id="status-title">Review and download</h2></div>
            <div role="status" aria-live="polite" aria-atomic="true">
              <h3 className="status-title">{status.title}</h3>
              {state === "processing" && progress ? <><p>{progress.stage}</p><progress max="100" value={progress.percent}>{progress.percent}%</progress><p className="privacy-reminder">Your image stays on this device.</p><button className="danger-link" type="button" onClick={cancel}>Cancel processing</button></> : null}
              {error ? <p className="error-copy"><strong>{error.code}</strong><br />{error.message}</p> : null}
              {plan ? <PlanSummary plan={plan} /> : <p className="empty-copy">The inspection, minimum changes, and validation checklist will appear here.</p>}
            </div>

            {plan?.plan.target.crop.required && state !== "adapted" ? (
              <div className="crop-editor" aria-labelledby="crop-title">
                <h3 id="crop-title">Choose the crop</h3>
                {sourceUrl && crop ? <div className="crop-stage"><img src={sourceUrl} alt={`Crop preview of ${sourceFile?.name ?? "selected image"}`} /><span className="crop-mask" aria-hidden="true" style={{ left: `${crop.x * 100}%`, top: `${crop.y * 100}%`, width: `${crop.width * 100}%`, height: `${crop.height * 100}%` }} /></div> : null}
                <label htmlFor="crop-position">{cropAxis(plan.plan.source_width, plan.plan.source_height, plan.plan.target.width, plan.plan.target.height) === "horizontal" ? "Horizontal" : "Vertical"} crop position: {cropPosition}%</label>
                <input id="crop-position" type="range" min="0" max="100" value={cropPosition} disabled={state === "processing"} onChange={(event) => setCropPosition(Number(event.target.value))} />
                <label className="check-label"><input type="checkbox" checked={cropConsent} disabled={state === "processing"} onChange={(event) => setCropConsent(event.target.checked)} /> I approve removing the shaded edges to match {plan.plan.target.width} × {plan.plan.target.height}.</label>
              </div>
            ) : null}

            {(state === "planned" || state === "crop_approval_required") ? <button type="button" onClick={() => void adaptImage()} disabled={Boolean(plan?.plan.target.crop.required && !cropConsent)}>Adapt and validate</button> : null}

            {checklist.length ? <div className="checklist"><h3>Requirement checklist</h3><ul>{checklist.map((check) => <li key={check.constraint_id} className={check.result}><span aria-hidden="true">{check.result === "pass" ? "✓" : check.result === "fail" ? "×" : "?"}</span><span><strong>{check.field}</strong><br />{check.actual ?? "Unknown"} / needs {check.required}</span><span className="sr-result">{check.result}</span></li>)}</ul></div> : null}

            {(state === "adapted" || state === "compatible") ? <p className="validation-boundary">This output was validated against the requirements you confirmed. A destination may still have undocumented rules.</p> : null}

            {downloadUrl && download && (state === "adapted" || state === "compatible") ? <a className="download-button" href={downloadUrl} download={download.name}>{state === "compatible" && !outputBuffer ? "Use original image" : `Download ${download.extension.toUpperCase()}`}</a> : null}
          </section>
        </div>
      </main>

      <footer><p>Your image stays on this device. Fitifact has no upload or cloud fallback.</p><p>{__FITIFACT_HEIC_APPROVED__ ? <>Optional HEIC decoder approved for this build; see the <a href={`${import.meta.env.BASE_URL}THIRD_PARTY_NOTICES.md`}>third-party notices</a>.</> : "HEIC decoder disabled in this build."}</p></footer>
    </div>
  );
}

function PlanSummary({ plan }: { plan: PlanReport }) {
  const image = plan.inspection.image;
  return (
    <div className="plan-summary">
      <p><strong>Source:</strong> {image?.format?.toUpperCase()} · {image?.width} × {image?.height} · {formatBytes(plan.inspection.byte_length)}</p>
      {plan.report.compatible ? <p>No target changes are needed.</p> : <><p><strong>Proposed:</strong> {plan.plan.target.format.toUpperCase()} · {plan.plan.target.width} × {plan.plan.target.height}{plan.plan.target.max_bytes ? ` · at most ${formatBytes(plan.plan.target.max_bytes)}` : ""}</p><ul>{plan.plan.warnings.map((warning) => <li key={warning}>{warning}</li>)}</ul></>}
    </div>
  );
}
