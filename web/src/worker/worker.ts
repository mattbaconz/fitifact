/// <reference lib="webworker" />

import type { ErrorReport } from "../types";
import type * as WasmEngine from "../wasm/fitifact_wasm.js";
import { classifyInput } from "./magic";
import { productStateForError, type WorkerRequest, type WorkerResponse } from "./protocol";

type Engine = typeof WasmEngine;
type StoredSource =
  | { kind: "bytes"; value: Uint8Array }
  | { kind: "rgba"; value: Uint8Array; width: number; height: number };

const scope: DedicatedWorkerGlobalScope = self as unknown as DedicatedWorkerGlobalScope;
const MAX_INPUT_BYTES = 32 * 1024 * 1024;
let enginePromise: Promise<Engine> | null = null;
let source: StoredSource | null = null;

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

async function analyze(request: Extract<WorkerRequest, { type: "analyze" }>) {
  const bytes = new Uint8Array(request.buffer);
  if (bytes.byteLength > MAX_INPUT_BYTES) {
    throw localFailure("INSPECTION_LIMIT", "This file exceeds the 32 MiB local input limit.");
  }
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
    const { decodeSingleHeic } = await import("./heic-decoder");
    const decoded = await decodeSingleHeic(bytes);
    source = { kind: "rgba", value: decoded.rgba, width: decoded.width, height: decoded.height };
  } else {
    source = { kind: "bytes", value: bytes };
  }
  return plan(request.id, request.constraintsJson);
}

async function plan(id: number, constraintsJson: string): Promise<unknown> {
  if (!source) throw localFailure("INPUT_INVALID", "Choose an image before planning.");
  const wasm = await engine(id);
  progress(id, "Inspecting and planning minimum changes", 55);
  const json =
    source.kind === "bytes"
      ? wasm.plan_bytes(source.value, constraintsJson)
      : wasm.plan_rgba(source.value, source.width, source.height, constraintsJson);
  const report = parseReport(json);
  progress(id, "Plan ready for review", 100);
  return report;
}

async function adapt(request: Extract<WorkerRequest, { type: "adapt" }>) {
  if (!source) throw localFailure("INPUT_INVALID", "Choose an image before adapting.");
  const wasm = await engine(request.id);
  const options = JSON.stringify({ crop: request.crop, crop_consent: request.crop !== null });
  progress(request.id, "Applying the approved plan locally", 40);
  const result =
    source.kind === "bytes"
      ? wasm.adapt_bytes(source.value, request.constraintsJson, options)
      : wasm.adapt_rgba(
          source.value,
          source.width,
          source.height,
          request.constraintsJson,
          options,
        );
  try {
    const report = parseReport(result.report_json);
    progress(request.id, "Validating every requirement", 82);
    const output = result.take_output();
    progress(request.id, "Validation complete", 100);
    if (!output) return { report };
    let buffer: ArrayBuffer;
    if (
      output.buffer instanceof ArrayBuffer &&
      output.byteOffset === 0 &&
      output.byteLength === output.buffer.byteLength
    ) {
      buffer = output.buffer;
    } else {
      const owned = new Uint8Array(output.byteLength);
      owned.set(output);
      buffer = owned.buffer;
    }
    return { report, output: buffer };
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
      } else if (request.type === "analyze") {
        post({ id: request.id, type: "result", report: await analyze(request) });
      } else if (request.type === "replan") {
        post({ id: request.id, type: "result", report: await plan(request.id, request.constraintsJson) });
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
