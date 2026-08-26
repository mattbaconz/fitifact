import { useEffect, useMemo, useRef, useState, type ClipboardEvent, type DragEvent, type KeyboardEvent } from "react";
import { constraintSetFromEditable, editableTargetFromConstraints, formatBytes } from "./lib/constraints";
import { cropAxis, cropForAspect } from "./lib/crop";
import { errorCopy } from "./lib/error-copy";
import { describeActions, describeProblems, inspectLine, leftoverNote } from "./lib/explain";
import { deleteSavedTarget, listSavedTargets, saveTarget, type SavedTarget } from "./lib/saved-targets";
import {
  clearLastTarget,
  loadLastTarget,
  loadSettings,
  saveLastTarget,
  saveSettings,
  type AppSettings,
} from "./lib/settings";
import { summarizeTarget } from "./lib/target-summary";
import type {
  AdaptReport,
  ConstraintSet,
  EditableTarget,
  ErrorReport,
  InspectReport,
  OutputFormat,
  PlanReport,
  ProductState,
  RequirementParse,
} from "./types";
import { ImageWorkerClient, WorkerFailure, type ProgressUpdate } from "./worker/client";

const ACCEPT = ".tif,.tiff,.bmp,.gif,.webp,.heic,.heif,image/*";
const PREVIEW_KINDS = new Set(["jpeg", "png", "webp", "gif", "bmp"]);

const STATE_COPY: Record<ProductState, { title: string; tone: string }> = {
  idle: { title: "Drop a file", tone: "neutral" },
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

function outputDetails(format: OutputFormat, originalName: string, suffix: boolean) {
  const extension = format === "jpeg" ? "jpg" : format;
  const mime = format === "jpeg" ? "image/jpeg" : format === "png" ? "image/png" : "image/webp";
  const stem = originalName.replace(/\.[^.]*$/, "").replace(/[^a-zA-Z0-9._-]+/g, "-") || "image";
  return { extension, mime, name: suffix ? `${stem}.fitifact.${extension}` : `${stem}.${extension}` };
}

function applyPlanState(report: PlanReport): ProductState {
  if (report.report.compatible && report.plan.noop) return "compatible";
  if (report.plan.target.crop.required || report.plan.target.first_frame?.required) {
    return "crop_approval_required";
  }
  return "planned";
}

function approvalTitle(plan: PlanReport | null): string {
  const crop = Boolean(plan?.plan.target.crop.required);
  const frame = Boolean(plan?.plan.target.first_frame?.required);
  if (crop && frame) return "Approval required";
  if (frame) return "First-frame approval required";
  return "Crop approval required";
}

function sessionFromLastTarget(): {
  requirements: string;
  target: EditableTarget | null;
  confirmed: string | null;
} {
  const last = loadLastTarget();
  if (!last) return { requirements: "", target: null, confirmed: null };
  try {
    const constraints = JSON.parse(last.constraintsJson) as ConstraintSet;
    if (constraints.schema !== "fitifact.constraints/v1" || !Array.isArray(constraints.hard)) {
      throw new Error("invalid last target");
    }
    return {
      requirements: last.requirements,
      target: editableTargetFromConstraints(constraints),
      confirmed: JSON.stringify(constraints),
    };
  } catch {
    clearLastTarget();
    return { requirements: "", target: null, confirmed: null };
  }
}

export function App() {
  const clientRef = useRef<ImageWorkerClient | null>(null);
  const operationRef = useRef(0);
  const parseGen = useRef(0);
  const parseTimer = useRef<number | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const sidebarRef = useRef<HTMLElement | null>(null);
  if (!clientRef.current) clientRef.current = new ImageWorkerClient();
  const client = clientRef.current;
  const [seed] = useState(sessionFromLastTarget);
  const [requirements, setRequirements] = useState(seed.requirements);
  const [parsed, setParsed] = useState<RequirementParse | null>(null);
  const [target, setTarget] = useState<EditableTarget | null>(seed.target);
  const [confirmedConstraintsJson, setConfirmedConstraintsJson] = useState<string | null>(seed.confirmed);
  const [targetDirty, setTargetDirty] = useState(false);
  const [editingTarget, setEditingTarget] = useState(false);
  const [heicPreviewMissing, setHeicPreviewMissing] = useState(false);
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
  const [firstFrameConsent, setFirstFrameConsent] = useState(false);
  const [dragging, setDragging] = useState(false);
  const [savedTargets, setSavedTargets] = useState<SavedTarget[]>(() => listSavedTargets());
  const [targetName, setTargetName] = useState("");
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [settings, setSettings] = useState<AppSettings>(() => loadSettings());
  const confirmedRef = useRef<string | null>(null);
  const targetDirtyRef = useRef(false);
  const processingRef = useRef(false);
  const settingsRef = useRef(settings);
  confirmedRef.current = confirmedConstraintsJson;
  targetDirtyRef.current = targetDirty;
  settingsRef.current = settings;

  useEffect(() => () => {
    client.dispose();
    if (parseTimer.current) window.clearTimeout(parseTimer.current);
  }, [client]);

  const inspectRef = useRef<(file: File) => Promise<void>>(async () => undefined);

  useEffect(() => {
    function onPaste(event: globalThis.ClipboardEvent) {
      const data = event.clipboardData;
      if (!data || processingRef.current) return;
      let file: File | null = null;
      for (const item of data.items) {
        if (!item.type.startsWith("image/")) continue;
        file = item.getAsFile();
        if (file) break;
      }
      if (!file) {
        file = [...data.files].find((candidate) => candidate.type.startsWith("image/")) ?? null;
      }
      if (!file) return;
      event.preventDefault();
      void inspectRef.current(file);
    }
    window.addEventListener("paste", onPaste);
    return () => window.removeEventListener("paste", onPaste);
  }, []);

  useEffect(() => {
    if (!sidebarOpen) return;
    const sidebar = sidebarRef.current;
    const previously = document.activeElement as HTMLElement | null;
    const focusable = () => Array.from(
      sidebar?.querySelectorAll<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
      ) ?? [],
    ).filter((node) => !node.hasAttribute("disabled"));
    focusable()[0]?.focus();
    function onKey(event: globalThis.KeyboardEvent) {
      if (event.key === "Escape") {
        event.preventDefault();
        setSidebarOpen(false);
        return;
      }
      if (event.key !== "Tab") return;
      const nodes = focusable();
      if (!nodes.length) return;
      const first = nodes[0];
      const last = nodes[nodes.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    }
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("keydown", onKey);
      previously?.focus();
    };
  }, [sidebarOpen]);

  const inspectedSafeImage = Boolean(inspection?.kind && PREVIEW_KINDS.has(inspection.kind));
  const previewBlob = useMemo(
    () => (previewBuffer ? new Blob([previewBuffer], { type: "image/png" }) : null),
    [previewBuffer],
  );
  const sourceUrl = useObjectUrl(previewBlob ?? (inspectedSafeImage ? sourceFile : null));
  const outputFormat = adapted?.output_artifact.image?.format ?? plan?.plan.target.format ?? null;
  const download = useMemo(
    () => (outputFormat && sourceFile && (outputFormat === "jpeg" || outputFormat === "png" || outputFormat === "webp")
      ? outputDetails(outputFormat, sourceFile.name, settings.fitifactSuffix)
      : null),
    [outputFormat, sourceFile, settings.fitifactSuffix],
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
    setFirstFrameConsent(false);
    setProgress(null);
    if (clearSource) {
      setSourceFile(null);
      setInspection(null);
      setHeicPreviewMissing(false);
    }
  }

  function persistConfirmed(constraintsJson: string, text = requirements) {
    setConfirmedConstraintsJson(constraintsJson);
    saveLastTarget({ requirements: text, constraintsJson });
  }

  function scheduleParse(value: string) {
    if (parseTimer.current) window.clearTimeout(parseTimer.current);
    parseTimer.current = window.setTimeout(() => {
      void autoParse(value);
    }, 400);
  }

  function editRequirements(value: string, immediate = false) {
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
      setState(inspection ? "inspected" : "idle");
      return;
    }
    setState(inspection ? "inspected" : "idle");
    if (immediate) void autoParse(value);
    else scheduleParse(value);
  }

  function onRequirementsPaste(event: ClipboardEvent<HTMLTextAreaElement>) {
    const imageItem = [...(event.clipboardData?.items ?? [])].find((item) => item.type.startsWith("image/"));
    if (imageItem) return;
    const pasted = event.clipboardData.getData("text/plain");
    if (!pasted) return;
    event.preventDefault();
    const area = event.currentTarget;
    const start = area.selectionStart ?? area.value.length;
    const end = area.selectionEnd ?? area.value.length;
    editRequirements(`${area.value.slice(0, start)}${pasted}${area.value.slice(end)}`, true);
  }

  function editTarget(update: (current: EditableTarget) => EditableTarget) {
    if (!target) return;
    ++operationRef.current;
    setTarget((current) => current ? update(current) : current);
    setTargetDirty(true);
    clearDerivedState(false);
    setError(null);
    setState(inspection ? "inspected" : "requirements_ready");
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
        setState(inspection ? "inspected" : "error");
        return;
      }
      if (!report.constraints) {
        setTarget(null);
        setConfirmedConstraintsJson(null);
        setError({
          schema: "fitifact.error/v1",
          code: "INPUT_INVALID",
          message: errorCopy("INPUT_INVALID", "No supported image format, size, or dimension requirement was found."),
        });
        setState(inspection ? "inspected" : "error");
        return;
      }
      const editable = editableTargetFromConstraints(report.constraints);
      const confirmed = JSON.stringify(report.constraints);
      setTarget(editable);
      persistConfirmed(confirmed, text);
      setTargetDirty(false);
      setError(null);
      if (sourceFile && inspection && !processingRef.current) await runPlan(confirmed);
      else if (inspection) setState("inspected");
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
    setHeicPreviewMissing(false);
    setCropConsent(false);
    setFirstFrameConsent(settingsRef.current.firstFrameConsentDefault);
    setError(null);
    processingRef.current = true;
    setState("processing");
    try {
      const { report, preview } = await client.inspect<InspectReport>(file, onProgress(operation));
      if (operation !== operationRef.current) return;
      setInspection(report);
      setHeicPreviewMissing(report.kind === "heic" && !preview);
      if (preview) setPreviewBuffer(preview);
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

  inspectRef.current = inspectFile;

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
      persistConfirmed(constraintsSnapshot ?? constraintsJson);
      if (preview) setPreviewBuffer(preview);
      setCropConsent(false);
      setFirstFrameConsent(settingsRef.current.firstFrameConsentDefault);
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
      persistConfirmed(constraintsSnapshot ?? draft);
      setTargetDirty(false);
      setEditingTarget(false);
      if (preview) setPreviewBuffer(preview);
      setCropConsent(false);
      setFirstFrameConsent(settingsRef.current.firstFrameConsentDefault);
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

  async function adaptImage() {
    if (!plan || !confirmedConstraintsJson || targetDirty || state === "processing") return;
    if (plan.plan.target.crop.required && (!crop || !cropConsent)) {
      setState("crop_approval_required");
      return;
    }
    if (plan.plan.target.first_frame?.required && !firstFrameConsent) {
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
        firstFrameConsent,
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

  function onDropKey(event: KeyboardEvent<HTMLDivElement>) {
    if (event.key !== "Enter" && event.key !== " ") return;
    event.preventDefault();
    fileInputRef.current?.click();
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
      persistConfirmed(confirmed, saved.requirements);
      if (inspection) await runPlan(confirmed);
      else setState("requirements_ready");
    } catch (caught) {
      handleFailure(caught, operationRef.current);
    }
  }

  function updateSettings(next: AppSettings) {
    setSettings(next);
    saveSettings(next);
  }

  async function useSampleImage() {
    if (state === "processing") return;
    const response = await fetch(`${import.meta.env.BASE_URL}samples/too-small-640x480.png`);
    const blob = await response.blob();
    setSidebarOpen(false);
    void inspectFile(new File([blob], "too-small-640x480.png", { type: "image/png" }));
  }

  const checklist = adapted?.report.checks ?? plan?.report.checks ?? [];
  const status = STATE_COPY[state];
  const statusTitle = state === "crop_approval_required" ? approvalTitle(plan) : status.title;
  const problems = plan ? describeProblems(plan) : [];
  const actions = plan ? describeActions(plan) : [];
  const leftover = leftoverNote(parsed?.unresolved.map((item) => item.text) ?? []);
  const inspectFacts = inspection
    ? inspectLine(
        inspection.kind,
        inspection.artifact.image?.width,
        inspection.artifact.image?.height,
        inspection.artifact.byte_length,
      )
    : null;
  const showWork = Boolean(inspection);
  const needsApproval = Boolean(
    (plan?.plan.target.crop.required && !cropConsent) ||
      (plan?.plan.target.first_frame?.required && !firstFrameConsent),
  );
  const formatOptions: OutputFormat[] = ["jpeg", "png", "webp"];

  return (
    <div className={`app-shell ${showWork ? "has-file" : "is-idle"}`}>
      <header className="site-header">
        <a className="wordmark" href="#top" aria-label="Fitifact home">
          <img className="brand-mark" src={`${import.meta.env.BASE_URL}ft-logo.png`} alt="" />
          <span>Fitifact</span>
        </a>
        <p className="privacy-line">Local · nothing is uploaded</p>
        <button
          type="button"
          className="ghost"
          aria-expanded={sidebarOpen}
          aria-controls="app-sidebar"
          onClick={() => setSidebarOpen(true)}
        >
          Menu
        </button>
      </header>

      <main id="top">
        {!showWork ? (
          <div className="drop-canvas">
            <div
              className={`drop-zone drop-zone-hero ${dragging ? "is-dragging" : ""} ${state === "processing" ? "is-disabled" : ""}`}
              aria-disabled={state === "processing"}
              aria-label="Drop a file"
              tabIndex={0}
              onKeyDown={onDropKey}
              onDragOver={(event) => { event.preventDefault(); if (state !== "processing") setDragging(true); }}
              onDragLeave={() => setDragging(false)}
              onDrop={onDrop}
            >
              <p>Drop a file</p>
              <p id="image-help">JPEG PNG WebP HEIC · TIFF BMP GIF</p>
              <label className="button-label" htmlFor="image-file">Choose an image</label>
              <input
                id="image-file"
                ref={fileInputRef}
                className="visually-hidden"
                type="file"
                accept={ACCEPT}
                aria-describedby="image-help"
                onChange={(event) => {
                  const file = event.currentTarget.files?.item(0);
                  if (file) void inspectFile(file);
                  event.currentTarget.value = "";
                }}
                disabled={state === "processing"}
              />
            </div>
            <div className={`idle-status ${status.tone}`} role="status" aria-live="polite" aria-atomic="true">
              {state !== "idle" ? <h2 className="status-title">{statusTitle}</h2> : null}
              {state === "processing" && progress ? (
                <>
                  <p>{progress.stage}</p>
                  <progress max="100" value={progress.percent}>{progress.percent}%</progress>
                  <p className="privacy-reminder">Your image stays on this device.</p>
                  <button className="danger-link" type="button" onClick={cancel}>Cancel processing</button>
                </>
              ) : null}
              {error ? <p className="error-copy">{error.message}</p> : null}
            </div>
          </div>
        ) : (
          <div className="work-surface">
            <section className="card file-card">
              <div className="file-chip file-row">
                <strong>{sourceFile?.name}</strong>
                <span>{inspectFacts ?? formatBytes(sourceFile?.size ?? 0)}</span>
                {heicPreviewMissing ? <span>Preview unavailable for this phone photo.</span> : null}
              </div>
              <div
                className={`drop-zone drop-zone-compact ${dragging ? "is-dragging" : ""} ${state === "processing" ? "is-disabled" : ""}`}
                aria-disabled={state === "processing"}
                aria-label="Replace image"
                tabIndex={0}
                onKeyDown={onDropKey}
                onDragOver={(event) => { event.preventDefault(); if (state !== "processing") setDragging(true); }}
                onDragLeave={() => setDragging(false)}
                onDrop={onDrop}
              >
                <label className="button-label" htmlFor="image-file-replace">Choose an image</label>
                <input
                  id="image-file-replace"
                  ref={fileInputRef}
                  className="visually-hidden"
                  type="file"
                  accept={ACCEPT}
                  onChange={(event) => {
                    const file = event.currentTarget.files?.item(0);
                    if (file) void inspectFile(file);
                    event.currentTarget.value = "";
                  }}
                  disabled={state === "processing"}
                />
              </div>
            </section>

            <section className="card destination-card" aria-labelledby="requirements-title">
              <h2 id="requirements-title">Paste what the form said</h2>
              <label htmlFor="requirements">Rejection message or requirements</label>
              <textarea
                id="requirements"
                rows={4}
                value={requirements}
                onPaste={onRequirementsPaste}
                onChange={(event) => editRequirements(event.target.value)}
                disabled={state === "processing"}
              />
              {leftover ? <div className="notice" role="note">{leftover}</div> : null}

              <h2 id="target-title">I understood this as</h2>
              {target ? (
                <>
                  <p className="target-summary">{summarizeTarget(target)}</p>
                  <div className="target-actions">
                    <button
                      className="secondary"
                      type="button"
                      onClick={() => {
                        if (targetDirty) void replan();
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
                      <fieldset className="format-options" disabled={state === "processing"}>
                        <legend>Allowed formats</legend>
                        {formatOptions.map((format) => (
                          <label key={format}>
                            <input
                              type="checkbox"
                              checked={target.formats.includes(format)}
                              onChange={(event) => editTarget((current) => ({
                                ...current,
                                formats: event.target.checked
                                  ? [...current.formats, format]
                                  : current.formats.filter((item) => item !== format),
                              }))}
                            />
                            {format === "jpeg" ? "JPEG" : format === "png" ? "PNG" : "WebP"}
                          </label>
                        ))}
                      </fieldset>
                      <label>Maximum bytes<input inputMode="numeric" value={target.maxBytes} disabled={state === "processing"} onChange={(event) => editTarget((current) => ({ ...current, maxBytes: event.target.value }))} placeholder="No limit" /></label>
                      <fieldset className="dimension-fields" disabled={state === "processing"}><legend>Width</legend><label>Exact<input aria-label="Exact width" inputMode="numeric" value={target.widthExact} onChange={(event) => editTarget((current) => ({ ...current, widthExact: event.target.value }))} placeholder="Any" /></label><label>Minimum<input aria-label="Minimum width" inputMode="numeric" value={target.widthMin} onChange={(event) => editTarget((current) => ({ ...current, widthMin: event.target.value }))} placeholder="None" /></label><label>Maximum<input aria-label="Maximum width" inputMode="numeric" value={target.widthMax} onChange={(event) => editTarget((current) => ({ ...current, widthMax: event.target.value }))} placeholder="None" /></label></fieldset>
                      <fieldset className="dimension-fields" disabled={state === "processing"}><legend>Height</legend><label>Exact<input aria-label="Exact height" inputMode="numeric" value={target.heightExact} onChange={(event) => editTarget((current) => ({ ...current, heightExact: event.target.value }))} placeholder="Any" /></label><label>Minimum<input aria-label="Minimum height" inputMode="numeric" value={target.heightMin} onChange={(event) => editTarget((current) => ({ ...current, heightMin: event.target.value }))} placeholder="None" /></label><label>Maximum<input aria-label="Maximum height" inputMode="numeric" value={target.heightMax} onChange={(event) => editTarget((current) => ({ ...current, heightMax: event.target.value }))} placeholder="None" /></label></fieldset>
                      {targetDirty ? <button className="secondary" type="button" onClick={() => void replan()} disabled={state === "processing"}>Review target changes</button> : null}
                    </div>
                  ) : null}
                </>
              ) : <p className="empty-copy">Paste the rejection or requirements and Fitifact will show the normalized target here.</p>}
            </section>

            <section className={`card status-card ${status.tone}`} aria-labelledby="status-title">
              <h2 id="status-title" className="status-title">{statusTitle}</h2>
              <div role="status" aria-live="polite" aria-atomic="true">
                {state === "processing" && progress ? <><p>{progress.stage}</p><progress max="100" value={progress.percent}>{progress.percent}%</progress><p className="privacy-reminder">Your image stays on this device.</p><button className="danger-link" type="button" onClick={cancel}>Cancel processing</button></> : null}
                {error ? <p className="error-copy">{error.message}</p> : null}
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
                ) : state === "processing" ? null : <p className="empty-copy">Paste what the form said. Fitifact will explain the minimum changes.</p>}
              </div>

              {plan?.plan.target.crop.required && state !== "adapted" ? (
                <div className="crop-editor" aria-labelledby="crop-title">
                  <h3 id="crop-title">Choose the crop</h3>
                  {sourceUrl && crop ? <div className="crop-stage"><img src={sourceUrl} alt={`Crop preview of ${sourceFile?.name ?? "selected image"}`} /><span className="crop-mask" aria-hidden="true" style={{ left: `${crop.x * 100}%`, top: `${crop.y * 100}%`, width: `${crop.width * 100}%`, height: `${crop.height * 100}%` }} /></div> : null}
                  <label htmlFor="crop-position">{cropAxis(plan.plan.source_width, plan.plan.source_height, plan.plan.target.width, plan.plan.target.height) === "horizontal" ? "Horizontal" : "Vertical"} crop position: {cropPosition}%</label>
                  <input id="crop-position" type="range" min="0" max="100" value={cropPosition} disabled={state === "processing"} onChange={(event) => setCropPosition(Number(event.target.value))} />
                  <label className="check-label" htmlFor="crop-consent"><input id="crop-consent" type="checkbox" checked={cropConsent} disabled={state === "processing"} onChange={(event) => setCropConsent(event.target.checked)} /> I approve removing the shaded edges to match {plan.plan.target.width} × {plan.plan.target.height}.</label>
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
                  I approve keeping only the first frame or page. Extra frames will be discarded.
                </label>
              ) : null}

              {(state === "planned" || state === "crop_approval_required") ? <button type="button" onClick={() => void adaptImage()} disabled={needsApproval}>Fix image</button> : null}

              {checklist.length ? <div className="checklist"><h3>Requirement checklist</h3><ul>{checklist.map((check) => <li key={check.constraint_id} className={check.result}><span aria-hidden="true">{check.result === "pass" ? "✓" : check.result === "fail" ? "×" : "?"}</span><span><strong>{check.field}</strong><br />{check.actual ?? "Unknown"} / needs {check.required}</span><span className="sr-result">{check.result}</span></li>)}</ul></div> : null}

              {(state === "adapted" || state === "compatible") ? <p className="validation-boundary">This output was validated against the requirements you confirmed. A destination may still have undocumented rules.</p> : null}

              {downloadUrl && download && (state === "adapted" || state === "compatible") ? <a className="download-button" href={downloadUrl} download={download.name}>{state === "compatible" && !outputBuffer ? "Use original image" : `Download ${download.extension.toUpperCase()}`}</a> : null}
            </section>
          </div>
        )}
      </main>

      {sidebarOpen ? (
        <div className="sidebar-overlay" onClick={() => setSidebarOpen(false)}>
          <aside
            id="app-sidebar"
            className="sidebar"
            ref={sidebarRef}
            role="dialog"
            aria-modal="true"
            aria-labelledby="sidebar-title"
            onClick={(event) => event.stopPropagation()}
          >
            <div className="sidebar-head">
              <h2 id="sidebar-title">Menu</h2>
              <button type="button" className="ghost" onClick={() => setSidebarOpen(false)}>Close</button>
            </div>

            <section>
              <h3>Saved targets</h3>
              {target && confirmedConstraintsJson ? (
                <div className="saved-row">
                  <input id="target-name" value={targetName} disabled={state === "processing"} onChange={(event) => setTargetName(event.target.value)} placeholder={summarizeTarget(target)} />
                  <button className="secondary" type="button" onClick={persistTarget} disabled={state === "processing"}>Save target</button>
                </div>
              ) : <p className="empty-copy">Confirm a target after dropping a file to save it on this device.</p>}
              {savedTargets.length ? (
                <ul className="saved-list">
                  {savedTargets.map((saved) => (
                    <li key={saved.id}>
                      <button type="button" className="saved-chip" disabled={state === "processing"} onClick={() => { void applySaved(saved); setSidebarOpen(false); }}>{saved.name}</button>
                      <button type="button" className="danger-link saved-delete" disabled={state === "processing"} onClick={() => { deleteSavedTarget(saved.id); setSavedTargets(listSavedTargets()); }}>Remove {saved.name}</button>
                    </li>
                  ))}
                </ul>
              ) : null}
              {confirmedConstraintsJson ? (
                <button
                  className="secondary"
                  type="button"
                  onClick={() => {
                    clearLastTarget();
                    setConfirmedConstraintsJson(null);
                    setTarget(null);
                    setRequirements("");
                    setParsed(null);
                    clearDerivedState(false);
                    if (inspection) setState("inspected");
                  }}
                >
                  Clear last-used target
                </button>
              ) : null}
            </section>

            <section>
              <h3>Settings</h3>
              <label className="check-label" htmlFor="suffix-setting">
                <input
                  id="suffix-setting"
                  type="checkbox"
                  checked={settings.fitifactSuffix}
                  onChange={(event) => updateSettings({ ...settings, fitifactSuffix: event.target.checked })}
                />
                Add .fitifact to download names
              </label>
              <p className="empty-copy">JPEG quality floor stays 50. Fitifact will not go lower.</p>
              <label className="check-label" htmlFor="frame-setting">
                <input
                  id="frame-setting"
                  type="checkbox"
                  checked={settings.firstFrameConsentDefault}
                  onChange={(event) => updateSettings({ ...settings, firstFrameConsentDefault: event.target.checked })}
                />
                Pre-check first-frame consent
              </label>
            </section>

            <section>
              <h3>About</h3>
              <p className="empty-copy">Your image stays on this device. Nothing is uploaded.</p>
              <p className="empty-copy">This is a video? The web app adapts images. The CLI remuxes and transcodes.</p>
              <p className="empty-copy">{__FITIFACT_HEIC_APPROVED__ ? <>HEIC phone photos decode locally; see the <a href={`${import.meta.env.BASE_URL}THIRD_PARTY_NOTICES.md`}>third-party notices</a>.</> : "HEIC decoder disabled in this build."}</p>
              <button className="secondary" type="button" onClick={() => void useSampleImage()} disabled={state === "processing"}>Try a sample image</button>
            </section>
          </aside>
        </div>
      ) : null}
    </div>
  );
}
