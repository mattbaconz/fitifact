// The panel: framed over an upload page with the files the content script held,
// or opened in a tab from the popup to fit one chosen video and download it.
import { formatBytes } from "../../../web/src/lib/constraints";
import {
  inspectMediaLine,
  type DesktopAdaptResult,
  type DesktopArtifact,
  type MediaPlanReport,
} from "../../../web/src/lib/desktop-engine";
import { FAMILY_LABEL, DISCORD_CAPS, type DestinationFamily, type DiscordCap } from "../../../web/src/lib/destinations";
import { errorCopy } from "../../../web/src/lib/error-copy";
import { describeKept, describeMediaActions, describeProblemsFromChecks } from "../../../web/src/lib/explain";
import { ageLabel, capLabel, daysSince, isStale, registryProfile } from "../../../web/src/lib/registry";
import { outputDetails } from "../../../web/src/lib/share";
import type { ConstraintSet } from "../../../web/src/types";
import { ImageWorkerClient, WorkerFailure, type ProgressUpdate } from "../../../web/src/worker/client";
import { fillPlans, loadPlan, savePlan, SITES, videoCap, videoProfile } from "./sites";

const byId = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
const ui = {
  sheet: byId("sheet"),
  count: byId("count"),
  plan: byId<HTMLSelectElement>("plan"),
  choose: byId("choose"),
  destination: byId<HTMLSelectElement>("destination"),
  pick: byId<HTMLButtonElement>("pick"),
  file: byId<HTMLInputElement>("file"),
  title: byId("title"),
  name: byId("name"),
  facts: byId("facts"),
  detail: byId("detail"),
  rows: byId("rows"),
  progress: byId("progress"),
  fill: byId("fill"),
  notice: byId("notice"),
  noticeIcon: document.getElementById("notice-icon") as unknown as SVGPathElement,
  noticeText: byId("notice-text"),
  fit: byId<HTMLButtonElement>("fit"),
  download: byId<HTMLAnchorElement>("download"),
  original: byId<HTMLButtonElement>("original"),
  cancel: byId<HTMLButtonElement>("cancel"),
};

interface Held {
  port: MessagePort;
  files: File[];
  /** Indices of the files over the cap when they were picked. */
  over: number[];
  family: DestinationFamily;
}

interface View {
  title: string;
  name?: string;
  facts?: string;
  detail?: string;
  rows?: Array<[string, string]>;
  progress?: number;
  notice?: string;
  /** The notice's tone; warning unless it confirms a verified result. */
  tone?: Tone;
  /** The fit button's label; shown only when there is a plan to run. */
  fit?: string;
  original?: boolean;
  /** The quiet action's label; null hides it. */
  cancel?: string | null;
  download?: File;
  /** Offer the Discord plan choice, which can change the cap. */
  plan?: boolean;
}

type Choice = "fit" | "original" | "cancel" | "replan";
type Tone = "success" | "warning";

// The web app's CheckPassIcon and WarningIcon: a tone is never color alone.
const TONE_ICON: Record<Tone, string> = {
  success: "M5 12.5l4.2 4.2L19 7.5",
  warning: "M12 4l9 16H3z M12 10v4 M12 17.5h.01",
};

const client = new ImageWorkerClient();
let held: Held | null = null;
let family: DestinationFamily = "discord";
let choose: ((choice: Choice) => void) | null = null;
let working = false;
let downloadUrl: string | null = null;

function setText(element: HTMLElement, value: string | undefined) {
  element.textContent = value ?? "";
  element.hidden = !value;
}

function show(view: View) {
  ui.title.textContent = view.title;
  setText(ui.name, view.name);
  setText(ui.facts, view.facts);
  setText(ui.detail, view.detail);
  const tone = view.tone ?? "warning";
  ui.notice.hidden = !view.notice;
  ui.notice.className = `ft-notice note ft-tone-${tone}`;
  ui.noticeIcon.setAttribute("d", TONE_ICON[tone]);
  ui.noticeText.textContent = view.notice ?? "";
  ui.rows.replaceChildren(
    ...(view.rows ?? []).flatMap(([label, value]) => {
      const term = document.createElement("dt");
      const description = document.createElement("dd");
      term.textContent = label;
      description.textContent = value;
      return [term, description];
    }),
  );
  ui.rows.hidden = !view.rows?.length;
  ui.progress.hidden = view.progress === undefined;
  ui.progress.setAttribute("aria-valuenow", String(view.progress ?? 0));
  ui.fill.style.width = `${view.progress ?? 0}%`;
  ui.fit.hidden = !view.fit;
  ui.fit.textContent = view.fit ?? "";
  ui.original.hidden = !view.original || !held;
  ui.cancel.hidden = view.cancel === null;
  ui.cancel.textContent = view.cancel ?? "Cancel";
  ui.plan.hidden = !view.plan || family !== "discord";
  if (downloadUrl) URL.revokeObjectURL(downloadUrl);
  downloadUrl = view.download ? URL.createObjectURL(view.download) : null;
  ui.download.hidden = !view.download;
  if (view.download && downloadUrl) {
    ui.download.href = downloadUrl;
    ui.download.download = view.download.name;
    ui.download.textContent = `Download ${view.download.name}`;
  }
  (view.fit ? ui.fit : view.download ? ui.download : null)?.focus();
}

function ask(): Promise<Choice> {
  working = false;
  return new Promise((resolve) => {
    choose = (choice) => {
      choose = null;
      resolve(choice);
    };
  });
}

function staleNotice(profileId: string, destination: string): string | undefined {
  const profile = registryProfile(profileId);
  if (!profile || !isStale(profile)) return undefined;
  return `${destination}'s rules were last checked ${ageLabel(daysSince(profile.last_verified))}. Check before you rely on it.`;
}

function failureMessage(caught: unknown): string {
  if (caught instanceof WorkerFailure) return errorCopy(caught.report.code, caught.report.message);
  return caught instanceof Error ? caught.message : "Local processing failed.";
}

/**
 * Inspects, plans, and (if the person agrees) fits one video. Returns the file the page should
 * get: the validated output, or the original when it already fits or the person keeps it.
 */
async function fitOne(file: File): Promise<File | "cancel"> {
  const destination = FAMILY_LABEL[family];
  const checking = `Checking for ${destination}`;
  const onProgress = (title: string) => (update: ProgressUpdate) =>
    show({ title, name: file.name, detail: update.stage, progress: update.percent });
  working = true;
  show({ title: checking, name: file.name, facts: formatBytes(file.size), progress: 4 });
  try {
    const { report: artifact } = await client.mediaInspect<DesktopArtifact>(file, onProgress(checking));
    for (;;) {
      working = true;
      const plan: DiscordCap = family === "discord" ? await loadPlan() : "free";
      const cap = videoCap(family, plan);
      // A plan change can make the held file fit as it is; then the page gets it untouched.
      if (held && file.size <= cap) return file;
      const profileId = videoProfile(family, plan);
      const { report: constraints } = await client.compileProfile<ConstraintSet>(profileId);
      const constraintsJson = JSON.stringify(constraints);
      const { report: planned } = await client.mediaPlan<MediaPlanReport>(constraintsJson, onProgress(checking));
      if (planned.outcome.kind === "compatible") return file;
      const where = family === "discord" ? `${destination} ${DISCORD_CAPS[plan].label}` : destination;
      const facts = inspectMediaLine(artifact, formatBytes(file.size));
      const notice = staleNotice(profileId, destination);
      const refusal =
        planned.browser?.status === "refused"
          ? planned.browser.reason
          : planned.outcome.kind === "cannot_satisfy"
            ? planned.outcome.blocking?.map((item) => item.message).join(" ") ||
              `No change Fitifact can make gets this under ${destination}'s rules.`
            : null;
      // The size is the headline; any other failing rule gets its own row.
      const alsoFails = describeProblemsFromChecks({
        ...planned.report,
        compatible: true,
        checks: planned.report.checks.filter((check) => check.field !== "file.bytes"),
      });
      if (refusal) {
        show({ title: `Can't fit this for ${destination} here`, name: file.name, facts, detail: refusal, notice, original: true, plan: true });
      } else {
        show({
          title: `Too big for ${destination}`,
          name: file.name,
          facts,
          detail: `${where} takes videos up to ${capLabel(cap)}.`,
          rows: [
            ...(alsoFails.length ? [["Also fails", alsoFails.join(". ")] as [string, string]] : []),
            ["Change", describeMediaActions(planned.outcome.plan?.steps).join(". ")],
            ["Keeps", describeKept(null, planned.outcome.plan?.preserved ?? []).join(" · ") || "Nothing else changes"],
          ],
          notice,
          fit: `Fit for ${destination}`,
          original: true,
          plan: true,
        });
      }
      const choice = await ask();
      if (choice === "replan") continue;
      if (choice !== "fit") return choice === "original" ? file : "cancel";
      working = true;
      const fitting = `Fitting for ${destination}`;
      show({ title: fitting, name: file.name, detail: "Starting", progress: 10 });
      const name = outputDetails("mp4", file.name, false).name;
      const { report, output } = await client.mediaAdapt<DesktopAdaptResult>(constraintsJson, name, onProgress(fitting));
      if (report.status === "compatible") return file;
      // Only an output the engine re-inspected and validated ever leaves this panel.
      if (report.status === "adapted" && report.validation?.status === "pass" && report.validation.checks.compatible && output) {
        return new File([output], name, { type: "video/mp4" });
      }
      throw new WorkerFailure("validation_failure", {
        schema: "fitifact.error/v1",
        code: report.error?.code ?? "VALIDATION_FAILED",
        message: report.error?.message ?? "The new file didn't pass every check, so Fitifact kept nothing.",
      });
    }
  } catch (caught) {
    if (caught instanceof WorkerFailure && caught.state === "cancelled") return "cancel";
    show({ title: "Could not fit this video", name: file.name, detail: failureMessage(caught), original: true });
    return (await ask()) === "original" ? file : "cancel";
  }
}

function close() {
  held?.port.postMessage({ type: "close" });
}

async function fitHeld(hold: Held) {
  const files = [...hold.files];
  const fitted: Array<[File, File]> = [];
  for (const [step, index] of hold.over.entries()) {
    setText(ui.count, hold.over.length > 1 ? `${step + 1} of ${hold.over.length}` : undefined);
    const result = await fitOne(files[index]);
    if (result === "cancel") return close();
    if (result !== files[index]) fitted.push([files[index], result]);
    files[index] = result;
  }
  hold.port.postMessage({ type: "deliver", files });
  if (!fitted.length) return close();
  setText(ui.count, undefined);
  show({
    title: `Fits ${FAMILY_LABEL[family]} now`,
    name: fitted.map(([before, after]) => `${after.name} · ${formatBytes(before.size)} → ${formatBytes(after.size)}`).join("\n"),
    notice: "Verified: the new file matches the destination rules.",
    tone: "success",
    detail: "Added to your message.",
    cancel: "Done",
  });
  setTimeout(close, 5000);
}

function chooser() {
  ui.choose.hidden = false;
  show({
    title: "Fit a video",
    detail: "Pick where it's going, then choose a video or drop one here. Fitifact changes only what that destination needs.",
    cancel: null,
    plan: true,
  });
}

async function fitChosen(file: File) {
  if (working || choose || ui.choose.hidden) return;
  ui.choose.hidden = true;
  family = ui.destination.value as DestinationFamily;
  const result = await fitOne(file);
  working = false;
  if (result === "cancel") return chooser();
  const destination = FAMILY_LABEL[family];
  show(
    result === file
      ? {
          title: `Already fits ${destination}`,
          name: file.name,
          notice: "Verified: this file already matches the destination. Nothing was changed.",
          tone: "success",
          cancel: "Fit another",
        }
      : {
          title: `Fits ${destination} now`,
          name: result.name,
          facts: `${formatBytes(file.size)} → ${formatBytes(result.size)}`,
          notice: "Verified: the new file matches the destination rules.",
          tone: "success",
          download: result,
          cancel: "Fit another",
        },
  );
}

ui.fit.addEventListener("click", () => choose?.("fit"));
ui.original.addEventListener("click", () => choose?.("original"));
ui.cancel.addEventListener("click", () => {
  if (choose) return choose("cancel");
  if (held) return close();
  if (working) return client.cancel();
  chooser();
});
ui.plan.addEventListener("change", async () => {
  await savePlan(ui.plan.value as DiscordCap);
  choose?.("replan");
});

// Framed: tell the content script how tall the card is.
new ResizeObserver(() => held?.port.postMessage({ type: "size", height: ui.sheet.getBoundingClientRect().height })).observe(ui.sheet);

if (window.parent !== window) {
  window.addEventListener("message", (event: MessageEvent) => {
    const origin = new URL(event.origin);
    const site = origin.protocol === "https:" ? SITES[origin.hostname] : undefined;
    // Only the watched page this frame sits on can hand it files.
    if (held || !site || event.source !== window.parent) return;
    const data = event.data as { type?: string; files?: unknown; over?: unknown };
    const port = event.ports[0];
    if (data?.type !== "fitifact.hold" || !port || !Array.isArray(data.files) || !Array.isArray(data.over)) return;
    const files: unknown[] = data.files;
    if (!files.every((file) => file instanceof File)) return;
    family = site.family;
    held = {
      port,
      files: files as File[],
      over: data.over.filter((index): index is number => Number.isInteger(index) && index >= 0 && index < files.length),
      family,
    };
    void fitHeld(held);
  });
} else {
  document.documentElement.classList.add("page");
  ui.destination.replaceChildren(
    ...[...new Set(Object.values(SITES).map((site) => site.family))].map((value) => new Option(FAMILY_LABEL[value], value)),
  );
  ui.destination.addEventListener("change", () => {
    family = ui.destination.value as DestinationFamily;
    chooser();
  });
  ui.pick.addEventListener("click", () => ui.file.click());
  ui.file.addEventListener("change", () => {
    const file = ui.file.files?.[0];
    ui.file.value = "";
    if (file) void fitChosen(file);
  });
  document.addEventListener("dragover", (event) => event.preventDefault());
  document.addEventListener("drop", (event) => {
    event.preventDefault();
    const file = event.dataTransfer?.files[0];
    if (file) void fitChosen(file);
  });
  chooser();
}

void fillPlans(ui.plan);
