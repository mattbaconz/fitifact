const drop = document.getElementById("drop");
const pick = document.getElementById("pick");
const fileInput = document.getElementById("file");
const logEl = document.getElementById("log");
const preview = document.getElementById("preview");

let engine = null;
let previewUrl = null;

function log(text, className) {
  logEl.className = className || "";
  logEl.textContent = text;
}

function sniff(bytes) {
  if (
    bytes.length >= 8 &&
    bytes[0] === 0x89 &&
    bytes[1] === 0x50 &&
    bytes[2] === 0x4e &&
    bytes[3] === 0x47
  ) {
    return "png";
  }
  if (bytes.length >= 3 && bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff) {
    return "jpeg";
  }
  return "other";
}

function revokePreview() {
  if (previewUrl) {
    URL.revokeObjectURL(previewUrl);
    previewUrl = null;
  }
  preview.removeAttribute("src");
  preview.style.display = "none";
}

function showPreview(file, kind) {
  revokePreview();
  if (kind !== "jpeg" && kind !== "png") {
    return;
  }
  previewUrl = URL.createObjectURL(file);
  preview.src = previewUrl;
  preview.style.display = "block";
}

async function loadEngine() {
  try {
    const mod = await import("./pkg/fitifact_wasm.js");
    await mod.default();
    engine = mod;
    log("Ready. Drop a JPEG or PNG.\nTarget: JPEG.\nMedia runtime: not loaded.");
  } catch {
    engine = null;
    log(
      "The local WASM module is not built yet.\nFrom the repository root:\n\nwasm-pack build crates/fitifact-wasm --target web --out-dir ../../web/pkg\n\nRUSTFLAGS may need --cfg getrandom_backend=\"wasm_js\" on wasm32.",
      "warn"
    );
  }
}

function downloadJpeg(bytes) {
  const blob = new Blob([bytes], { type: "image/jpeg" });
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = "fitifact.jpg";
  link.className = "button";
  link.textContent = "Save JPEG";
  logEl.append("\n");
  logEl.append(link);
  link.addEventListener(
    "click",
    () => {
      window.setTimeout(() => URL.revokeObjectURL(url), 0);
    },
    { once: true }
  );
}

async function handleFile(file) {
  if (!engine) {
    log("Build the local WASM module before adapting files.", "warn");
    return;
  }
  drop.classList.add("busy");
  try {
    const buffer = new Uint8Array(await file.arrayBuffer());
    const kind = sniff(buffer);
    showPreview(file, kind);
    const inspected = JSON.parse(engine.inspect_bytes(buffer));
    if (inspected.schema === "fitifact.error/v1") {
      log(`${inspected.code}\n${inspected.message}`, "warn");
      return;
    }
    const checked = JSON.parse(engine.check_bytes(buffer));
    const planned = JSON.parse(engine.plan_bytes(buffer));
    const adapted = engine.adapt_bytes(buffer);
    const report = JSON.parse(adapted.report_json);
    const lines = [
      `File: ${file.name}`,
      `Family: ${inspected.family}`,
      `Format: ${inspected.image && inspected.image.format}`,
      `Check: ${checked.compatible ? "already fits" : "needs a change"}`,
      `Plan: ${planned.kind || planned.code}`,
      `Adapt: ${report.status}`,
      `Media runtime loaded: ${report.media_runtime_loaded}`,
      `Uploads to Fitifact: 0 bytes`,
    ];
    if (report.explanation && report.explanation.summary) {
      lines.push(report.explanation.summary);
    }
    if (report.error && report.error.message) {
      lines.push(report.error.message);
    }
    log(lines.join("\n"), report.status === "cannot_satisfy" || report.status === "failed" ? "warn" : "ok");
    if (report.status === "adapted" && adapted.output) {
      downloadJpeg(adapted.output);
    }
  } catch (err) {
    log(String(err), "warn");
  } finally {
    drop.classList.remove("busy");
  }
}

pick.addEventListener("click", () => fileInput.click());
fileInput.addEventListener("change", () => {
  const file = fileInput.files && fileInput.files[0];
  if (file) {
    handleFile(file);
  }
  fileInput.value = "";
});
drop.addEventListener("dragover", (event) => {
  event.preventDefault();
});
drop.addEventListener("drop", (event) => {
  event.preventDefault();
  const file = event.dataTransfer && event.dataTransfer.files && event.dataTransfer.files[0];
  if (file) {
    handleFile(file);
  }
});

loadEngine();
