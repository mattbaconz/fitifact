import { useEffect, useMemo, useRef, useState, type DragEvent } from "react";
import { constraintSetFromEditable, editableTargetFromConstraints, formatBytes } from "./lib/constraints";
import { cropAxis, cropForAspect } from "./lib/crop";
import { describeActions, describeProblems, inspectLine } from "./lib/explain";
import { deleteSavedTarget, listSavedTargets, saveTarget, type SavedTarget } from "./lib/saved-targets";
import { summarizeTarget } from "./lib/target-summary";
import type {
  AdaptReport,
  ConstraintSet,
  EditableTarget,
  ErrorReport,
  InspectReport,
  PlanReport,
  ProductState,
  RequirementParse,
} from "./types";
import { ImageWorkerClient, WorkerFailure, type ProgressUpdate } from "./worker/client";

const STATE_COPY: Record<ProductState, { title: string; tone: string }> = {
  idle: { title: "Drop an image to start", tone: "neutral" },
  inspected: { title: "Paste what the form told you", tone: "neutral" },
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
  unsupported_heic: { title: "This is a phone photo this build cannot decode yet", tone: "warning" },
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

function applyPlanState(report: PlanReport): ProductState {
  if (report.report.compatible && report.plan.noop) return "compatible";
  if (report.plan.target.crop.required) return "crop_approval_required";
  return "planned";
}

export function App() {
  const clientRef = useRef<ImageWorkerClient | null>(null);
  const operationRef = useRef(0);
  const parseGen = useRef(0);
  const parseTimer = useRef<number | null>(null);
  if (!clientRef.current) clientRef.current = new ImageWorkerClient();
  const client = clientRef.current;
  const [requirements, setRequirements] = useState("");
  const [parsed, setParsed] = useState<RequirementParse | null>(null);
  const [target, setTarget] = useState<EditableTarget | null>(null);
  const [confirmedConstraintsJson, setConfirmedConstraintsJson] = useState<string | null>(null);
  const [targetDirty, setTargetDirty] = useState(false);
  const [editingTarget, setEditingTarget] = useState(false);
  const [state, setState] = useState<ProductState>("idle");
  const [progress, setProgress] = useState<ProgressUpdate | null>(null);
  const [error, setError] = useState<ErrorReport | null>(null);
  const [sourceFile, setSourceFile] = useState<File | null>(null);
  const [inspection, setInspection] = useState<InspectReport | null>(null);
  const [plan, setPlan] = useState<PlanReport | null>(null);
  const [adapted, setAdapted] = useState<AdaptReport | null>(null);
  const [outputBuffer, setOutputBuffer] = useState<ArrayBuffer | null>(null);
  const [previewBuffer, setPreviewBuffer] = useState<ArrayBuffer | null>(null);
  const [cropPosition, setCropPosition] = useState(50);
  const [cropConsent, setCropConsent] = useState(false);
  const [dragging, setDragging] = useState(false);
  const [savedTargets, setSavedTargets] = useState<SavedTarget[]>(() => listSavedTargets());
  const [targetName, setTargetName] = useState("");
  const confirmedRef = useRef<string | null>(null);
  const targetDirtyRef = useRef(false);
  const processingRef = useRef(false);
  confirmedRef.current = confirmedConstraintsJson;
  targetDirtyRef.current = targetDirty;

  useEffect(() => () => {
    client.dispose();
    if (parseTimer.current) window.clearTimeout(parseTimer.current);
  }, [client]);

  const inspectedSafeImage = Boolean(
    sourceFile &&
      inspection?.kind &&
      ["jpeg", "png", "webp"].includes(inspection.kind),
  );
  const previewBlob = useMemo(
    () => (previewBuffer ? new Blob([previewBuffer], { type: "image/png" }) : null),
    [previewBuffer],
  );
  const sourceUrl = useObjectUrl(previewBlob ?? (inspectedSafeImage ? sourceFile : null));
  const outputFormat = adapted?.output_artifact.image?.format ?? plan?.plan.target.format ?? null;
  const download = useMemo(
    () => (outputFormat && sourceFile && (outputFormat === "jpeg" || outputFormat === "png")
      ? outputDetails(outputFormat, sourceFile.name)
      : null),
    [outputFormat, sourceFile],
  );
  const outputBlob = useMemo(
    () => (outputBuffer && download ? new Blob([outputBuffer], { type: download.mime }) : null),
    [outputBuffer, download],
  );
  const originalBlob = useMemo(
    () => state === "compatible" && !outputBuffer && sourceFile && download && inspection?.kind !== "heic"
      ? new Blob([sourceFile], { type: download.mime })
      : null,
    [state, outputBuffer, sourceFile, download, inspection],
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
    if (clearSource) {
      setSourceFile(null);
      setInspection(null);
    }
  }

  function idleAfterClear() {
    if (sourceFile && inspection) setState("inspected");
    else if (confirmedConstraintsJson && !targetDirty) setState("requirements_ready");
    else setState("idle");
  }

  function scheduleParse(value: string) {
    if (parseTimer.current) window.clearTimeout(parseTimer.current);
    parseTimer.current = window.setTimeout(() => {
      void autoParse(value);
    }, 400);
  }

  function editRequirements(value: string) {
    parseGen.current += 1;
    setRequirements(value);
    setParsed(null);
    setTarget(null);
    setConfirmedConstraintsJson(null);
    setTargetDirty(false);
    setEditingTarget(false);
    clearDerivedState(false);
    setError(null);
    if (!value.trim()) {
      idleAfterClear();
      return;
    }
    if (sourceFile) setState("inspected");
    else setState("idle");
    scheduleParse(value);
  }

  function editTarget(update: (current: EditableTarget) => EditableTarget) {
    if (!target) return;
    ++operationRef.current;
    setTarget((current) => current ? update(current) : current);
    setTargetDirty(true);
    clearDerivedState(false);
    setError(null);
    setState(sourceFile ? "inspected" : "requirements_ready");
  }

  async function autoParse(value: string) {
    const text = value.trim();
    if (!text) return;
    const gen = ++parseGen.current;
    try {
      const { report } = await client.compile<RequirementParse>(text);
      if (gen !== parseGen.current) return;
      setParsed(report);
      if (report.ambiguities.length) {
        setTarget(null);
        setConfirmedConstraintsJson(null);
        setEditingTarget(true);
        setError({
          schema: "fitifact.error/v1",
          code: "REQUIREMENTS_AMBIGUOUS",
          message: report.ambiguities.map((item) => item.message).join(" "),
        });
        setState(sourceFile ? "inspected" : "error");
        return;
      }
      if (!report.constraints) {
        setTarget(null);
        setConfirmedConstraintsJson(null);
        setError({
          schema: "fitifact.error/v1",
          code: "INPUT_INVALID",
          message: "No supported image format, size, or dimension requirement was found.",
        });
        setState(sourceFile ? "inspected" : "error");
        return;
      }
      const editable = editableTargetFromConstraints(report.constraints);
      const confirmed = JSON.stringify(report.constraints);
      setTarget(editable);
      setConfirmedConstraintsJson(confirmed);
      setTargetDirty(false);
      setError(null);
      if (sourceFile && !processingRef.current) await runPlan(confirmed);
      else if (sourceFile) setState("inspected");
      else setState("requirements_ready");
    } catch (caught) {
      if (gen !== parseGen.current) return;
      handleFailure(caught, operationRef.current);
    }
  }

  function draftConstraintsJson(): string {
    if (!target) throw new Error("Review a valid requirement first.");
    return JSON.stringify(constraintSetFromEditable(target));
  }

  async function inspectFile(file: File) {
    if (state === "processing") return;
    const operation = beginOperation();
    setSourceFile(file);
    setInspection(null);
    setPlan(null);
    setAdapted(null);
    setOutputBuffer(null);
    setPreviewBuffer(null);
    setCropConsent(false);
    setError(null);
    processingRef.current = true;
    setState("processing");
    try {
      const { report } = await client.inspect<InspectReport>(file, onProgress(operation));
      if (operation !== operationRef.current) return;
      setInspection(report);
      if (confirmedRef.current && !targetDirtyRef.current) await runPlan(confirmedRef.current, operation);
      else setState("inspected");
    } catch (caught) {
      handleFailure(caught, operation);
    } finally {
      if (operation === operationRef.current) {
        setProgress(null);
        processingRef.current = false;
      }
    }
  }

  async function runPlan(constraintsJson: string, existingOperation?: number) {
    const operation = existingOperation ?? beginOperation();
    processingRef.current = true;
    setState("processing");
    setError(null);
    setPlan(null);
    setAdapted(null);
    setOutputBuffer(null);
    try {
      const { report, preview, constraintsSnapshot } = await client.plan<PlanReport>(
        constraintsJson,
        onProgress(operation),
      );
      if (operation !== operationRef.current) return;
      setPlan(report);
      setConfirmedConstraintsJson(constraintsSnapshot ?? constraintsJson);
      if (preview) setPreviewBuffer(preview);
      setCropConsent(false);
      const next = applyPlanState(report);
      if (next === "compatible") setOutputBuffer(preview ?? null);
      setState(next);
    } catch (caught) {
      handleFailure(caught, operation);
    } finally {
      if (operation === operationRef.current) {
        setProgress(null);
        processingRef.current = false;
      }
    }
  }

  async function replan() {
    if (!sourceFile || !confirmedConstraintsJson || !target || state === "processing") return;
    const operation = beginOperation();
    processingRef.current = true;
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
      setEditingTarget(false);
      if (preview) setPreviewBuffer(preview);
      setCropConsent(false);
      const next = applyPlanState(report);
      if (next === "compatible") setOutputBuffer(preview ?? null);
      setState(next);
    } catch (caught) {
      handleFailure(caught, operation);
    } finally {
      if (operation === operationRef.current) {
        setProgress(null);
        processingRef.current = false;
      }
    }
  }

  async function confirmTarget() {
    if (!target || sourceFile || state === "processing") return;
    const operation = beginOperation();
    processingRef.current = true;
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
      setEditingTarget(false);
      setState("requirements_ready");
    } catch (caught) {
      handleFailure(caught, operation);
    } finally {
      if (operation === operationRef.current) {
        setProgress(null);
        processingRef.current = false;
      }
    }
  }

  async function adaptImage() {
    if (!plan || !confirmedConstraintsJson || targetDirty || state === "processing") return;
    if (plan.plan.target.crop.required && (!crop || !cropConsent)) {
      setState("crop_approval_required");
      return;
    }
    const operation = beginOperation();
    processingRef.current = true;
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
      if (operation === operationRef.current) {
        setProgress(null);
        processingRef.current = false;
      }
    }
  }

  function cancel() {
    ++operationRef.current;
    client.cancel();
    processingRef.current = false;
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
    if (state === "processing") return;
    const file = event.dataTransfer.files.item(0);
    if (file) void inspectFile(file);
  }

  function persistTarget() {
    if (!target || !confirmedConstraintsJson) return;
    try {
      const saved = saveTarget({
        name: targetName || summarizeTarget(target),
        requirements,
        constraintsJson: confirmedConstraintsJson,
      });
      setSavedTargets(listSavedTargets());
      setTargetName(saved.name);
    } catch (caught) {
      setError({
        schema: "fitifact.error/v1",
        code: "INPUT_INVALID",
        message: caught instanceof Error ? caught.message : "Could not save this target.",
      });
    }
  }

  async function applySaved(saved: SavedTarget) {
    parseGen.current += 1;
    if (parseTimer.current) window.clearTimeout(parseTimer.current);
    setRequirements(saved.requirements);
    setParsed(null);
    setError(null);
    setTargetDirty(false);
    setEditingTarget(false);
    setTargetName(saved.name);
    clearDerivedState(false);
    try {
      const { report } = await client.compileConstraints<ConstraintSet>(saved.constraintsJson);
      setTarget(editableTargetFromConstraints(report));
      const confirmed = JSON.stringify(report);
      setConfirmedConstraintsJson(confirmed);
      if (sourceFile) await runPlan(confirmed);
      else setState("requirements_ready");
    } catch (caught) {
      handleFailure(caught, operationRef.current);
    }
  }

  const checklist = adapted?.report.checks ?? plan?.report.checks ?? [];
  const status = STATE_COPY[state];
  const problems = plan ? describeProblems(plan) : [];
  const actions = plan ? describeActions(plan) : [];
  const inspectFacts = inspection
    ? inspectLine(
        inspection.kind,
        inspection.artifact.image?.width,
        inspection.artifact.image?.height,
        inspection.artifact.byte_length,
      )
    : null;

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
          <p className="eyebrow">Upload rejected?</p>
          <h1 id="page-title">Make your image pass the upload</h1>
          <p className="lede">Give Fitifact the file and what the form told you.</p>
        </section>

        <div className="workflow-grid">
          <section className="card upload-card" aria-labelledby="image-title">
            <div className="step-heading"><span>1</span><h2 id="image-title">Drop your image</h2></div>
            <div
              className={`drop-zone ${dragging ? "is-dragging" : ""} ${state === "processing" ? "is-disabled" : ""}`}
              aria-disabled={state === "processing"}
              onDragOver={(event) => { event.preventDefault(); if (state !== "processing") setDragging(true); }}
              onDragLeave={() => setDragging(false)}
              onDrop={onDrop}
            >
              <p><strong>JPEG, PNG, WebP, or a phone photo (HEIC)</strong></p>
              <p id="image-help">{__FITIFACT_HEIC_APPROVED__ ? "Processed locally. Nothing is uploaded." : "HEIC phone photos cannot be decoded in this build."}</p>
              <label className="button-label" htmlFor="image-file">Choose an image</label>
              <input
                id="image-file"
                className="visually-hidden"
                type="file"
                accept="image/jpeg,image/png,image/webp,.heic,.heif"
                aria-describedby="image-help"
                onChange={(event) => {
                  const file = event.currentTarget.files?.item(0);
                  if (file) void inspectFile(file);
                  event.currentTarget.value = "";
                }}
                disabled={state === "processing"}
              />
            </div>
            {sourceFile ? (
              <p className="file-row">
                <span>{sourceFile.name}</span>
                <span>{inspectFacts ?? formatBytes(sourceFile.size)}</span>
              </p>
            ) : null}
            <div className="trust-strip" aria-label="Privacy and processing details">
              <p><span>01</span> Your image stays on this device</p>
              <p><span>02</span> Nothing is uploaded to Fitifact</p>
              <p><span>03</span> The result is checked before download</p>
            </div>
          </section>

          <section className="card requirements-card" aria-labelledby="requirements-title">
            <div className="step-heading"><span>2</span><h2 id="requirements-title">What did the upload form tell you?</h2></div>
            <label htmlFor="requirements">Rejection message or requirements</label>
            <p id="requirements-hint" className="field-hint">Paste the rejection or size and format rules from the form.</p>
            <textarea
              id="requirements"
              rows={4}
              value={requirements}
              aria-describedby="requirements-hint"
              onChange={(event) => editRequirements(event.target.value)}
              disabled={state === "processing"}
            />
            {parsed?.unresolved.length ? (
              <div className="notice" role="note">
                <strong>Not used:</strong> {parsed.unresolved.map((item) => item.text.trim()).filter(Boolean).join(" · ")}
              </div>
            ) : null}
            {target && confirmedConstraintsJson ? (
              <div className="saved-targets">
                <label htmlFor="target-name">Save this target on this device</label>
                <div className="saved-row">
                  <input id="target-name" value={targetName} disabled={state === "processing"} onChange={(event) => setTargetName(event.target.value)} placeholder={summarizeTarget(target)} />
                  <button className="secondary" type="button" onClick={persistTarget} disabled={state === "processing"}>Save target</button>
                </div>
                {savedTargets.length ? (
                  <ul className="saved-list">
                    {savedTargets.map((saved) => (
                      <li key={saved.id}>
                        <button type="button" className="saved-chip" disabled={state === "processing"} onClick={() => void applySaved(saved)}>{saved.name}</button>
                        <button type="button" className="danger-link saved-delete" disabled={state === "processing"} onClick={() => { deleteSavedTarget(saved.id); setSavedTargets(listSavedTargets()); }}>Remove {saved.name}</button>
                      </li>
                    ))}
                  </ul>
                ) : null}
              </div>
            ) : null}
          </section>

          <section className="card target-card" aria-labelledby="target-title">
            <div className="step-heading"><span>3</span><h2 id="target-title">I understood this as</h2></div>
            {target ? (
              <>
                <p className="target-summary">{summarizeTarget(target)}</p>
                <div className="target-actions">
                  <button
                    className="secondary"
                    type="button"
                    onClick={() => {
                      if (targetDirty) void (sourceFile ? replan() : confirmTarget());
                      else setEditingTarget(false);
                    }}
                    disabled={state === "processing"}
                  >
                    Looks right
                  </button>
                  <button className="secondary" type="button" onClick={() => setEditingTarget((open) => !open)} disabled={state === "processing"}>
                    {editingTarget ? "Hide editor" : "Edit"}
                  </button>
                </div>
                {editingTarget ? (
                  <div className="target-form">
                    <fieldset className="format-options" disabled={state === "processing"}><legend>Allowed formats</legend>{(["jpeg", "png"] as const).map((format) => <label key={format}><input type="checkbox" checked={target.formats.includes(format)} onChange={(event) => editTarget((current) => ({ ...current, formats: event.target.checked ? [...current.formats, format] : current.formats.filter((item) => item !== format) }))} /> {format.toUpperCase()}</label>)}</fieldset>
                    <label>Maximum bytes<input inputMode="numeric" value={target.maxBytes} disabled={state === "processing"} onChange={(event) => editTarget((current) => ({ ...current, maxBytes: event.target.value }))} placeholder="No limit" /></label>
                    <fieldset className="dimension-fields" disabled={state === "processing"}><legend>Width</legend><label>Exact<input aria-label="Exact width" inputMode="numeric" value={target.widthExact} onChange={(event) => editTarget((current) => ({ ...current, widthExact: event.target.value }))} placeholder="Any" /></label><label>Minimum<input aria-label="Minimum width" inputMode="numeric" value={target.widthMin} onChange={(event) => editTarget((current) => ({ ...current, widthMin: event.target.value }))} placeholder="None" /></label><label>Maximum<input aria-label="Maximum width" inputMode="numeric" value={target.widthMax} onChange={(event) => editTarget((current) => ({ ...current, widthMax: event.target.value }))} placeholder="None" /></label></fieldset>
                    <fieldset className="dimension-fields" disabled={state === "processing"}><legend>Height</legend><label>Exact<input aria-label="Exact height" inputMode="numeric" value={target.heightExact} onChange={(event) => editTarget((current) => ({ ...current, heightExact: event.target.value }))} placeholder="Any" /></label><label>Minimum<input aria-label="Minimum height" inputMode="numeric" value={target.heightMin} onChange={(event) => editTarget((current) => ({ ...current, heightMin: event.target.value }))} placeholder="None" /></label><label>Maximum<input aria-label="Maximum height" inputMode="numeric" value={target.heightMax} onChange={(event) => editTarget((current) => ({ ...current, heightMax: event.target.value }))} placeholder="None" /></label></fieldset>
                    {targetDirty ? <button className="secondary" type="button" onClick={() => void (sourceFile ? replan() : confirmTarget())} disabled={state === "processing"}>{sourceFile ? "Review target changes" : "Confirm target changes"}</button> : null}
                  </div>
                ) : null}
              </>
            ) : <p className="empty-copy">Paste the rejection or requirements and Fitifact will show the normalized target here.</p>}
          </section>

          <section className={`card status-card ${status.tone}`} aria-labelledby="status-title">
            <div className="step-heading"><span>4</span><h2 id="status-title">Review and download</h2></div>
            <div role="status" aria-live="polite" aria-atomic="true">
              <h3 className="status-title">{status.title}</h3>
              {state === "processing" && progress ? <><p>{progress.stage}</p><progress max="100" value={progress.percent}>{progress.percent}%</progress><p className="privacy-reminder">Your image stays on this device.</p><button className="danger-link" type="button" onClick={cancel}>Cancel processing</button></> : null}
              {error ? <p className="error-copy"><strong>{error.code}</strong><br />{error.message}</p> : null}
              {inspectFacts && !plan ? <p className="empty-copy">{inspectFacts}</p> : null}
              {plan ? (
                <div className="plan-summary">
                  <p><strong>Your file:</strong> {inspectFacts ?? `${plan.inspection.image?.format?.toUpperCase()} · ${plan.inspection.image?.width} × ${plan.inspection.image?.height} · ${formatBytes(plan.inspection.byte_length)}`}</p>
                  {plan.report.compatible && plan.plan.noop ? <p>No target changes are needed.</p> : (
                    <>
                      {problems.length ? <><p><strong>{problems.length} problem{problems.length === 1 ? "" : "s"} found</strong></p><ul>{problems.map((item) => <li key={item}>{item}</li>)}</ul></> : null}
                      {actions.length ? <><p><strong>What I’ll do</strong></p><ul>{actions.map((item) => <li key={item}>{item}</li>)}</ul></> : null}
                    </>
                  )}
                </div>
              ) : state === "processing" ? null : <p className="empty-copy">Drop a file, paste what the form said, then Fitifact will explain the minimum changes.</p>}
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

            {(state === "planned" || state === "crop_approval_required") ? <button type="button" onClick={() => void adaptImage()} disabled={Boolean(plan?.plan.target.crop.required && !cropConsent)}>Fix image</button> : null}

            {checklist.length ? <div className="checklist"><h3>Requirement checklist</h3><ul>{checklist.map((check) => <li key={check.constraint_id} className={check.result}><span aria-hidden="true">{check.result === "pass" ? "✓" : check.result === "fail" ? "×" : "?"}</span><span><strong>{check.field}</strong><br />{check.actual ?? "Unknown"} / needs {check.required}</span><span className="sr-result">{check.result}</span></li>)}</ul></div> : null}

            {(state === "adapted" || state === "compatible") ? <p className="validation-boundary">This output was validated against the requirements you confirmed. A destination may still have undocumented rules.</p> : null}

            {downloadUrl && download && (state === "adapted" || state === "compatible") ? <a className="download-button" href={downloadUrl} download={download.name}>{state === "compatible" && !outputBuffer ? "Use original image" : `Download ${download.extension.toUpperCase()}`}</a> : null}
          </section>
        </div>
      </main>

      <footer>
        <p>Your image stays on this device. Fitifact has no upload or cloud fallback.</p>
        <p>{__FITIFACT_HEIC_APPROVED__ ? <>HEIC phone photos decode locally; see the <a href={`${import.meta.env.BASE_URL}THIRD_PARTY_NOTICES.md`}>third-party notices</a>.</> : "HEIC decoder disabled in this build."}</p>
      </footer>
    </div>
  );
}
