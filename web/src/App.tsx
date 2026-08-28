import { useEffect, useMemo, useRef, useState, type ClipboardEvent, type DragEvent, type KeyboardEvent } from "react";
import {
  BrandMark,
  ChatIcon,
  CheckFailIcon,
  CheckPassIcon,
  CheckUnknownIcon,
  CloseIcon,
  DownloadIcon,
  DropIcon,
  GitHubIcon,
  JpegIcon,
  MailIcon,
  MenuIcon,
  SlackIcon,
  VideoIcon,
  WhatsAppIcon,
  XIcon,
} from "./icons";
import { SetupSheet } from "./SetupSheet";
import { constraintSetFromEditable, editableTargetFromConstraints, formatBytes } from "./lib/constraints";
import { cropAxis, cropForAspect } from "./lib/crop";
import {
  constraintsLookLikeImage,
  constraintsLookLikeMedia,
  desktopTargetFromText,
  fileFromBytes,
  fileNameFromPath,
  isDesktop,
  isProfileId,
  type DesktopTarget,
} from "./lib/desktop";
import { publicDocsHref } from "./lib/docs-url";
import {
  FFMPEG_INSTALL_COPY,
  desktopAdapt,
  desktopDoctor,
  desktopInspect,
  desktopOpenDialog,
  desktopPlan,
  desktopReadHeader,
  desktopReadImage,
  inspectMediaLine,
  type DesktopAdaptResult,
  type DesktopArtifact,
  type DesktopPlanOutcome,
  type DoctorReport,
} from "./lib/desktop-engine";
import {
  destinationChips,
  familyForProfile,
  profileForFamily,
  resolveProfile,
  sameAsLastTimeCopy,
  usingDestinationCopy,
  type DestinationFamily,
  type FileKind,
} from "./lib/destinations";
import { errorCopy, mapErrorCode } from "./lib/error-copy";
import { checkLabel, describeActions, describeProblems, formatCheckValue, inspectLine, leftoverNote, understoodNote } from "./lib/explain";
import { deleteSavedTarget, listSavedTargets, saveTarget, type SavedTarget } from "./lib/saved-targets";
import {
  clearLastTarget,
  loadLastTarget,
  loadSettings,
  saveLastTarget,
  saveSettings,
  type AppSettings,
} from "./lib/settings";
import { declareSetup, loadSetup, setupIsNewer, type SetupState } from "./lib/setup";
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
import { classifyInput, isStillImage, refuseMessage } from "./worker/magic";
import { productStateForError } from "./worker/protocol";

const ACCEPT = ".tif,.tiff,.bmp,.gif,.webp,.heic,.heif,image/*";
const ACCEPT_DESKTOP = `${ACCEPT},.mp4,.mov,.m4v,video/mp4,video/quicktime`;
const PREVIEW_KINDS = new Set(["jpeg", "png", "webp", "gif", "bmp"]);
const CHIP_ICONS: Record<DestinationFamily, typeof ChatIcon> = {
  discord: ChatIcon,
  gmail: MailIcon,
  github: GitHubIcon,
  slack: SlackIcon,
  whatsapp: WhatsAppIcon,
  x: XIcon,
  jpeg: JpegIcon,
  "generic-video": VideoIcon,
} as const;

const STATE_COPY: Record<ProductState, { title: string; tone: string }> = {
  idle: { title: "Drop a file", tone: "neutral" },
  inspected: { title: "Choose a destination", tone: "neutral" },
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

function pasteBoxFromLast(requirements: string, profile: string | null): string {
  if (!requirements.trim()) return "";
  if (profile && (requirements === profile || isProfileId(requirements))) return "";
  if (!profile && isProfileId(requirements)) return "";
  return requirements;
}

function sessionFromLastTarget(): {
  requirements: string;
  target: EditableTarget | null;
  confirmed: string | null;
  profile: string | null;
} {
  const last = loadLastTarget();
  if (!last) return { requirements: "", target: null, confirmed: null, profile: null };
  const profile = last.profile && isProfileId(last.profile) ? last.profile : null;
  const json = last.constraintsJson?.trim() ?? "";
  if (json) {
    try {
      const parsed = JSON.parse(json) as ConstraintSet & { profile?: string };
      if (parsed.schema === "fitifact.constraints/v1" && Array.isArray(parsed.hard)) {
        const mediaOnly = constraintsLookLikeMedia(json) && !constraintsLookLikeImage(json);
        return {
          requirements: pasteBoxFromLast(last.requirements, profile),
          target: mediaOnly ? null : editableTargetFromConstraints(parsed),
          confirmed: JSON.stringify(parsed),
          profile,
        };
      }
      if (typeof parsed.profile === "string" && isProfileId(parsed.profile)) {
        return {
          requirements: pasteBoxFromLast(last.requirements, parsed.profile),
          target: null,
          confirmed: json,
          profile: parsed.profile,
        };
      }
    } catch {
      if (!profile) {
        clearLastTarget();
        return { requirements: "", target: null, confirmed: null, profile: null };
      }
    }
  }
  if (profile) {
    return {
      requirements: pasteBoxFromLast(last.requirements, profile),
      target: null,
      confirmed: json || JSON.stringify({ profile }),
      profile,
    };
  }
  clearLastTarget();
  return { requirements: "", target: null, confirmed: null, profile: null };
}

function editableFromSet(report: ConstraintSet): EditableTarget | null {
  const json = JSON.stringify(report);
  if (constraintsLookLikeMedia(json) && !constraintsLookLikeImage(json)) return null;
  try {
    return editableTargetFromConstraints(report);
  } catch {
    return null;
  }
}

function unsupportedFailure(message: string) {
  const report = {
    schema: "fitifact.error/v1" as const,
    code: "INSPECTION_UNSUPPORTED",
    message,
  };
  return new WorkerFailure(productStateForError(report), report);
}

function mediaPlanState(outcome: DesktopPlanOutcome): ProductState {
  if (outcome.kind === "compatible") return "compatible";
  if (outcome.kind === "cannot_satisfy") return "cannot_satisfy";
  return "planned";
}

function mediaStepCopy(outcome: DesktopPlanOutcome | null): string[] {
  return outcome?.plan?.steps.map((step) => {
    const reason = step.reasons?.[0]?.message;
    return reason ?? step.operation.replace(/_/g, " ");
  }) ?? [];
}

export function App() {
  const clientRef = useRef<ImageWorkerClient | null>(null);
  const operationRef = useRef(0);
  const parseGen = useRef(0);
  const parseTimer = useRef<number | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const sidebarRef = useRef<HTMLElement | null>(null);
  const desktop = isDesktop();
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
  const [mediaPath, setMediaPath] = useState<string | null>(null);
  const [mediaArtifact, setMediaArtifact] = useState<DesktopArtifact | null>(null);
  const [mediaOutcome, setMediaOutcome] = useState<DesktopPlanOutcome | null>(null);
  const [mediaAdapt, setMediaAdapt] = useState<DesktopAdaptResult | null>(null);
  const [desktopTarget, setDesktopTarget] = useState<DesktopTarget | null>(null);
  const [doctor, setDoctor] = useState<DoctorReport | null>(null);
  const [setup, setSetup] = useState<SetupState>(() => loadSetup());
  const [setupOpen, setSetupOpen] = useState(() => desktop && !loadSetup().completed);
  const [lastRejected, setLastRejected] = useState<{ name: string; message: string } | null>(null);
  const [activeProfile, setActiveProfile] = useState<string | null>(seed.profile);
  const [appliedFromLastUsed, setAppliedFromLastUsed] = useState(false);
  const confirmedRef = useRef<string | null>(null);
  const targetDirtyRef = useRef(false);
  const processingRef = useRef(false);
  const settingsRef = useRef(settings);
  const sourceFileRef = useRef<File | null>(null);
  const mediaPathRef = useRef<string | null>(null);
  const setupRef = useRef(setup);
  const activeProfileRef = useRef<string | null>(seed.profile);
  confirmedRef.current = confirmedConstraintsJson;
  targetDirtyRef.current = targetDirty;
  settingsRef.current = settings;
  mediaPathRef.current = mediaPath;
  setupRef.current = setup;
  activeProfileRef.current = activeProfile;

  useEffect(() => () => {
    client.dispose();
    if (parseTimer.current) window.clearTimeout(parseTimer.current);
  }, [client]);

  const inspectRef = useRef<(file: File, desktopPath?: string) => Promise<void>>(async () => undefined);

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

  useEffect(() => {
    if (!desktop) return;
    let disposed = false;
    let unlistenDrop: (() => void) | undefined;
    let unlistenProgress: (() => void) | undefined;
    void (async () => {
      const { getCurrentWebview } = await import("@tauri-apps/api/webview");
      const { listen } = await import("@tauri-apps/api/event");
      if (disposed) return;
      unlistenDrop = await getCurrentWebview().onDragDropEvent((event) => {
        if (event.payload.type === "enter" || event.payload.type === "over") {
          if (!processingRef.current) setDragging(true);
          return;
        }
        if (event.payload.type === "leave") {
          setDragging(false);
          return;
        }
        if (event.payload.type !== "drop") return;
        setDragging(false);
        const path = event.payload.paths[0];
        if (path) void openDesktopPathRef.current(path);
      });
      unlistenProgress = await listen<ProgressUpdate>("fitifact-progress", (event) => {
        setProgress(event.payload);
      });
      try {
        const report = await desktopDoctor();
        if (!disposed) setDoctor(report);
      } catch {
        if (!disposed) setDoctor(null);
      }
    })();
    return () => {
      disposed = true;
      unlistenDrop?.();
      unlistenProgress?.();
    };
  }, [desktop]);

  async function refreshDoctor() {
    if (!desktop) return;
    try {
      setDoctor(await desktopDoctor());
    } catch {
      setDoctor(null);
    }
  }

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
    const report = failure?.report ?? {
      schema: "fitifact.error/v1" as const,
      code: "EXECUTION_FAILED",
      message: caught instanceof Error ? caught.message : "Local processing failed.",
    };
    const mapped = mapErrorCode(report.code, report.message);
    setState(failure?.state && mapped === report.code ? failure.state : productStateForError({ code: mapped }));
    const message = errorCopy(mapped, report.message);
    setError({
      ...report,
      message: report.code === "PROVIDER_MISSING" ? `${message} ${FFMPEG_INSTALL_COPY}` : message,
    });
    setProgress(null);
  }

  function noteRejected(name: string, message: string) {
    setLastRejected({ name, message });
    if (sourceFileRef.current || mediaPathRef.current) return;
    handleFailure(unsupportedFailure(message), beginOperation());
  }

  function clearDerivedState(clearSource: boolean) {
    setPlan(null);
    setAdapted(null);
    setOutputBuffer(null);
    setPreviewBuffer(null);
    setCropConsent(false);
    setFirstFrameConsent(false);
    setProgress(null);
    setMediaOutcome(null);
    setMediaAdapt(null);
    if (clearSource) {
      setSourceFile(null);
      sourceFileRef.current = null;
      setInspection(null);
      setHeicPreviewMissing(false);
      setMediaPath(null);
      setMediaArtifact(null);
    }
  }

  function persistConfirmed(constraintsJson: string, text = requirements, profile?: string | null) {
    const nextProfile = profile === undefined ? activeProfileRef.current : profile;
    setConfirmedConstraintsJson(constraintsJson);
    setActiveProfile(nextProfile);
    activeProfileRef.current = nextProfile;
    saveLastTarget({
      requirements: text,
      constraintsJson,
      profile: nextProfile ?? undefined,
      savedAt: new Date().toISOString(),
    });
  }

  function commitSetup(next: SetupState, completed = next.completed) {
    const saved = declareSetup({ ...next, completed }, setupRef.current);
    setSetup(saved);
    setupRef.current = saved;
  }

  function currentKind(): FileKind | null {
    if (mediaPathRef.current || mediaArtifact) return "video";
    if (inspection || sourceFile) return "image";
    return null;
  }

  async function applyProfileId(id: string, existingOperation?: number, fromLastUsed = false) {
    parseGen.current += 1;
    setRequirements("");
    setParsed(null);
    setError(null);
    setTargetDirty(false);
    setEditingTarget(false);
    setAppliedFromLastUsed(fromLastUsed);
    if (mediaPathRef.current && !desktop) {
      const operation = existingOperation ?? beginOperation();
      handleFailure(unsupportedFailure(refuseMessage("video")), operation);
      return;
    }
    if (mediaPathRef.current && desktop) {
      const nativeTarget: DesktopTarget = { profile: id };
      setDesktopTarget(nativeTarget);
      setTarget(null);
      persistConfirmed(JSON.stringify(nativeTarget), "", id);
      await runDesktopPlan(nativeTarget, existingOperation);
      return;
    }
    try {
      const { report } = await client.compileProfile<ConstraintSet>(id);
      if (existingOperation !== undefined && existingOperation !== operationRef.current) return;
      const confirmed = JSON.stringify(report);
      setTarget(editableFromSet(report));
      setDesktopTarget({ profile: id, constraintsJson: confirmed });
      persistConfirmed(confirmed, "", id);
      if ((sourceFileRef.current || inspection) && !mediaPathRef.current) await runPlan(confirmed, existingOperation);
      else setState("requirements_ready");
    } catch (caught) {
      handleFailure(caught, existingOperation ?? operationRef.current);
    }
  }

  async function maybeApplyAfterInspect(kind: FileKind, operation: number) {
    if (targetDirtyRef.current) {
      setState("inspected");
      return;
    }
    const last = loadLastTarget();
    const currentSetup = setupRef.current;
    const setupProfile = resolveProfile(currentSetup, kind);
    if (kind === "video" && desktop && doctor && !doctor.healthy) {
      setState("inspected");
      return;
    }
    const setupWins = currentSetup.completed && setupIsNewer(currentSetup, last?.savedAt);
    if (setupWins && setupProfile) {
      await applyProfileId(setupProfile, operation, true);
      return;
    }
    const profile = activeProfileRef.current;
    if (profile) {
      await applyProfileId(profile, operation, true);
      return;
    }
    const confirmed = confirmedRef.current;
    if (confirmed?.includes("fitifact.constraints/v1")) {
      await runPlan(confirmed, operation);
      return;
    }
    if (currentSetup.completed && setupProfile) {
      await applyProfileId(setupProfile, operation, true);
      return;
    }
    setState("inspected");
  }

  async function applyDestination(family: DestinationFamily) {
    if (state === "processing") return;
    const kind = currentKind();
    if (!kind) return;
    if (kind === "video" && !desktop) {
      handleFailure(unsupportedFailure(refuseMessage("video")), beginOperation());
      return;
    }
    if (kind === "video" && desktop && doctor && !doctor.healthy) {
      setError({
        schema: "fitifact.error/v1",
        code: "PROVIDER_MISSING",
        message: FFMPEG_INSTALL_COPY,
      });
      setState("error");
      return;
    }
    const id = profileForFamily(family, setup.discordCap, kind);
    if (!id) {
      setError({
        schema: "fitifact.error/v1",
        code: "INPUT_INVALID",
        message: kind === "video"
          ? "That destination is for still images."
          : "That destination is for video. The web app adapts images. Use the desktop app or CLI after ffmpeg is on PATH.",
      });
      setState("inspected");
      return;
    }
    await applyProfileId(id);
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
    setDesktopTarget(null);
    setActiveProfile(null);
    activeProfileRef.current = null;
    setTargetDirty(false);
    setEditingTarget(false);
    setAppliedFromLastUsed(false);
    clearDerivedState(false);
    setError(null);
    if (!value.trim()) {
      setState(inspection || mediaArtifact ? "inspected" : "idle");
      return;
    }
    setState(inspection || mediaArtifact ? "inspected" : "idle");
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
    const nativeTarget = desktopTargetFromText(text);
    if (nativeTarget?.profile) {
      if (gen !== parseGen.current) return;
      await applyProfileId(nativeTarget.profile);
      return;
    }
    if (nativeTarget && desktop && mediaPathRef.current) {
      setDesktopTarget(nativeTarget);
      setParsed(null);
      setTarget(null);
      setTargetDirty(false);
      setEditingTarget(false);
      setError(null);
      persistConfirmed(JSON.stringify(nativeTarget), text, null);
      if (gen !== parseGen.current) return;
      await runDesktopPlan(nativeTarget);
      return;
    }
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
        setState(inspection || mediaArtifact ? "inspected" : "error");
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
        setState(inspection || mediaArtifact ? "inspected" : "error");
        return;
      }
      const editable = editableTargetFromConstraints(report.constraints);
      const confirmed = JSON.stringify(report.constraints);
      setTarget(editable);
      persistConfirmed(confirmed, text, null);
      setTargetDirty(false);
      setDesktopTarget({ constraintsJson: confirmed });
      setError(null);
      if (mediaPathRef.current) {
        setState("inspected");
        setError({
          schema: "fitifact.error/v1",
          code: "INPUT_INVALID",
          message: "This is video. Paste a profile id such as discord/video-upload, or choose a destination below.",
        });
        return;
      }
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

  async function inspectDesktopMedia(path: string, operation: number) {
    processingRef.current = true;
    setState("processing");
    setMediaPath(path);
    setMediaArtifact(null);
    setMediaOutcome(null);
    setMediaAdapt(null);
    setInspection(null);
    setPlan(null);
    setAdapted(null);
    setOutputBuffer(null);
    setPreviewBuffer(null);
    try {
      const artifact = await desktopInspect(path);
      if (operation !== operationRef.current) return;
      setMediaArtifact(artifact);
      const nativeTarget = desktopTarget ?? desktopTargetFromText(requirements);
      if (nativeTarget && !constraintsLookLikeImage(confirmedRef.current)) {
        setDesktopTarget(nativeTarget);
        await runDesktopPlan(nativeTarget, operation);
      } else {
        await maybeApplyAfterInspect("video", operation);
      }
    } catch (caught) {
      handleFailure(caught, operation);
    } finally {
      if (operation === operationRef.current) {
        setProgress(null);
        processingRef.current = false;
      }
    }
  }

  async function openDesktopPath(path: string) {
    if (state === "processing") return;
    const header = await desktopReadHeader(path);
    const kind = classifyInput(header);
    if (kind === "video") {
      const operation = beginOperation();
      const placeholder = new File([], fileNameFromPath(path));
      setSourceFile(placeholder);
      sourceFileRef.current = placeholder;
      setError(null);
      await inspectDesktopMedia(path, operation);
      return;
    }
    if (!isStillImage(kind)) {
      noteRejected(fileNameFromPath(path), refuseMessage(kind));
      return;
    }
    const bytes = await desktopReadImage(path);
    const type = kind === "jpeg" ? "image/jpeg" : kind === "png" ? "image/png" : kind === "webp" ? "image/webp" : "";
    await inspectFile(fileFromBytes(bytes, fileNameFromPath(path), type || undefined));
  }

  const openDesktopPathRef = useRef(openDesktopPath);
  openDesktopPathRef.current = openDesktopPath;

  async function inspectFile(file: File, desktopPath?: string) {
    if (state === "processing") return;
    const header = new Uint8Array(await file.slice(0, Math.min(file.size, 65_536)).arrayBuffer());
    const kind = classifyInput(header);
    if (!isStillImage(kind)) {
      if (desktop && kind === "video") {
        if (desktopPath) {
          await inspectDesktopMedia(desktopPath, beginOperation());
          return;
        }
        noteRejected(file.name, "Drop or choose the file so Fitifact can write next to the original.");
        return;
      }
      noteRejected(file.name, refuseMessage(kind));
      return;
    }
    setLastRejected(null);
    const operation = beginOperation();
    setSourceFile(file);
    sourceFileRef.current = file;
    setInspection(null);
    setPlan(null);
    setAdapted(null);
    setOutputBuffer(null);
    setPreviewBuffer(null);
    setHeicPreviewMissing(false);
    setMediaPath(null);
    setMediaArtifact(null);
    setMediaOutcome(null);
    setMediaAdapt(null);
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
      await maybeApplyAfterInspect("image", operation);
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

  async function runDesktopPlan(targetSpec: DesktopTarget, existingOperation?: number) {
    const path = mediaPathRef.current;
    if (!path) return;
    const operation = existingOperation ?? beginOperation();
    processingRef.current = true;
    setState("processing");
    setError(null);
    setMediaOutcome(null);
    setMediaAdapt(null);
    try {
      const outcome = await desktopPlan(path, targetSpec);
      if (operation !== operationRef.current) return;
      setMediaOutcome(outcome);
      persistConfirmed(JSON.stringify(targetSpec), requirements, targetSpec.profile ?? activeProfileRef.current);
      setState(mediaPlanState(outcome));
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
    if (mediaPathRef.current) {
      const spec = desktopTarget ?? { constraintsJson };
      await runDesktopPlan(spec, existingOperation);
      return;
    }
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
      persistConfirmed(constraintsSnapshot ?? draft, requirements, null);
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
    if (mediaPathRef.current && desktopTarget) {
      if (state === "processing") return;
      const operation = beginOperation();
      processingRef.current = true;
      setState("processing");
      setError(null);
      try {
        const result = await desktopAdapt(mediaPathRef.current, desktopTarget);
        if (operation !== operationRef.current) return;
        setMediaAdapt(result);
        if (result.status === "compatible") setState("compatible");
        else if (result.status === "adapted") setState("adapted");
        else if (result.status === "cannot_satisfy") setState("cannot_satisfy");
        else handleFailure(new WorkerFailure("validation_failure", {
          schema: "fitifact.error/v1",
          code: result.error && "code" in result.error ? String(result.error.code) : "VALIDATION_FAILED",
          message: result.error && "message" in result.error ? String(result.error.message) : "Adaptation failed.",
        }), operation);
      } catch (caught) {
        handleFailure(caught, operation);
      } finally {
        if (operation === operationRef.current) {
          setProgress(null);
          processingRef.current = false;
        }
      }
      return;
    }
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
    if (desktop || state === "processing") return;
    const file = event.dataTransfer.files.item(0);
    if (file) void inspectFile(file);
  }

  function onDropKey(event: KeyboardEvent<HTMLDivElement>) {
    if (event.key !== "Enter" && event.key !== " ") return;
    event.preventDefault();
    if (desktop) {
      void chooseDesktopFile();
      return;
    }
    fileInputRef.current?.click();
  }

  async function chooseDesktopFile() {
    if (state === "processing") return;
    const path = await desktopOpenDialog();
    if (path) await openDesktopPath(path);
  }

  function persistTarget() {
    const json = confirmedConstraintsJson ?? (desktopTarget ? JSON.stringify(desktopTarget) : null);
    if (!json) return;
    try {
      const saved = saveTarget({
        name: targetName || (target ? summarizeTarget(target) : desktopTarget?.profile) || "Saved target",
        requirements,
        constraintsJson: json,
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
    const nativeTarget = desktopTargetFromText(saved.requirements);
    if (nativeTarget?.profile) {
      await applyProfileId(nativeTarget.profile);
      return;
    }
    if (nativeTarget && mediaPathRef.current) {
      setDesktopTarget(nativeTarget);
      persistConfirmed(saved.constraintsJson, saved.requirements, null);
      await runDesktopPlan(nativeTarget);
      return;
    }
    try {
      const parsed = JSON.parse(saved.constraintsJson) as { profile?: string };
      if (typeof parsed.profile === "string" && isProfileId(parsed.profile)) {
        await applyProfileId(parsed.profile);
        return;
      }
      const { report } = await client.compileConstraints<ConstraintSet>(saved.constraintsJson);
      setTarget(editableFromSet(report) ?? editableTargetFromConstraints(report));
      const confirmed = JSON.stringify(report);
      persistConfirmed(confirmed, saved.requirements, null);
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

  const checklist = mediaAdapt?.report?.checks ?? adapted?.report.checks ?? plan?.report.checks ?? [];
  const status = STATE_COPY[state];
  const statusTitle = state === "crop_approval_required"
    ? approvalTitle(plan)
    : state === "adapted" && mediaArtifact
      ? "File adapted and validated"
      : status.title;
  const problems = plan ? describeProblems(plan) : mediaOutcome?.blocking?.map((item) => item.message) ?? [];
  const actions = plan ? describeActions(plan) : mediaStepCopy(mediaOutcome);
  const leftover = leftoverNote(parsed?.unresolved.map((item) => item.text) ?? []);
  const understood = understoodNote(parsed);
  const inspectFacts = mediaArtifact
    ? inspectMediaLine(mediaArtifact, formatBytes(mediaArtifact.byte_length))
    : inspection
      ? inspectLine(
          inspection.kind,
          inspection.artifact.image?.width,
          inspection.artifact.image?.height,
          inspection.artifact.byte_length,
        )
      : null;
  const showWork = Boolean(inspection || mediaArtifact);
  const needsApproval = Boolean(
    (plan?.plan.target.crop.required && !cropConsent) ||
      (plan?.plan.target.first_frame?.required && !firstFrameConsent),
  );
  const formatOptions: OutputFormat[] = ["jpeg", "png", "webp"];
  const chooseLabel = desktop ? "Choose a file" : "Choose an image";
  const accept = desktop ? ACCEPT_DESKTOP : ACCEPT;
  const doctorUnhealthy = Boolean(desktop && doctor && !doctor.healthy);
  const chips = destinationChips(setup.discordCap, { includeVideo: desktop });
  const activeFamily = activeProfile ? familyForProfile(activeProfile) : null;
  const usingCopy = activeFamily
    ? appliedFromLastUsed
      ? sameAsLastTimeCopy(activeFamily, setup.discordCap)
      : usingDestinationCopy(activeFamily, setup.discordCap)
    : null;
  const canSaveTarget = Boolean(confirmedConstraintsJson || desktopTarget?.profile);

  function focusPasteOverride() {
    const area = document.getElementById("requirements");
    area?.scrollIntoView({ block: "center" });
    if (area instanceof HTMLTextAreaElement) area.focus();
  }

  return (
    <div className={`app-shell ${showWork ? "has-file" : "is-idle"}`}>
      <header className="site-header">
        <a className="wordmark" href="#top" aria-label="Fitifact home">
          <BrandMark className="brand-mark" />
          <span>Fitifact</span>
        </a>
        <button
          type="button"
          className="ghost menu-button"
          aria-label="Menu"
          aria-expanded={sidebarOpen}
          aria-controls="app-sidebar"
          onClick={() => setSidebarOpen((open) => !open)}
        >
          <span className="menu-icons">
            <MenuIcon className={`menu-icon${sidebarOpen ? " is-hidden" : ""}`} />
            <CloseIcon className={`menu-icon${sidebarOpen ? "" : " is-hidden"}`} />
          </span>
          Menu
        </button>
      </header>

      <main id="top">
        {!showWork ? (
          <div className="drop-canvas">
            <h1 className="drop-headline">Make your image pass the upload.</h1>
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
              <DropIcon className="drop-glyph" />
              <p>Drop a file</p>
              <p id="image-help">{desktop ? "JPEG PNG WebP HEIC TIFF BMP GIF · MP4 MOV" : "JPEG PNG WebP HEIC · TIFF BMP GIF · max 32 MiB or 24 megapixels"}</p>
              <label
                className="button-label"
                htmlFor={desktop ? undefined : "image-file"}
                onClick={desktop ? (event) => { event.preventDefault(); void chooseDesktopFile(); } : undefined}
              >
                {chooseLabel}
              </label>
              <input
                id="image-file"
                ref={fileInputRef}
                className="visually-hidden"
                type="file"
                accept={accept}
                aria-describedby="image-help"
                onChange={(event) => {
                  const file = event.currentTarget.files?.item(0);
                  if (file) void inspectFile(file);
                  event.currentTarget.value = "";
                }}
                disabled={state === "processing"}
              />
            </div>
            <button
              className="ghost drop-sample"
              type="button"
              onClick={() => void useSampleImage()}
              disabled={state === "processing"}
            >
              Try a sample image
            </button>
            <div className={`idle-status ${status.tone}`} role="status" aria-live="polite" aria-atomic="true">
              {state !== "idle" ? <h2 className="status-title">{statusTitle}</h2> : null}
              {doctorUnhealthy ? <p className="notice">{FFMPEG_INSTALL_COPY}</p> : null}
              {state === "processing" && progress ? (
                <>
                  <p>{progress.stage}</p>
                  <progress max="100" value={progress.percent}>{progress.percent}%</progress>
                  <p className="privacy-reminder">Your image stays on this device.</p>
                  <button className="danger-link" type="button" onClick={cancel}>Cancel processing</button>
                </>
              ) : null}
              {lastRejected ? (
                <div className="reject-card" role="status">
                  <strong>{lastRejected.name}</strong>
                  <p className="error-copy">{lastRejected.message}</p>
                </div>
              ) : error ? <p className="error-copy">{error.message}</p> : null}
            </div>
          </div>
        ) : (
          <div className="work-surface">
            <section className="card file-card">
              <div className="file-card-main">
                {sourceUrl ? (
                  <img
                    className="file-thumb"
                    src={sourceUrl}
                    alt={`Preview of ${sourceFile?.name ?? "selected image"}`}
                  />
                ) : null}
                <div className="file-chip file-row">
                  <strong>{sourceFile?.name ?? (mediaPath ? fileNameFromPath(mediaPath) : "")}</strong>
                  <span className="numeric">{inspectFacts ?? formatBytes(sourceFile?.size ?? 0)}</span>
                  {heicPreviewMissing ? <span>Preview unavailable for this phone photo.</span> : null}
                </div>
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
                <label
                  className="button-label"
                  htmlFor={desktop ? undefined : "image-file-replace"}
                  onClick={desktop ? (event) => { event.preventDefault(); void chooseDesktopFile(); } : undefined}
                >
                  {chooseLabel}
                </label>
                <input
                  id="image-file-replace"
                  ref={fileInputRef}
                  className="visually-hidden"
                  type="file"
                  accept={accept}
                  onChange={(event) => {
                    const file = event.currentTarget.files?.item(0);
                    if (file) void inspectFile(file);
                    event.currentTarget.value = "";
                  }}
                  disabled={state === "processing"}
                />
              </div>
              {lastRejected ? (
                <p className="error-copy file-reject" role="status">
                  {lastRejected.name}: {lastRejected.message}
                </p>
              ) : null}
            </section>

            <section className="card destination-card" aria-labelledby="destination-title">
              <h2 id="destination-title">Where does it need to work?</h2>
              <div className="destination-chips">
                {chips.map((chip) => {
                  const Icon = CHIP_ICONS[chip.family];
                  const selected = familyForProfile(activeProfile ?? "") === chip.family;
                  const videoLocked = chip.videoOnly && doctorUnhealthy;
                  return (
                    <button
                      key={chip.family}
                      type="button"
                      className={`destination-chip${selected ? " is-selected" : ""}`}
                      disabled={state === "processing" || videoLocked}
                      aria-pressed={selected}
                      onClick={() => void applyDestination(chip.family)}
                    >
                      <span className="destination-chip-label"><Icon />{chip.label}</span>
                      <span className="destination-chip-sub">{videoLocked ? "Needs ffmpeg on PATH" : chip.subtitle}</span>
                    </button>
                  );
                })}
              </div>
              {usingCopy ? <p className="empty-copy">{usingCopy}</p> : null}
              <h2 id="requirements-title">The form said something else</h2>
              <label htmlFor="requirements">Rejection message or requirements</label>
              <textarea
                id="requirements"
                rows={4}
                value={requirements}
                placeholder="Or paste the exact rejection (size, format, dimensions)."
                onPaste={onRequirementsPaste}
                onChange={(event) => editRequirements(event.target.value)}
                disabled={state === "processing"}
              />
              {understood ? <div className="notice" role="note">{understood}</div> : null}
              {leftover ? <p className="leftover-copy">{leftover}</p> : null}

              <h2 id="target-title">I understood this as</h2>
              {mediaArtifact && desktopTarget ? (
                <p className="target-summary">
                  {desktopTarget.profile ? `--for ${desktopTarget.profile}` : "Pasted destination constraints"}
                </p>
              ) : target ? (
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
                      {editingTarget ? "Hide" : "Edit"}
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
              ) : <p className="empty-copy">{mediaArtifact ? "Paste a profile id such as discord/video-upload, YAML constraints, or --for generic/video-upload." : "Paste the rejection if a chip isn't right."}</p>}
            </section>

            <section className={`card status-card ${status.tone}`} aria-labelledby="status-title">
              <h2 id="status-title" className="status-title">{statusTitle}</h2>
              <div role="status" aria-live="polite" aria-atomic="true">
                {doctorUnhealthy ? <p className="notice">{FFMPEG_INSTALL_COPY}</p> : null}
                {state === "processing" && progress ? <><p>{progress.stage}</p><progress max="100" value={progress.percent}>{progress.percent}%</progress><p className="privacy-reminder">Your image stays on this device.</p><button className="danger-link" type="button" onClick={cancel}>Cancel processing</button></> : null}
                {error ? <p className="error-copy">{error.message}</p> : null}
                {inspectFacts && !plan && !mediaOutcome ? <p className="empty-copy numeric">{inspectFacts}</p> : null}
                {plan || mediaOutcome ? (
                  <div className="plan-summary">
                    <p><strong>Your file:</strong> {inspectFacts ?? `${plan?.inspection.image?.format?.toUpperCase()} · ${plan?.inspection.image?.width} × ${plan?.inspection.image?.height} · ${formatBytes(plan?.inspection.byte_length ?? 0)}`}</p>
                    {(plan?.report.compatible && plan.plan.noop) || mediaOutcome?.kind === "compatible" ? <p>This file already fits. Nothing needs to change.</p> : (
                      <>
                        {problems.length ? <><p><strong>{problems.length} problem{problems.length === 1 ? "" : "s"} found</strong></p><ul>{problems.map((item) => <li key={item}>{item}</li>)}</ul></> : null}
                        {actions.length ? <><p><strong>What I’ll do</strong></p><ul>{actions.map((item) => <li key={item}>{item}</li>)}</ul></> : null}
                      </>
                    )}
                  </div>
                ) : state === "processing" ? null : (
                  <div className="inspect-status">
                    {sourceUrl ? (
                      <img
                        className="status-preview"
                        src={sourceUrl}
                        alt={`Preview of ${sourceFile?.name ?? "selected image"}`}
                      />
                    ) : null}
                    {inspectFacts ? <p className="numeric">{inspectFacts}</p> : null}
                    <p className="empty-copy">Tap a destination. Fitifact will explain the minimum changes.</p>
                  </div>
                )}
              </div>

              {plan?.plan.target.crop.required && state !== "adapted" ? (
                <div className="crop-editor" aria-labelledby="crop-title">
                  <h3 id="crop-title">Choose the crop</h3>
                  {sourceUrl && crop ? <div className="crop-stage"><img src={sourceUrl} alt={`Crop preview of ${sourceFile?.name ?? "selected image"}`} /><span className="crop-mask" aria-hidden="true" style={{ left: `${crop.x * 100}%`, top: `${crop.y * 100}%`, width: `${crop.width * 100}%`, height: `${crop.height * 100}%` }} /></div> : null}
                  <label htmlFor="crop-position">{cropAxis(plan.plan.source_width, plan.plan.source_height, plan.plan.target.width, plan.plan.target.height) === "horizontal" ? "Horizontal" : "Vertical"} crop position: <span className="numeric">{cropPosition}%</span></label>
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

              {showWork && state !== "adapted" && state !== "compatible" && state !== "processing" ? (
                <button
                  type="button"
                  onClick={() => void adaptImage()}
                  disabled={(state !== "planned" && state !== "crop_approval_required") || needsApproval}
                >
                  {mediaArtifact ? "Adapt file" : "Fix image"}
                </button>
              ) : null}

              {checklist.length ? <div className="checklist"><h3>Requirement checklist</h3><ul>{checklist.map((check) => <li key={check.constraint_id} className={check.result}><span className="check-mark" aria-hidden="true">{check.result === "pass" ? <CheckPassIcon /> : check.result === "fail" ? <CheckFailIcon /> : <CheckUnknownIcon />}</span><span><strong>{checkLabel(check.field)}</strong><br />{formatCheckValue(check.field, check.actual)} / needs {formatCheckValue(check.field, check.required)}</span><span className="sr-result">{check.result}</span></li>)}</ul></div> : null}

              {(state === "adapted" || state === "compatible") ? <p className="validation-boundary">This output was validated against the requirements you confirmed. A destination may still have undocumented rules.</p> : null}

              {(state === "adapted" || state === "compatible") && (sourceUrl || downloadUrl) && !mediaArtifact ? (
                <div className="before-after">
                  {sourceUrl ? (
                    <figure>
                      <img src={sourceUrl} alt={`Before: ${sourceFile?.name ?? "original"}`} />
                      <figcaption>Before</figcaption>
                    </figure>
                  ) : null}
                  {downloadUrl ? (
                    <figure>
                      <img src={downloadUrl} alt={`After: ${download?.name ?? "adapted image"}`} />
                      <figcaption>After</figcaption>
                    </figure>
                  ) : null}
                </div>
              ) : null}

              {mediaAdapt?.output && (state === "adapted" || state === "compatible") ? (
                <p className="empty-copy numeric">Saved next to the original as {fileNameFromPath(mediaAdapt.output)}</p>
              ) : null}

              {downloadUrl && download && (state === "adapted" || state === "compatible") && !mediaArtifact ? (
                <a className="download-button" href={downloadUrl} download={download.name}>
                  <DownloadIcon />
                  {state === "compatible" && !outputBuffer ? "Use original image" : `Download ${download.extension.toUpperCase()}`}
                </a>
              ) : null}

              {(state === "adapted" || state === "compatible") ? (
                <button className="danger-link" type="button" onClick={focusPasteOverride}>
                  Destination still said no? Paste the new message.
                </button>
              ) : null}
            </section>
          </div>
        )}
      </main>

      <footer className="site-footer">
        <a href="https://github.com/mattbaconz/fitifact">
          <GitHubIcon />
          GitHub
        </a>
        <p>Local · nothing is uploaded</p>
        <p className="numeric">v0.1.0-rc.6</p>
      </footer>

      {setupOpen ? (
        <div className="setup-overlay" role="dialog" aria-modal="true" aria-labelledby="setup-title">
          <SetupSheet
            setup={setup}
            doctor={doctor}
            doctorCopy={FFMPEG_INSTALL_COPY}
            confirmLabel="Use these destinations"
            onChange={setSetup}
            onConfirm={() => {
              commitSetup(setup, true);
              setSetupOpen(false);
            }}
            onDismiss={setup.completed ? () => setSetupOpen(false) : undefined}
            onRecheck={refreshDoctor}
          />
        </div>
      ) : null}

      <div
        className={`sidebar-overlay${sidebarOpen ? " is-open" : ""}`}
        onClick={() => setSidebarOpen(false)}
        aria-hidden={!sidebarOpen}
        inert={!sidebarOpen}
      >
        <aside
          id="app-sidebar"
          className="sidebar"
          ref={sidebarRef}
          role={sidebarOpen ? "dialog" : undefined}
          aria-modal={sidebarOpen ? "true" : undefined}
          aria-labelledby="sidebar-title"
          onClick={(event) => event.stopPropagation()}
        >
          <div className="sidebar-head">
            <h2 id="sidebar-title">Menu</h2>
            <button type="button" className="ghost" onClick={() => setSidebarOpen(false)}>Close</button>
          </div>

          <section>
            <h3>Saved targets</h3>
            {canSaveTarget ? (
              <div className="saved-row">
                <input id="target-name" value={targetName} disabled={state === "processing"} onChange={(event) => setTargetName(event.target.value)} placeholder={target ? summarizeTarget(target) : desktopTarget?.profile ?? "Name this target"} />
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
                  setDesktopTarget(null);
                  setActiveProfile(null);
                  activeProfileRef.current = null;
                  setRequirements("");
                  setParsed(null);
                  clearDerivedState(false);
                  if (inspection || mediaArtifact) setState("inspected");
                }}
              >
                Clear last-used target
              </button>
            ) : null}
          </section>

          <section>
            <h3>{desktop ? "Setup" : "Destinations"}</h3>
            {desktop ? (
              <>
                <p className="empty-copy">Doctor, destinations, and the Discord cap live on one sheet. Fitifact cannot see Nitro.</p>
                <button
                  className="secondary"
                  type="button"
                  onClick={() => {
                    setSetupOpen(true);
                    setSidebarOpen(false);
                  }}
                >
                  Setup
                </button>
              </>
            ) : (
              <SetupSheet
                setup={setup}
                doctor={null}
                doctorCopy=""
                showDoctor={false}
                includeVideo={false}
                confirmLabel="Save destinations"
                onChange={setSetup}
                onConfirm={() => commitSetup(setup, true)}
              />
            )}
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
            <p className="empty-copy"><a href={publicDocsHref()}>Docs</a> covering Pages, desktop, destinations, and the CLI.</p>
            <p className="empty-copy">{desktop ? "Images adapt in-process. MP4 and MOV use system FFmpeg. WebM, MKV, PDF, and SVG are refused." : "This is a video? The web app adapts images. Use the desktop app or CLI after ffmpeg is on PATH."}</p>
            {doctorUnhealthy ? <p className="notice">{FFMPEG_INSTALL_COPY}</p> : null}
            <p className="empty-copy">{__FITIFACT_HEIC_APPROVED__ ? <>HEIC phone photos decode locally; see the <a href={`${import.meta.env.BASE_URL}THIRD_PARTY_NOTICES.md`}>third-party notices</a>.</> : "HEIC decoder disabled in this build."}</p>
            <button className="secondary" type="button" onClick={() => void useSampleImage()} disabled={state === "processing"}>Try a sample image</button>
          </section>
        </aside>
      </div>
    </div>
  );
}
