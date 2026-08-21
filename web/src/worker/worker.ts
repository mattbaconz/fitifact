/// <reference lib="webworker" />

import type { ErrorReport } from "../types";
import type * as WasmEngine from "../wasm/fitifact_wasm.js";
import { classifyInput } from "./magic";
import { productStateForError, type WorkerRequest, type WorkerResponse } from "./protocol";
import {
  EncodedResourceLimit,
  enterWasmWithEncodedLimit,
  readFileWithinLimit,
} from "./resource";
import { StaleConstraints, withMatchingConstraints } from "./snapshot";

type Engine = typeof WasmEngine;
type StoredSource =
  | { kind: "bytes"; value: Uint8Array; generation: number; encodedLength: number; maxEncodedBytes: number; constraintsSnapshot: string }
  | { kind: "rgba"; value: Uint8Array; width: number; height: number; generation: number; encodedLength: number; maxEncodedBytes: number; constraintsSnapshot: string };
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

function requireConstraintsMatch<T>(stored: string, expected: string, operation: () => T): T {
  try {
    return withMatchingConstraints(stored, expected, operation);
  } catch (error) {
    if (error instanceof StaleConstraints) throw localFailure("INPUT_INVALID", error.message);
    throw error;
  }
}

async function analyze(request: Extract<WorkerRequest, { type: "analyze" }>) {
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
    const constraintsSnapshot = enterWasmWithEncodedLimit(
      bytes.byteLength,
      limits.max_encoded_bytes,
      () => canonicalConstraints(wasm, request.constraintsJson),
    );
    const kind = classifyInput(bytes);
    progress(request.id, "Checking the file type", 18);
    if (kind === "unsupported") {
      throw localFailure(
        "INSPECTION_UNSUPPORTED",
        "This file is not a supported JPEG or PNG. SVG and HTML are never rendered.",
      );
    }
    if (kind === "heic") {
      if (!__FITIFACT_HEIC_APPROVED__) {
        throw localFailure(
          "UNSUPPORTED_HEIC",
          "HEIC was detected, but this build has not approved the optional local decoder.",
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
        constraintsSnapshot,
      };
    } else {
      source = {
        kind: "bytes",
        value: bytes,
        generation,
        encodedLength: bytes.byteLength,
        maxEncodedBytes: limits.max_encoded_bytes,
        constraintsSnapshot,
      };
    }
    return await plan(request.id, constraintsSnapshot, generation, constraintsSnapshot);
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
  const options = JSON.stringify({ crop: request.crop, crop_consent: request.crop !== null });
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
      } else if (request.type === "analyze") {
        const result = await analyze(request);
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
