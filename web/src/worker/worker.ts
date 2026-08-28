/// <reference lib="webworker" />

import type { Artifact, ErrorReport, ImageKind, InspectReport } from "../types";
import type * as WasmEngine from "../wasm/fitifact_wasm.js";
import { classifyInput, refuseMessage } from "./magic";
import { productStateForError, type WorkerRequest, type WorkerResponse } from "./protocol";
import {
  EncodedResourceLimit,
  enterWasmWithEncodedLimit,
  readFileWithinLimit,
} from "./resource";
import { StaleConstraints, withMatchingConstraints } from "./snapshot";

type Engine = typeof WasmEngine;
type StoredSource =
  | { kind: "bytes"; value: Uint8Array; generation: number; encodedLength: number; maxEncodedBytes: number; constraintsSnapshot: string | null }
  | { kind: "rgba"; value: Uint8Array; width: number; height: number; generation: number; encodedLength: number; maxEncodedBytes: number; constraintsSnapshot: string | null };
interface ImageLimits {
  schema: "fitifact.image-limits/v1";
  max_encoded_bytes: number;
  max_decoded_pixels: number;
}

const scope: DedicatedWorkerGlobalScope = self as unknown as DedicatedWorkerGlobalScope;
let enginePromise: Promise<Engine> | null = null;
let source: StoredSource | null = null;
let sourceGeneration = 0;

class EngineFailure extends Error {
  constructor(readonly report: ErrorReport) {
    super(report.message);
  }
}

function post(response: WorkerResponse, transfers: Transferable[] = []) {
  scope.postMessage(response, transfers);
}

function progress(id: number, stage: string, percent: number) {
  post({ id, type: "progress", stage, percent });
}

async function engine(id: number): Promise<Engine> {
  if (!enginePromise) {
    progress(id, "Loading the local image engine", 8);
    enginePromise = import("../wasm/fitifact_wasm.js").then(async (module) => {
      await module.default();
      return module;
    });
  }
  return enginePromise;
}

function parseReport<T>(json: string): T {
  const report = JSON.parse(json) as T | ErrorReport;
  if (
    report &&
    typeof report === "object" &&
    "schema" in report &&
    report.schema === "fitifact.error/v1"
  ) {
    throw new EngineFailure(report as ErrorReport);
  }
  return report as T;
}

function localFailure(code: string, message: string): EngineFailure {
  return new EngineFailure({ schema: "fitifact.error/v1", code, message, details: {} });
}

function ownedArrayBuffer(bytes: Uint8Array): ArrayBuffer {
  if (
    bytes.buffer instanceof ArrayBuffer &&
    bytes.byteOffset === 0 &&
    bytes.byteLength === bytes.buffer.byteLength
  ) {
    return bytes.buffer;
  }
  const owned = new Uint8Array(bytes.byteLength);
  owned.set(bytes);
  return owned.buffer;
}

function requireCurrent(generation: number): StoredSource {
  if (!source || sourceGeneration !== generation || source.generation !== generation) {
    throw localFailure("EXECUTION_CANCELLED", "A newer image replaced this operation.");
  }
  return source;
}

function canonicalConstraints(wasm: Engine, constraintsJson: string): string {
  return JSON.stringify(parseReport(wasm.compile_constraints(constraintsJson)));
}

function enterSourceWasm<T>(selected: StoredSource, operation: () => T): T {
  try {
    return enterWasmWithEncodedLimit(
      selected.encodedLength,
      selected.maxEncodedBytes,
      operation,
    );
  } catch (error) {
    if (error instanceof EncodedResourceLimit) {
      throw localFailure("INSPECTION_LIMIT", error.message);
    }
    throw error;
  }
}

function requireConstraintsMatch<T>(stored: string | null, expected: string, operation: () => T): T {
  try {
    return withMatchingConstraints(stored, expected, operation);
  } catch (error) {
    if (error instanceof StaleConstraints) throw localFailure("INPUT_INVALID", error.message);
    throw error;
  }
}

async function pngPreviewFromRgba(
  rgba: Uint8Array,
  width: number,
  height: number,
): Promise<ArrayBuffer | undefined> {
  try {
    if (typeof OffscreenCanvas === "undefined") return undefined;
    const maxEdge = 512;
    const scale = Math.min(1, maxEdge / Math.max(width, height));
    const outWidth = Math.max(1, Math.round(width * scale));
    const outHeight = Math.max(1, Math.round(height * scale));
    const source = new OffscreenCanvas(width, height);
    const context = source.getContext("2d");
    if (!context) return undefined;
    const imageData = context.createImageData(width, height);
    imageData.data.set(rgba);
    context.putImageData(imageData, 0, 0);
    let canvas: OffscreenCanvas = source;
    if (outWidth !== width || outHeight !== height) {
      const scaled = new OffscreenCanvas(outWidth, outHeight);
      const scaledContext = scaled.getContext("2d");
      if (!scaledContext) return undefined;
      scaledContext.drawImage(source, 0, 0, outWidth, outHeight);
      canvas = scaled;
    }
    const blob = await canvas.convertToBlob({ type: "image/png" });
    return await blob.arrayBuffer();
  } catch {
    return undefined;
  }
}

function heicArtifact(bytes: Uint8Array, width: number, height: number): Artifact {
  return {
    schema: "fitifact.artifact/v1",
    byte_length: bytes.byteLength,
    family: "image",
    image: {
      format: "heif",
      width,
      height,
      alpha: true,
      animated: false,
    },
  };
}

async function inspect(request: Extract<WorkerRequest, { type: "inspect" }>) {
  const generation = ++sourceGeneration;
  source = null;
  try {
    const wasm = await engine(request.id);
    if (generation !== sourceGeneration) {
      throw localFailure("EXECUTION_CANCELLED", "A newer image replaced this operation.");
    }
    const limits = parseReport<ImageLimits>(wasm.image_limits());
    let buffer: ArrayBuffer;
    try {
      buffer = await readFileWithinLimit(request.file, limits.max_encoded_bytes);
    } catch (error) {
      if (error instanceof EncodedResourceLimit) {
        throw localFailure("INSPECTION_LIMIT", error.message);
      }
      throw error;
    }
    if (generation !== sourceGeneration) {
      throw localFailure("EXECUTION_CANCELLED", "A newer image replaced this operation.");
    }
    const bytes = new Uint8Array(buffer);
    const kind = classifyInput(bytes);
    progress(request.id, "Checking the file type", 18);
    if (kind === "video" || kind === "matroska" || kind === "pdf" || kind === "zip" || kind === "unsupported") {
      throw localFailure("INSPECTION_UNSUPPORTED", refuseMessage(kind));
    }
    if (kind === "heic") {
      if (!__FITIFACT_HEIC_APPROVED__) {
        throw localFailure(
          "UNSUPPORTED_HEIC",
          "This is a phone photo this build cannot decode yet.",
        );
      }
      progress(request.id, "Loading the approved HEIC decoder", 28);
      const { decodeSingleHeic, HeicDecodeFailure } = await import("./heic-decoder");
      let decoded;
      try {
        decoded = await decodeSingleHeic(bytes, limits.max_decoded_pixels);
      } catch (error) {
        if (error instanceof HeicDecodeFailure) throw localFailure(error.code, error.message);
        throw error;
      }
      if (generation !== sourceGeneration) {
        throw localFailure("EXECUTION_CANCELLED", "A newer image replaced this operation.");
      }
      source = {
        kind: "rgba",
        value: decoded.rgba,
        width: decoded.width,
        height: decoded.height,
        generation,
        encodedLength: bytes.byteLength,
        maxEncodedBytes: limits.max_encoded_bytes,
        constraintsSnapshot: null,
      };
      progress(request.id, "Inspection ready", 100);
      const report: InspectReport = {
        schema: "fitifact.inspect/v1",
        kind,
        artifact: heicArtifact(bytes, decoded.width, decoded.height),
      };
      const preview = await pngPreviewFromRgba(decoded.rgba, decoded.width, decoded.height);
      return preview ? { report, preview } : { report };
    }
    const artifact = parseReport<Artifact>(enterWasmWithEncodedLimit(
      bytes.byteLength,
      limits.max_encoded_bytes,
      () => wasm.inspect_bytes(bytes),
    ));
    source = {
      kind: "bytes",
      value: bytes,
      generation,
      encodedLength: bytes.byteLength,
      maxEncodedBytes: limits.max_encoded_bytes,
      constraintsSnapshot: null,
    };
    progress(request.id, "Inspection ready", 100);
    return { report: { schema: "fitifact.inspect/v1", kind: kind as ImageKind, artifact } satisfies InspectReport };
  } catch (error) {
    if (sourceGeneration === generation) source = null;
    throw error;
  }
}

async function plan(
  id: number,
  constraintsJson: string,
  generation = sourceGeneration,
  expectedSnapshot?: string,
) {
  const selected = requireCurrent(generation);
  const wasm = await engine(id);
  requireCurrent(generation);
  if (expectedSnapshot !== undefined) {
    requireConstraintsMatch(selected.constraintsSnapshot, expectedSnapshot, () => undefined);
  }
  const constraintsSnapshot = enterSourceWasm(selected, () =>
    canonicalConstraints(wasm, constraintsJson),
  );
  progress(id, "Inspecting and planning minimum changes", 55);
  if (selected.kind === "bytes") {
    const report = parseReport(enterSourceWasm(selected, () =>
      wasm.plan_bytes(selected.value, constraintsSnapshot),
    ));
    requireCurrent(generation);
    selected.constraintsSnapshot = constraintsSnapshot;
    progress(id, "Plan ready for review", 100);
    return { report, constraintsSnapshot };
  }
  const result = enterSourceWasm(selected, () => wasm.plan_rgba(
    selected.value, selected.width, selected.height, constraintsSnapshot,
  ));
  try {
    const report = parseReport(result.report_json);
    requireCurrent(generation);
    const previewBytes = result.take_preview();
    const preview = previewBytes ? ownedArrayBuffer(previewBytes) : undefined;
    selected.constraintsSnapshot = constraintsSnapshot;
    progress(id, "Plan ready for review", 100);
    return { report, preview, constraintsSnapshot };
  } finally {
    result.free();
  }
}

async function adapt(request: Extract<WorkerRequest, { type: "adapt" }>) {
  const generation = sourceGeneration;
  const selected = requireCurrent(generation);
  const wasm = await engine(request.id);
  requireCurrent(generation);
  const constraintsSnapshot = enterSourceWasm(selected, () =>
    canonicalConstraints(wasm, request.constraintsJson),
  );
  requireConstraintsMatch(selected.constraintsSnapshot, constraintsSnapshot, () => undefined);
  const options = JSON.stringify({
    crop: request.crop,
    crop_consent: request.crop !== null,
    first_frame_consent: request.firstFrameConsent,
  });
  progress(request.id, "Applying the approved plan locally", 40);
  const result =
    selected.kind === "bytes"
      ? enterSourceWasm(selected, () => wasm.adapt_bytes(selected.value, constraintsSnapshot, options))
      : enterSourceWasm(selected, () => wasm.adapt_rgba(
          selected.value,
          selected.width,
          selected.height,
          constraintsSnapshot,
          options,
        ));
  try {
    const report = parseReport(result.report_json);
    requireCurrent(generation);
    progress(request.id, "Validating every requirement", 82);
    const output = result.take_output();
    progress(request.id, "Validation complete", 100);
    if (!output) return { report };
    return { report, output: ownedArrayBuffer(output) };
  } finally {
    result.free();
  }
}

scope.addEventListener("message", (event: MessageEvent<WorkerRequest>) => {
  const request = event.data;
  void (async () => {
    try {
      if (request.type === "compile") {
        const wasm = await engine(request.id);
        progress(request.id, "Reading the requirement", 55);
        const report = parseReport(wasm.compile_requirements(request.requirements));
        post({ id: request.id, type: "result", report });
      } else if (request.type === "compile_constraints") {
        const wasm = await engine(request.id);
        progress(request.id, "Checking the edited target", 55);
        const report = parseReport(wasm.compile_constraints(request.constraintsJson));
        post({ id: request.id, type: "result", report });
      } else if (request.type === "compile_profile") {
        const wasm = await engine(request.id);
        progress(request.id, "Loading the destination profile", 55);
        const report = parseReport(wasm.compile_profile(request.profileId));
        post({ id: request.id, type: "result", report });
      } else if (request.type === "inspect") {
        const result = await inspect(request);
        const transfers = result.preview ? [result.preview] : [];
        post({ id: request.id, type: "result", ...result }, transfers);
      } else if (request.type === "plan") {
        const result = await plan(request.id, request.constraintsJson);
        const transfers = result.preview ? [result.preview] : [];
        post({ id: request.id, type: "result", ...result }, transfers);
      } else if (request.type === "replan") {
        const result = await plan(
          request.id,
          request.constraintsJson,
          sourceGeneration,
          request.previousConstraintsJson,
        );
        const transfers = result.preview ? [result.preview] : [];
        post({ id: request.id, type: "result", ...result }, transfers);
      } else {
        const result = await adapt(request);
        if ("output" in result && result.output) {
          post({ id: request.id, type: "result", report: result.report, output: result.output }, [
            result.output,
          ]);
        } else {
          post({ id: request.id, type: "result", report: result.report });
        }
      }
    } catch (error) {
      const report =
        error instanceof EngineFailure
          ? error.report
          : ({
              schema: "fitifact.error/v1",
              code: "EXECUTION_FAILED",
              message: error instanceof Error ? error.message : "Local processing failed.",
              details: {},
            } satisfies ErrorReport);
      post({ id: request.id, type: "failure", state: productStateForError(report), report });
    }
  })();
});
