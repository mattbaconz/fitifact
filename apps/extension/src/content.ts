// Holds a video that is over this site's cap before the page sees it, and hands
// the page whatever the panel returns: a validated fit, or the original.
import type { contentSites } from "./sites";

declare const __FITIFACT_SITES__: ReturnType<typeof contentSites>;

const site = __FITIFACT_SITES__[location.hostname];
const panelUrl = chrome.runtime.getURL("panel.html");
let plan = "free";

const VIDEO_NAME = /\.(mp4|m4v|mov|mkv|webm)$/i;

/** Which files look like video over the cap. A hint, not a verdict: the panel reads the bytes. */
function overCap(files: readonly File[]): number[] {
  const cap = site.caps[plan] ?? site.caps.free;
  return files.flatMap((file, index) =>
    (file.type.startsWith("video/") || (!file.type && VIDEO_NAME.test(file.name))) && file.size > cap ? [index] : [],
  );
}

let replaying = false;

function replay(dispatch: () => void) {
  replaying = true;
  try {
    dispatch();
  } finally {
    replaying = false;
  }
}

function heldInput(event: Event): HTMLInputElement | null {
  const input = event.target;
  if (replaying || !(input instanceof HTMLInputElement) || input.type !== "file" || !input.files) return null;
  if (site.videoInputsOnly && !/video/i.test(input.accept)) return null;
  return overCap([...input.files]).length ? input : null;
}

function transfer(files: readonly File[]): DataTransfer {
  const data = new DataTransfer();
  for (const file of files) data.items.add(file);
  return data;
}

function watch() {
  // `input` fires just before `change`; the page reads neither until the panel answers.
  window.addEventListener("input", (event) => {
    if (heldInput(event)) event.stopImmediatePropagation();
  }, true);

  window.addEventListener("change", (event) => {
    const input = heldInput(event);
    if (!input) return;
    event.stopImmediatePropagation();
    hold([...input.files!], (files) => {
      input.files = transfer(files).files;
      replay(() => {
        input.dispatchEvent(new Event("input", { bubbles: true, composed: true }));
        input.dispatchEvent(new Event("change", { bubbles: true }));
      });
    }, () => {
      input.value = "";
    });
  }, true);

  window.addEventListener("drop", (event) => {
    const files = [...(event.dataTransfer?.files ?? [])];
    const target = event.target;
    if (replaying || !(target instanceof Node) || !overCap(files).length) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    const { clientX, clientY, screenX, screenY } = event;
    hold(files, (fitted) => {
      const dataTransfer = transfer(fitted);
      replay(() =>
        target.dispatchEvent(
          new DragEvent("drop", { bubbles: true, cancelable: true, composed: true, clientX, clientY, screenX, screenY, dataTransfer }),
        ),
      );
    });
  }, true);
}

let dismiss: (() => void) | null = null;

/** Opens the panel with the held files; `deliver` runs at most once, with what the page should get. */
function hold(files: File[], deliver: (files: File[]) => void, abandon?: () => void) {
  dismiss?.();
  const frame = document.createElement("iframe");
  const channel = new MessageChannel();
  let delivered = false;
  const close = () => {
    channel.port1.close();
    // Fade out, then remove, so the click that closed the panel finishes inside it first.
    frame.style.opacity = "0";
    frame.style.pointerEvents = "none";
    setTimeout(() => frame.remove(), 160);
    if (dismiss === cancel) dismiss = null;
    if (!delivered) abandon?.();
  };
  const cancel = () => close();
  dismiss = cancel;
  channel.port1.onmessage = ({ data }) => {
    if (data?.type === "size") frame.style.height = `${Math.ceil(Number(data.height))}px`;
    else if (data?.type === "deliver" && !delivered) {
      delivered = true;
      deliver(data.files as File[]);
    } else if (data?.type === "close") close();
  };
  frame.addEventListener("load", () => {
    frame.contentWindow?.postMessage(
      { type: "fitifact.hold", files, over: overCap(files) },
      new URL(panelUrl).origin,
      [channel.port2],
    );
  }, { once: true });
  frame.src = panelUrl;
  frame.title = "Fitifact";
  Object.assign(frame.style, {
    position: "fixed",
    top: "16px",
    right: "16px",
    zIndex: "2147483647",
    width: "min(400px, calc(100vw - 32px))",
    height: "220px",
    border: "0",
    borderRadius: "12px",
    boxShadow: "0 18px 48px rgb(0 0 0 / 0.42)",
    colorScheme: "normal",
    transition: matchMedia("(prefers-reduced-motion: reduce)").matches ? "none" : "opacity 140ms ease",
  });
  document.documentElement.append(frame);
}

if (site) {
  void chrome.storage.local.get("discordPlan").then(({ discordPlan }) => {
    if (typeof discordPlan === "string") plan = discordPlan;
  });
  chrome.storage.onChanged.addListener((changes) => {
    if (changes.discordPlan) plan = String(changes.discordPlan.newValue ?? "free");
  });
  watch();
}
