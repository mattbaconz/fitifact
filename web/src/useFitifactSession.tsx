import {
  useEffect,
  useMemo,
  useRef,
  useState,
  type ClipboardEvent,
  type DragEvent,
  type KeyboardEvent,
} from "react";
import { type SampleScenario } from "./components/ProductIntro";
import { readStored, writeStored } from "./lib/storage";
import {
  ChatIcon,
  GitHubIcon,
  JpegIcon,
  MailIcon,
  SlackIcon,
  VideoIcon,
  WhatsAppIcon,
  XIcon,
} from "./icons";
import {
  constraintSetFromEditable,
  editableTargetFromConstraints,
  formatBytes,
} from "./lib/constraints";
import { cropForAspect } from "./lib/crop";
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
import {
  desktopAdapt,
  desktopCompileTarget,
  desktopCancel,
  desktopCheck,
  desktopDoctor,
  desktopInspect,
  desktopOpenDialog,
  desktopPlan,
  desktopReadHeader,
  desktopReadImage,
  desktopReveal,
  fileKindFromArtifact,
  inspectMediaLine,
  type DesktopAdaptResult,
  type DesktopArtifact,
  type DesktopPlanOutcome,
  type DoctorReport,
  type MediaPlanReport,
} from "./lib/desktop-engine";
import {
  destinationChips,
  familyForProfile,
  familyFromQuery,
  profileForFamily,
  resolveProfile,
  sameAsLastTimeCopy,
  usingDestinationCopy,
  type DestinationFamily,
  type FileKind,
} from "./lib/destinations";
import { desktopGuideHref } from "./lib/docs-url";
import { errorCopy, mapErrorCode } from "./lib/error-copy";
import {
  describeActions,
  describeKept,
  describeMediaActions,
  describeProblems,
  describeProblemsFromChecks,
  describeVerified,
  inspectLine,
  leftoverNote,
  measuredRows,
  understoodNote,
} from "./lib/explain";
import { isStale, registryProfile } from "./lib/registry";
import { type SavedTarget } from "./lib/saved-targets";
import {
  clearLastTarget,
  loadLastTarget,
  loadSettings,
  saveLastTarget,
  saveSettings,
  type AppSettings,
} from "./lib/settings";
import {
  declareSetup,
  loadSetup,
  setupIsNewer,
  type SetupState,
} from "./lib/setup";
import type {
  AdaptReport,
  CompatibilityReport,
  ConstraintSet,
  EditableTarget,
  ErrorReport,
  InspectReport,
  OutputFormat,
  PlanReport,
  ProductState,
  RequirementParse,
} from "./types";
import {
  ImageWorkerClient,
  WorkerFailure,
  type ProgressUpdate,
} from "./worker/client";
import {
  classifyInput,
  isDesktopMedia,
  isStillImage,
  refuseMessage,
} from "./worker/magic";
import { productStateForError } from "./worker/protocol";

const ACCEPT_IMAGES = ".tif,.tiff,.bmp,.gif,.webp,.heic,.heif,image/*";
const ACCEPT = `${ACCEPT_IMAGES},.mp4,.mov,.m4v,.webm,.mkv,video/mp4,video/quicktime,video/webm,video/x-matroska`;
const ACCEPT_DESKTOP = `${ACCEPT},.mp3,.wav,.flac,.m4a,.ogg,.aac,audio/mpeg,audio/wav,audio/mp4`;
const NEEDS_DESKTOP_ENCODE =
  "This video needs re-encoding, which the web app can't do yet. Open it in the Fitifact desktop app.";
const PREVIEW_KINDS = new Set(["jpeg", "png", "webp", "gif", "bmp"]);
export const CHIP_ICONS: Record<DestinationFamily, typeof ChatIcon> = {
  discord: ChatIcon,
  gmail: MailIcon,
  github: GitHubIcon,
  slack: SlackIcon,
  whatsapp: WhatsAppIcon,
  x: XIcon,
  jpeg: JpegIcon,
  "generic-video": VideoIcon,
} as const;

export type Tone = "neutral" | "info" | "success" | "warning" | "danger" | "refusal";

const STATE_COPY: Record<ProductState, { title: string; tone: Tone }> = {
  idle: { title: "Drop a file", tone: "neutral" },
  inspected: { title: "Choose a destination", tone: "neutral" },
  requirements_ready: { title: "Ready", tone: "neutral" },
  processing: { title: "Working locally", tone: "neutral" },
  planned: { title: "Minimum changes ready", tone: "neutral" },
  compatible: {
    title: "Already compatible. Nothing changed.",
    tone: "success",
  },
  crop_approval_required: { title: "Crop approval required", tone: "info" },
  adapted: { title: "Adapted and verified", tone: "success" },
  cannot_satisfy: {
    title: "Cannot satisfy these requirements",
    tone: "refusal",
  },
  validation_failure: { title: "Validation failed", tone: "danger" },
  resource_limit: { title: "Resource limit reached", tone: "danger" },
  cancelled: { title: "Processing cancelled", tone: "neutral" },
  unsupported_heic: {
    title: "This is a phone photo this build cannot decode yet",
    tone: "warning",
  },
  error: { title: "Could not process this file", tone: "danger" },
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

function outputDetails(
  format: OutputFormat | "mp4",
  originalName: string,
  suffix: boolean,
) {
  const extension = format === "jpeg" ? "jpg" : format;
  const mime =
    format === "jpeg"
      ? "image/jpeg"
      : format === "png"
        ? "image/png"
        : format === "mp4"
          ? "video/mp4"
          : "image/webp";
  const stem =
    originalName.replace(/\.[^.]*$/, "").replace(/[^a-zA-Z0-9._-]+/g, "-") ||
    (format === "mp4" ? "video" : "image");
  return {
    extension,
    mime,
    name: suffix ? `${stem}.fitifact.${extension}` : `${stem}.${extension}`,
  };
}

function applyPlanState(report: PlanReport): ProductState {
  if (report.report.compatible && report.plan.noop) return "compatible";
  if (
    report.plan.target.crop.required ||
    report.plan.target.first_frame?.required
  ) {
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

function pasteBoxFromLast(
  requirements: string,
  profile: string | null,
): string {
  if (!requirements.trim()) return "";
  if (profile && (requirements === profile || isProfileId(requirements)))
    return "";
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
  if (!last)
    return { requirements: "", target: null, confirmed: null, profile: null };
  const profile =
    last.profile && isProfileId(last.profile) ? last.profile : null;
  const json = last.constraintsJson?.trim() ?? "";
  if (json) {
    try {
      const parsed = JSON.parse(json) as ConstraintSet & { profile?: string };
      if (
        parsed.schema === "fitifact.constraints/v1" &&
        Array.isArray(parsed.hard)
      ) {
        const mediaOnly =
          constraintsLookLikeMedia(json) && !constraintsLookLikeImage(json);
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
        return {
          requirements: "",
          target: null,
          confirmed: null,
          profile: null,
        };
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
  if (constraintsLookLikeMedia(json) && !constraintsLookLikeImage(json))
    return null;
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

export function RejectBlock({
  name,
  message,
  offerDesktop,
}: {
  name: string;
  message: string;
  offerDesktop: boolean;
}) {
  return (
    <div className="reject-card" role="status">
      <strong>{name}</strong>
      <p className="ft-notice reject-message">{message}</p>
      {offerDesktop ? (
        <a className="ft-btn download-button" href={desktopGuideHref()}>
          Get the desktop app
        </a>
      ) : null}
    </div>
  );
}

function mediaPlanState(outcome: DesktopPlanOutcome): ProductState {
  if (outcome.kind === "compatible") return "compatible";
  if (outcome.kind === "cannot_satisfy") return "cannot_satisfy";
  return "planned";
}

function mediaKept(outcome: DesktopPlanOutcome | null): string[] {
  return describeKept(null, outcome?.plan?.preserved ?? []);
}

export function useFitifactSession() {
  const clientRef = useRef<ImageWorkerClient | null>(null);
  const nativeJobRef = useRef<string | null>(null);
  const operationRef = useRef(0);
  const parseGen = useRef(0);
  const parseTimer = useRef<number | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const sidebarRef = useRef<HTMLElement | null>(null);
  const desktop = isDesktop();
  if (!clientRef.current) clientRef.current = new ImageWorkerClient();
  const client = clientRef.current;
  const [seed] = useState(sessionFromLastTarget);
  // A ?for= link picks the destination for the first matching file only; later choices win.
  const linkedFamilyRef = useRef(familyFromQuery(window.location.search));
  const [requirements, setRequirementsState] = useState(seed.requirements);
  // The text behind the target being confirmed. Plans persist it, not the
  // requirements their render captured, so remembered text matches its target.
  const targetTextRef = useRef(seed.requirements);
  function setRequirements(value: string) {
    targetTextRef.current = value;
    setRequirementsState(value);
  }
  const [parsed, setParsed] = useState<RequirementParse | null>(null);
  const [target, setTarget] = useState<EditableTarget | null>(seed.target);
  const [confirmedConstraintsJson, setConfirmedConstraintsJson] = useState<
    string | null
  >(seed.confirmed);
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
  const [targetMode, setTargetMode] = useState("destinations");
  const [notice, setNotice] = useState("");
  const [parsing, setParsing] = useState(false);
  const parsingId = useRef(0);
  useEffect(() => {
    const show = () =>
      setNotice(
        "Device storage is unavailable. Your session still works, but preferences may not survive a reload.",
      );
    window.addEventListener("fitifact-storage-unavailable", show);
    return () =>
      window.removeEventListener("fitifact-storage-unavailable", show);
  }, []);
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [settings, setSettings] = useState<AppSettings>(() => loadSettings());
  const [mediaPath, setMediaPath] = useState<string | null>(null);
  const [mediaArtifact, setMediaArtifact] = useState<DesktopArtifact | null>(
    null,
  );
  const [mediaOutcome, setMediaOutcome] = useState<DesktopPlanOutcome | null>(
    null,
  );
  const [mediaAdapt, setMediaAdapt] = useState<DesktopAdaptResult | null>(null);
  const [mediaChecks, setMediaChecks] = useState<CompatibilityReport | null>(
    null,
  );
  const [desktopTarget, setDesktopTarget] = useState<DesktopTarget | null>(
    null,
  );
  const [doctor, setDoctor] = useState<DoctorReport | null>(null);
  const [setup, setSetup] = useState<SetupState>(() => loadSetup());
  const [setupOpen, setSetupOpenState] = useState(
    () =>
      desktop &&
      !loadSetup().completed &&
      readStored("fitifact.setup-dismissed.v1") !== "true",
  );
  function setSetupOpen(open: boolean) {
    setSetupOpenState(open);
    if (!open) writeStored("fitifact.setup-dismissed.v1", "true");
  }
  const [lastRejected, setLastRejected] = useState<{
    name: string;
    message: string;
  } | null>(null);
  const [activeProfile, setActiveProfile] = useState<string | null>(
    seed.profile,
  );
  const [inspectingName, setInspectingName] = useState<string | null>(null);
  // The "Adapt and download" operation; only its own validated output downloads.
  const downloadAfterAdaptRef = useRef<number | null>(null);
  const [appliedFromLastUsed, setAppliedFromLastUsed] = useState(false);
  const confirmedRef = useRef<string | null>(null);
  const targetDirtyRef = useRef(false);
  const processingRef = useRef(false);
  const settingsRef = useRef(settings);
  const sourceFileRef = useRef<File | null>(null);
  const mediaPathRef = useRef<string | null>(null);
  // A video the browser worker probed (the web app has no media path).
  const browserMediaRef = useRef(false);
  const setupRef = useRef(setup);
  const activeProfileRef = useRef<string | null>(seed.profile);
  confirmedRef.current = confirmedConstraintsJson;
  targetDirtyRef.current = targetDirty;
  settingsRef.current = settings;
  mediaPathRef.current = mediaPath;
  browserMediaRef.current = !desktop && Boolean(mediaArtifact);
  setupRef.current = setup;
  activeProfileRef.current = activeProfile;

  useEffect(
    () => () => {
      client.dispose();
      if (parseTimer.current) window.clearTimeout(parseTimer.current);
    },
    [client],
  );

  const inspectRef = useRef<
    (file: File, desktopPath?: string) => Promise<boolean | undefined>
  >(async () => undefined);

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
        file =
          [...data.files].find((candidate) =>
            candidate.type.startsWith("image/"),
          ) ?? null;
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
    const focusable = () =>
      Array.from(
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
      unlistenProgress = await listen<ProgressUpdate>(
        "fitifact-progress",
        (event) => {
          const payload = event.payload as ProgressUpdate & {
            operationId?: string;
          };
          if (payload.operationId === nativeJobRef.current)
            setProgress(payload);
        },
      );
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
      setNotice(
        "The video tools could not be checked. Photos still work. Try Check again after restoring the desktop tools.",
      );
    }
  }

  const inspectedSafeImage = Boolean(
    inspection?.kind && PREVIEW_KINDS.has(inspection.kind),
  );
  const previewBlob = useMemo(
    () =>
      previewBuffer ? new Blob([previewBuffer], { type: "image/png" }) : null,
    [previewBuffer],
  );
  const sourceUrl = useObjectUrl(
    previewBlob ?? (inspectedSafeImage ? sourceFile : null),
  );
  const outputFormat = !desktop && mediaArtifact
    ? "mp4"
    : (adapted?.output_artifact.image?.format ?? plan?.plan.target.format ?? null);
  const download = useMemo(
    () =>
      outputFormat &&
      sourceFile &&
      (outputFormat === "jpeg" ||
        outputFormat === "png" ||
        outputFormat === "webp" ||
        outputFormat === "mp4")
        ? {
            ...outputDetails(
              outputFormat,
              sourceFile.name,
              settings.fitifactSuffix,
            ),
            ...(state === "compatible" && !outputBuffer
              ? { name: sourceFile.name }
              : {}),
          }
        : null,
    [outputFormat, sourceFile, settings.fitifactSuffix, state, outputBuffer],
  );
  const outputBlob = useMemo(
    () =>
      outputBuffer && download
        ? new Blob([outputBuffer], { type: download.mime })
        : null,
    [outputBuffer, download],
  );
  const originalBlob = useMemo(
    () =>
      state === "compatible" &&
      !outputBuffer &&
      sourceFile &&
      download &&
      inspection?.kind !== "heic"
        ? new Blob([sourceFile], { type: download.mime })
        : null,
    [state, outputBuffer, sourceFile, download, inspection],
  );
  const downloadBlob = outputBlob ?? originalBlob;
  const downloadUrl = useObjectUrl(downloadBlob);
  useEffect(() => {
    if (downloadAfterAdaptRef.current !== operationRef.current) return;
    if (!downloadUrl || !download) return;
    if (state !== "adapted" && state !== "compatible") return;
    downloadAfterAdaptRef.current = null;
    const link = document.createElement("a");
    link.href = downloadUrl;
    link.download = download.name;
    document.body.append(link);
    link.click();
    link.remove();
  }, [downloadUrl, download, state]);

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

  function handleFailure(caught: unknown, operation: number, native = false) {
    if (operation !== operationRef.current) return;
    const failure = caught instanceof WorkerFailure ? caught : null;
    const report = failure?.report ?? {
      schema: "fitifact.error/v1" as const,
      code: "EXECUTION_FAILED",
      message:
        caught instanceof Error ? caught.message : "Local processing failed.",
    };
    const mapped = mapErrorCode(report.code, report.message);
    setState(
      failure?.state && mapped === report.code
        ? failure.state
        : productStateForError({ code: mapped }),
    );
    const message = native || mediaPathRef.current || browserMediaRef.current
      ? report.message
      : errorCopy(mapped, report.message);
    setError({
      ...report,
      message,
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
    setMediaChecks(null);
    if (clearSource) {
      setSourceFile(null);
      sourceFileRef.current = null;
      setInspection(null);
      setHeicPreviewMissing(false);
      setMediaPath(null);
      setMediaArtifact(null);
      browserMediaRef.current = false;
    }
  }

  function persistConfirmed(
    constraintsJson: string,
    text = targetTextRef.current,
    profile?: string | null,
  ) {
    const nextProfile =
      profile === undefined ? activeProfileRef.current : profile;
    targetTextRef.current = text;
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
    if (mediaArtifact) return fileKindFromArtifact(mediaArtifact);
    if (mediaPathRef.current) return "video";
    if (inspection || sourceFile) return "image";
    return null;
  }

  async function applyProfileId(
    id: string,
    existingOperation?: number,
    fromLastUsed = false,
  ) {
    const operation = existingOperation ?? beginOperation();
    clearDerivedState(false);
    parseGen.current += 1;
    setRequirements("");
    setParsed(null);
    setError(null);
    setTargetDirty(false);
    setEditingTarget(false);
    setAppliedFromLastUsed(fromLastUsed);
    if (mediaPathRef.current && !desktop) {
      handleFailure(unsupportedFailure(refuseMessage("video")), operation);
      return;
    }
    if (mediaPathRef.current && desktop) {
      const nativeTarget: DesktopTarget = { profile: id };
      setDesktopTarget(nativeTarget);
      setTarget(null);
      try {
        const report = await desktopCompileTarget(nativeTarget);
        if (operation !== operationRef.current) return;
        nativeTarget.constraintsJson = JSON.stringify(report);
        persistConfirmed(nativeTarget.constraintsJson, "", id);
        await runDesktopPlan(nativeTarget, operation);
      } catch (caught) {
        handleFailure(caught, operation);
      }
      return;
    }
    if (browserMediaRef.current) {
      try {
        const { report } = await client.compileProfile<ConstraintSet>(id);
        if (operation !== operationRef.current) return;
        const confirmed = JSON.stringify(report);
        setTarget(null);
        setDesktopTarget({ profile: id, constraintsJson: confirmed });
        persistConfirmed(confirmed, "", id);
        await runBrowserMediaPlan(confirmed, operation);
      } catch (caught) {
        handleFailure(caught, operation);
      }
      return;
    }
    try {
      const { report } = await client.compileProfile<ConstraintSet>(id);
      if (operation !== operationRef.current) return;
      const confirmed = JSON.stringify(report);
      setTarget(editableFromSet(report));
      setDesktopTarget({ profile: id, constraintsJson: confirmed });
      persistConfirmed(confirmed, "", id);
      if ((sourceFileRef.current || inspection) && !mediaPathRef.current)
        await runPlan(confirmed, operation);
      else setState("requirements_ready");
    } catch (caught) {
      handleFailure(caught, operation);
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
    if (
      (kind === "video" || kind === "audio") &&
      desktop &&
      doctor &&
      !doctor.healthy
    ) {
      setState("inspected");
      return;
    }
    const linked = linkedFamilyRef.current;
    const linkedProfile = linked
      ? profileForFamily(linked, currentSetup.discordCap, kind)
      : null;
    if (linkedProfile) {
      linkedFamilyRef.current = null;
      await applyProfileId(linkedProfile, operation);
      return;
    }
    const setupWins =
      currentSetup.completed && setupIsNewer(currentSetup, last?.savedAt);
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
    if (kind === "audio" && !desktop) {
      handleFailure(unsupportedFailure(refuseMessage("audio")), beginOperation());
      return;
    }
    if (
      (kind === "video" || kind === "audio") &&
      desktop &&
      doctor &&
      !doctor.healthy
    ) {
      setError({
        schema: "fitifact.error/v1",
        code: "PROVIDER_MISSING",
        message: errorCopy("PROVIDER_MISSING", "Video tools did not run."),
      });
      setState("error");
      return;
    }
    const id = profileForFamily(family, setup.discordCap, kind);
    if (!id) {
      setError({
        schema: "fitifact.error/v1",
        code: "INPUT_INVALID",
        message:
          kind === "image"
            ? "That destination is for video."
            : kind === "audio"
              ? "That destination doesn't take sound files."
              : "That destination is for photos.",
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
    ++operationRef.current;
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
    const imageItem = [...(event.clipboardData?.items ?? [])].find((item) =>
      item.type.startsWith("image/"),
    );
    if (imageItem) return;
    const pasted = event.clipboardData.getData("text/plain");
    if (!pasted) return;
    event.preventDefault();
    const area = event.currentTarget;
    const start = area.selectionStart ?? area.value.length;
    const end = area.selectionEnd ?? area.value.length;
    editRequirements(
      `${area.value.slice(0, start)}${pasted}${area.value.slice(end)}`,
      true,
    );
  }

  function editTarget(update: (current: EditableTarget) => EditableTarget) {
    if (!target) return;
    ++operationRef.current;
    setTarget((current) => (current ? update(current) : current));
    setTargetDirty(true);
    clearDerivedState(false);
    setError(null);
    setState(inspection ? "inspected" : "requirements_ready");
  }

  async function autoParse(value: string) {
    const id = ++parsingId.current;
    setParsing(true);
    try {
      await parseRequirements(value);
    } finally {
      if (id === parsingId.current) setParsing(false);
    }
  }

  async function parseRequirements(value: string) {
    const text = value.trim();
    if (!text) return;
    const gen = ++parseGen.current;
    const nativeTarget = desktopTargetFromText(text);
    if (nativeTarget?.profile) {
      if (gen !== parseGen.current) return;
      await applyProfileId(nativeTarget.profile);
      return;
    }
    const browserConstraints = browserMediaRef.current
      ? nativeTarget?.constraintsJson
      : undefined;
    if (nativeTarget && ((desktop && mediaPathRef.current) || browserConstraints)) {
      setDesktopTarget(nativeTarget);
      setParsed(null);
      setTarget(null);
      setTargetDirty(false);
      setEditingTarget(false);
      setError(null);
      persistConfirmed(browserConstraints ?? JSON.stringify(nativeTarget), text, null);
      if (gen !== parseGen.current) return;
      if (browserConstraints) await runBrowserMediaPlan(browserConstraints);
      else await runDesktopPlan(nativeTarget);
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
          message: errorCopy(
            "INPUT_INVALID",
            "No supported image format, size, or dimension requirement was found.",
          ),
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
      if (mediaPathRef.current || browserMediaRef.current) {
        setState("inspected");
        setError({
          schema: "fitifact.error/v1",
          code: "INPUT_INVALID",
          message:
            fileKindFromArtifact(mediaArtifact) === "audio"
              ? "This is audio. Paste a profile id such as gmail/attachment, or choose a destination below."
              : "This is video. Paste a profile id such as discord/video-upload, or choose a destination below.",
        });
        return;
      }
      if (sourceFile && inspection && !processingRef.current)
        await runPlan(confirmed);
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
    const previousState = state;
    const hadValidSource = Boolean(inspection || mediaArtifact);
    processingRef.current = true;
    setState("processing");
    setInspectingName(fileNameFromPath(path));
    try {
      const artifact = await desktopInspect(path);
      if (operation !== operationRef.current) return;
      clearDerivedState(true);
      const placeholder = new File([], fileNameFromPath(path));
      setSourceFile(placeholder);
      sourceFileRef.current = placeholder;
      setMediaPath(path);
      mediaPathRef.current = path;
      setMediaArtifact(artifact);
      const nativeTarget = desktopTarget ?? desktopTargetFromText(requirements);
      if (nativeTarget && !constraintsLookLikeImage(confirmedRef.current)) {
        setDesktopTarget(nativeTarget);
        await runDesktopPlan(nativeTarget, operation);
      } else {
        await maybeApplyAfterInspect(
          fileKindFromArtifact(artifact) ?? "video",
          operation,
        );
      }
    } catch (caught) {
      if (operation !== operationRef.current) return;
      if (hadValidSource) {
        setState(previousState);
        setLastRejected({
          name: fileNameFromPath(path),
          message:
            caught instanceof Error
              ? caught.message
              : "This file could not be inspected.",
        });
      } else handleFailure(caught, operation, true);
    } finally {
      if (operation === operationRef.current) {
        setProgress(null);
        setInspectingName(null);
        processingRef.current = false;
      }
    }
  }

  async function inspectBrowserMedia(file: File, operation: number) {
    const previousState = state;
    const hadValidSource = Boolean(inspection || mediaArtifact);
    setError(null);
    processingRef.current = true;
    setState("processing");
    setInspectingName(file.name);
    try {
      const { report } = await client.mediaInspect<DesktopArtifact>(
        file,
        onProgress(operation),
      );
      if (operation !== operationRef.current) return;
      clearDerivedState(true);
      setLastRejected(null);
      setSourceFile(file);
      sourceFileRef.current = file;
      setMediaArtifact(report);
      browserMediaRef.current = true;
      await maybeApplyAfterInspect(fileKindFromArtifact(report) ?? "video", operation);
    } catch (caught) {
      if (operation !== operationRef.current) return;
      const failure = caught instanceof WorkerFailure ? caught : null;
      if (hadValidSource) {
        setState(previousState);
        setLastRejected({
          name: file.name,
          message: failure?.report.message ?? "This video could not be inspected.",
        });
      } else if (failure?.report.code === "INSPECTION_UNSUPPORTED") {
        clearDerivedState(true);
        setLastRejected({ name: file.name, message: failure.report.message });
        handleFailure(caught, operation, true);
      } else handleFailure(caught, operation, true);
    } finally {
      if (operation === operationRef.current) {
        setProgress(null);
        setInspectingName(null);
        processingRef.current = false;
      }
    }
  }

  async function openDesktopPath(path: string) {
    if (state === "processing") return;
    const header = await desktopReadHeader(path);
    const kind = classifyInput(header);
    if (isDesktopMedia(kind)) {
      const operation = beginOperation();
      setError(null);
      await inspectDesktopMedia(path, operation);
      return;
    }
    if (!isStillImage(kind)) {
      noteRejected(
        fileNameFromPath(path),
        kind === "unsupported"
          ? "This file could not be identified. It may be damaged or unsupported. Choose a supported photo, video, or audio file."
          : refuseMessage(kind),
      );
      return;
    }
    const bytes = await desktopReadImage(path);
    const type =
      kind === "jpeg"
        ? "image/jpeg"
        : kind === "png"
          ? "image/png"
          : kind === "webp"
            ? "image/webp"
            : "";
    await inspectFile(
      fileFromBytes(bytes, fileNameFromPath(path), type || undefined),
    );
  }

  const openDesktopPathRef = useRef(openDesktopPath);
  openDesktopPathRef.current = openDesktopPath;

  async function inspectFile(
    file: File,
    desktopPath?: string,
    skipRememberedTarget = false,
  ) {
    if (state === "processing") return;
    const header = new Uint8Array(
      await file.slice(0, Math.min(file.size, 65_536)).arrayBuffer(),
    );
    const kind = classifyInput(header);
    if (!isStillImage(kind)) {
      if (desktop && isDesktopMedia(kind)) {
        if (desktopPath) {
          await inspectDesktopMedia(desktopPath, beginOperation());
          return;
        }
        noteRejected(
          file.name,
          "Drop or choose the file so Fitifact can write next to the original.",
        );
        return;
      }
      if (!desktop && (kind === "video" || kind === "matroska")) {
        await inspectBrowserMedia(file, beginOperation());
        return;
      }
      noteRejected(file.name, refuseMessage(kind));
      return;
    }
    setLastRejected(null);
    const operation = beginOperation();
    const previousState = state;
    const hadValidSource = Boolean(inspection || mediaArtifact);
    const retainedFile = sourceFileRef.current;
    const retainedConstraints = confirmedRef.current;
    setError(null);
    processingRef.current = true;
    setState("processing");
    setInspectingName(file.name);
    try {
      const { report, preview } = await client.inspect<InspectReport>(
        file,
        onProgress(operation),
      );
      if (operation !== operationRef.current) return;
      clearDerivedState(false);
      setSourceFile(file);
      sourceFileRef.current = file;
      setMediaPath(null);
      mediaPathRef.current = null;
      setMediaArtifact(null);
      browserMediaRef.current = false;
      setInspection(report);
      setHeicPreviewMissing(report.kind === "heic" && !preview);
      if (preview) setPreviewBuffer(preview);
        if (skipRememberedTarget) setState("inspected");
        else await maybeApplyAfterInspect("image", operation);
        return operation === operationRef.current;
    } catch (caught) {
      if (operation !== operationRef.current) return;
      if (hadValidSource) {
        // A failed inspection clears the worker's source. Restore its confirmed
        // contract before exposing any retained plan or action again. A retained
        // video lives in the worker's separate media slot and is untouched.
        if (retainedFile && !browserMediaRef.current) {
          try {
            await client.inspect(retainedFile);
            if (operation !== operationRef.current) return;
            if (retainedConstraints) await client.plan(retainedConstraints);
            if (operation !== operationRef.current) return;
          } catch (recoveryError) {
            clearDerivedState(false);
            handleFailure(recoveryError, operation);
            return;
          }
        }
        setState(previousState);
        setLastRejected({
          name: file.name,
          message:
            caught instanceof Error
              ? caught.message
              : "This file could not be inspected. Your previous file is still available.",
        });
      } else {
        setSourceFile(file);
        sourceFileRef.current = file;
        handleFailure(caught, operation);
      }
    } finally {
      if (operation === operationRef.current) {
        setProgress(null);
        setInspectingName(null);
        processingRef.current = false;
      }
    }
  }

  inspectRef.current = inspectFile;

  async function runDesktopPlan(
    targetSpec: DesktopTarget,
    existingOperation?: number,
  ) {
    const path = mediaPathRef.current;
    if (!path) return;
    const operation = existingOperation ?? beginOperation();
    processingRef.current = true;
    setState("processing");
    setError(null);
    setMediaOutcome(null);
    setMediaAdapt(null);
    try {
      const [outcome, report, compiled] = await Promise.all([
        desktopPlan(path, targetSpec),
        desktopCheck(path, targetSpec),
        desktopCompileTarget(targetSpec),
      ]);
      if (operation !== operationRef.current) return;
      setMediaOutcome(outcome);
      setMediaChecks(report);
      persistConfirmed(
        JSON.stringify(compiled),
        targetTextRef.current,
        targetSpec.profile ?? activeProfileRef.current,
      );
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

  async function runBrowserMediaPlan(
    constraintsJson: string,
    existingOperation?: number,
  ) {
    const operation = existingOperation ?? beginOperation();
    processingRef.current = true;
    setState("processing");
    setError(null);
    setMediaOutcome(null);
    setMediaAdapt(null);
    setOutputBuffer(null);
    try {
      const { report } = await client.mediaPlan<MediaPlanReport>(
        constraintsJson,
        onProgress(operation),
      );
      if (operation !== operationRef.current) return;
      setMediaOutcome(report.outcome);
      setMediaChecks(report.report);
      persistConfirmed(
        JSON.stringify(report.constraints),
        targetTextRef.current,
        activeProfileRef.current,
      );
      if (report.needs_encoder) {
        handleFailure(unsupportedFailure(NEEDS_DESKTOP_ENCODE), operation);
        return;
      }
      setState(mediaPlanState(report.outcome));
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
    if (browserMediaRef.current) {
      await runBrowserMediaPlan(constraintsJson, existingOperation);
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
      const { report, preview, constraintsSnapshot } =
        await client.plan<PlanReport>(constraintsJson, onProgress(operation));
      if (operation !== operationRef.current) return;
      setPlan(report);
      persistConfirmed(constraintsSnapshot ?? constraintsJson);
      if (preview) setPreviewBuffer(preview);
      setCropConsent(false);
      setFirstFrameConsent(false);
      const next = applyPlanState(report);
      if (next === "compatible") setOutputBuffer(null);
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
    if (
      !sourceFile ||
      !confirmedConstraintsJson ||
      !target ||
      state === "processing"
    )
      return;
    const operation = beginOperation();
    processingRef.current = true;
    setState("processing");
    setError(null);
    setPlan(null);
    setAdapted(null);
    setOutputBuffer(null);
    try {
      const draft = draftConstraintsJson();
      const { report, preview, constraintsSnapshot } =
        await client.replan<PlanReport>(
          confirmedConstraintsJson,
          draft,
          onProgress(operation),
        );
      if (operation !== operationRef.current) return;
      setPlan(report);
      persistConfirmed(
        constraintsSnapshot ?? draft,
        targetTextRef.current,
        null,
      );
      setTargetDirty(false);
      setEditingTarget(false);
      if (preview) setPreviewBuffer(preview);
      setCropConsent(false);
      setFirstFrameConsent(false);
      const next = applyPlanState(report);
      if (next === "compatible") setOutputBuffer(null);
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

  function settleMediaResult(
    result: DesktopAdaptResult,
    operation: number,
  ): ProductState | null {
    setMediaAdapt(result);
    if (result.status === "compatible") {
      setState("compatible");
      return "compatible";
    }
    if (
      result.status === "adapted" &&
      result.validation?.status === "pass" &&
      result.validation.checks.compatible
    ) {
      setState("adapted");
      return "adapted";
    }
    if (result.status === "cannot_satisfy") {
      setState("cannot_satisfy");
      return "cannot_satisfy";
    }
    const code =
      result.error && "code" in result.error
        ? String(result.error.code)
        : "VALIDATION_FAILED";
    handleFailure(
      new WorkerFailure(productStateForError({ code }), {
        schema: "fitifact.error/v1",
        code,
        message:
          result.error && "message" in result.error
            ? String(result.error.message)
            : "Adaptation failed.",
      }),
      operation,
    );
    return null;
  }

  async function adaptImage(downloadAfter = false) {
    downloadAfterAdaptRef.current = null;
    if (mediaPathRef.current && desktopTarget) {
      if (state === "processing") return;
      const operation = beginOperation();
      processingRef.current = true;
      setState("processing");
      setError(null);
      try {
        const jobId = crypto.randomUUID();
        nativeJobRef.current = jobId;
        const result = await desktopAdapt(
          mediaPathRef.current,
          desktopTarget,
          jobId,
        );
        if (operation !== operationRef.current) return;
        settleMediaResult(result, operation);
      } catch (caught) {
        handleFailure(caught, operation);
      } finally {
        nativeJobRef.current = null;
        if (operation === operationRef.current) {
          setProgress(null);
          processingRef.current = false;
        }
      }
      return;
    }
    if (browserMediaRef.current) {
      if (state === "processing" || !confirmedConstraintsJson || !sourceFile)
        return;
      const operation = beginOperation();
      processingRef.current = true;
      setState("processing");
      setError(null);
      try {
        downloadAfterAdaptRef.current = downloadAfter ? operation : null;
        const { report, output } = await client.mediaAdapt<DesktopAdaptResult>(
          confirmedConstraintsJson,
          outputDetails("mp4", sourceFile.name, settings.fitifactSuffix).name,
          onProgress(operation),
        );
        if (operation !== operationRef.current) return;
        // Only a validated output is ever offered for download.
        if (settleMediaResult(report, operation) === "adapted" && output)
          setOutputBuffer(output);
        else if (report.status !== "compatible")
          downloadAfterAdaptRef.current = null;
      } catch (caught) {
        downloadAfterAdaptRef.current = null;
        handleFailure(caught, operation);
      } finally {
        if (operation === operationRef.current) {
          setProgress(null);
          processingRef.current = false;
        }
      }
      return;
    }
    if (
      !plan ||
      !confirmedConstraintsJson ||
      targetDirty ||
      state === "processing"
    )
      return;
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
      downloadAfterAdaptRef.current = downloadAfter ? operation : null;
      const { report, output } = await client.adapt<AdaptReport>(
        confirmedConstraintsJson,
        plan.plan.target.crop.required ? crop : null,
        firstFrameConsent,
        onProgress(operation),
      );
      if (operation !== operationRef.current) return;
      if (!report.report.compatible) downloadAfterAdaptRef.current = null;
      setAdapted(report);
      setOutputBuffer(output ?? null);
      setState(report.status === "compatible" ? "compatible" : "adapted");
    } catch (caught) {
      downloadAfterAdaptRef.current = null;
      handleFailure(caught, operation);
    } finally {
      if (operation === operationRef.current) {
        setProgress(null);
        processingRef.current = false;
      }
    }
  }

  async function cancel() {
    if (nativeJobRef.current) {
      try {
        const id = nativeJobRef.current;
        const requested = await desktopCancel(id);
        if (requested && nativeJobRef.current === id)
          setProgress({ stage: "Cancelling local processing", percent: 0 });
      } catch {
        setNotice(
          "Cancellation could not be requested. The job is still running.",
        );
      }
      return;
    }
    ++operationRef.current;
    client.cancel();
    processingRef.current = false;
    downloadAfterAdaptRef.current = null;
    setInspectingName(null);
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
    try {
      const path = await desktopOpenDialog();
      if (path) await openDesktopPath(path);
    } catch (caught) {
      setNotice(
        caught instanceof Error
          ? caught.message
          : "Could not open this file. Try choosing it again.",
      );
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
      if (desktop && mediaPathRef.current) {
        const nativeTarget = { constraintsJson: saved.constraintsJson };
        setTarget(null);
        setDesktopTarget(nativeTarget);
        await runDesktopPlan(nativeTarget);
        return;
      }
      if (browserMediaRef.current) {
        setTarget(null);
        setDesktopTarget({ constraintsJson: saved.constraintsJson });
        await runBrowserMediaPlan(saved.constraintsJson);
        return;
      }
      const { report } = await client.compileConstraints<ConstraintSet>(
        saved.constraintsJson,
      );
      setTarget(
        editableFromSet(report) ?? editableTargetFromConstraints(report),
      );
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

  async function useSampleImage(scenario: SampleScenario = "inspect") {
    if (state === "processing") return;
    const sampleOperation = beginOperation();
    let activeOperation = sampleOperation;
    setState("processing");
    setProgress({ stage: "Loading the local sample", percent: 0 });
    setSidebarOpen(false);
    setNotice("");
    try {
      const response = await fetch(
        `${import.meta.env.BASE_URL}samples/too-small-640x480.png`,
      );
      if (!response.ok)
        throw new Error(
          "The sample could not be loaded. Choose a file or try again.",
        );
      const blob = await response.blob();
      if (sampleOperation !== operationRef.current) return;
      const inspected = await inspectFile(
        new File([blob], "sample-640x480.png", { type: "image/png" }),
        undefined,
        scenario !== "inspect",
      );
      if (!inspected) return;
      activeOperation = operationRef.current;
      if (scenario !== "inspect") {
        const inspectedOperation = operationRef.current;
        const text =
          scenario === "unchanged"
            ? "PNG, max 2 MB"
            : scenario === "format"
              ? "JPEG, max 2 MB"
              : "JPEG, exactly 480 x 480";
        const { report } = await client.compile<RequirementParse>(text);
        if (inspectedOperation !== operationRef.current) return;
        if (!report.constraints)
          throw new Error("Sample requirements could not be read.");
        const json = JSON.stringify(report.constraints);
        setRequirements(text);
        setParsed(report);
        setTarget(editableTargetFromConstraints(report.constraints));
        setTargetDirty(false);
        setTargetMode("custom");
        persistConfirmed(json, text, null);
        await runPlan(json, inspectedOperation);
      }
    } catch (caught) {
      if (activeOperation !== operationRef.current) return;
      setProgress(null);
      setState(sourceFileRef.current ? "inspected" : "idle");
      setNotice(
        caught instanceof Error ? caught.message : "Could not load the sample.",
      );
    }
  }

  async function revealOutput() {
    const path = mediaAdapt?.output ?? mediaPath;
    if (!path) return;
    try {
      await desktopReveal(path);
    } catch {
      setNotice("Could not show the file. It may have moved or been removed. Last known location: " + path);
    }
  }

  async function retry() {
    if (state === "processing") return;
    if (mediaPath) await openDesktopPath(mediaPath);
    else if (sourceFile) await inspectFile(sourceFile);
    else void useSampleImage();
  }

  const checklist =
    mediaAdapt?.validation?.checks.checks ??
    mediaAdapt?.report?.checks ??
    mediaChecks?.checks ??
    adapted?.report.checks ??
    plan?.report.checks ??
    [];
  const status = STATE_COPY[state];
  const showWork = Boolean(inspection || mediaArtifact);
  const refusedInput = Boolean(lastRejected && !showWork && state !== "processing");
  const statusTitle =
    state === "crop_approval_required"
      ? approvalTitle(plan)
      : refusedInput
        ? "Fitifact can't adapt this file"
        : status.title;
  const problems = plan
    ? describeProblems(plan)
    : mediaChecks
      ? describeProblemsFromChecks(mediaChecks)
      : (mediaOutcome?.blocking?.map((item) => item.message) ?? []);
  const actions = plan
    ? describeActions(plan, state === "adapted")
    : describeMediaActions(mediaOutcome?.plan?.steps);
  const kept = plan ? ["Original file", ...describeKept(plan)] : mediaKept(mediaOutcome);
  const verifiedLine = describeVerified({
    compatible: state === "compatible",
    adapted: state === "adapted",
    checks: checklist,
  });
  const leftover = leftoverNote(
    parsed?.unresolved.map((item) => item.text) ?? [],
  );
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
  const needsApproval = Boolean(
    (plan?.plan.target.crop.required && !cropConsent) ||
      (plan?.plan.target.first_frame?.required && !firstFrameConsent),
  );
  const formatOptions: OutputFormat[] = ["jpeg", "png", "webp"];
  const chooseLabel = "Choose a file";
  const accept = desktop ? ACCEPT_DESKTOP : ACCEPT;
  const doctorUnhealthy = Boolean(desktop && doctor && !doctor.healthy);
  const fileKind = currentKind();
  const chips = destinationChips(setup.discordCap, {
    includeVideo: desktop || fileKind === "video",
  });
  const activeFamily = activeProfile ? familyForProfile(activeProfile) : null;
  const usingCopy = activeFamily
    ? appliedFromLastUsed
      ? sameAsLastTimeCopy(activeFamily, setup.discordCap)
      : usingDestinationCopy(activeFamily, setup.discordCap)
    : null;

  function focusPasteOverride() {
    setTargetMode("custom");
    requestAnimationFrame(() => {
      const area = document.getElementById("requirements");
      area?.scrollIntoView({ block: "center", behavior: "instant" });
      if (area instanceof HTMLTextAreaElement) area.focus();
    });
  }

  const reportObject =
    state === "adapted"
      ? (mediaAdapt ?? adapted)
      : state === "compatible"
        ? (mediaChecks ?? plan?.report)
        : null;
  const reportBlob = useMemo(
    () =>
      reportObject
        ? new Blob([JSON.stringify(reportObject, null, 2)], {
            type: "application/json",
          })
        : null,
    [reportObject],
  );
  const reportUrl = useObjectUrl(reportBlob);
  const constraintsBlob = useMemo(
    () =>
      confirmedConstraintsJson
        ? new Blob([confirmedConstraintsJson], { type: "application/json" })
        : null,
    [confirmedConstraintsJson],
  );
  const constraintsUrl = useObjectUrl(constraintsBlob);
  const warnings = [
    ...new Set([
      ...(state === "adapted" && adapted ? adapted.disclosures ?? [] : [
      ...(plan?.plan.warnings ?? []),
      ...(plan?.plan.target.quality_warnings ?? []),
      ...(plan?.plan.target.upscale_warnings ?? []),
      ...(mediaOutcome?.warnings ?? []),
      ...(mediaOutcome?.plan?.warnings ?? []),
      ...(mediaOutcome?.plan?.steps.flatMap((step) => step.warnings ?? []) ??
        []),
      ]),
    ]),
  ];
  const resultReady = state === "adapted" || state === "compatible";
  const staleRules = (() => {
    const profile = registryProfile(activeProfile);
    return Boolean(profile && isStale(profile));
  })();
  // Warning: a result exists but rests on something the user should re-check.
  const tone: Tone =
    resultReady && (leftover || staleRules) ? "warning" : refusedInput ? "refusal" : status.tone;
  const measured = measuredRows(
    plan?.report.checks ?? mediaChecks?.checks ?? [],
    state === "adapted"
      ? (mediaAdapt?.validation?.checks.checks ?? adapted?.report.checks ?? null)
      : null,
  );
  const adaptLabel = mediaPath ? "Adapt and save" : "Adapt and download";
  const outputFacts = mediaAdapt?.validation?.artifact
    ? inspectMediaLine(
        mediaAdapt.validation.artifact,
        formatBytes(mediaAdapt.validation.artifact.byte_length),
      )
    : adapted
      ? inspectLine(
          adapted.output_artifact.image?.format,
          adapted.output_artifact.image?.width,
          adapted.output_artifact.image?.height,
          adapted.output_artifact.byte_length,
        )
      : inspectFacts;
  let targetError: string | null = null;
  if (targetDirty && target) {
    try {
      constraintSetFromEditable(target);
    } catch (caught) {
      targetError =
        caught instanceof Error ? caught.message : "Check the target values.";
    }
  }
  return {
    showWork,
    inspectingName,
    downloadBlob,
    tone,
    staleRules,
    measured,
    adaptLabel,
    sidebarOpen,
    setSidebarOpen,
    notice,
    setNotice,
    dragging,
    state,
    onDropKey,
    setDragging,
    onDrop,
    desktop,
    chooseDesktopFile,
    chooseLabel,
    fileInputRef,
    accept,
    inspectFile,
    useSampleImage,
    status,
    statusTitle,
    progress,
    mediaPath,
    cancel,
    lastRejected,
    error,
    sourceUrl,
    sourceFile,
    inspectFacts,
    heicPreviewMissing,
    targetMode,
    setTargetMode,
    focusPasteOverride,
    chips,
    activeProfile,
    doctorUnhealthy,
    fileKind,
    setup,
    applyDestination,
    usingCopy,
    confirmedConstraintsJson,
    applySaved,
    requirements,
    onRequirementsPaste,
    editRequirements,
    understood,
    leftover,
    parsing,
    target,
    setEditingTarget,
    editingTarget,
    activeFamily,
    formatOptions,
    editTarget,
    targetDirty,
    replan,
    targetError,
    desktopTarget,
    resultReady,
    outputFacts,
    verifiedLine,
    downloadUrl,
    download,
    mediaArtifact,
    outputBuffer,
    mediaAdapt,
    revealOutput,
    nativeJobRef,
    retry,
    refreshDoctor,
    plan,
    mediaOutcome,
    problems,
    actions,
    kept,
    warnings,
    crop,
    cropPosition,
    setCropPosition,
    cropConsent,
    setCropConsent,
    firstFrameConsent,
    setFirstFrameConsent,
    adaptImage,
    needsApproval,
    checklist,
    reportUrl,
    constraintsUrl,
    setupOpen,
    doctor,
    setSetup,
    commitSetup,
    setSetupOpen,
    sidebarRef,
    settings,
    updateSettings,
  };
}

export type FitifactSession = ReturnType<typeof useFitifactSession>;
