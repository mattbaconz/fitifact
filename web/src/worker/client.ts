import type { CropRectangle, ErrorReport, ProductState } from "../types";
import { isWorkerResponse, type WorkerRequest, type WorkerResponse } from "./protocol";

export interface WorkerResult<T> {
  report: T;
  output?: ArrayBuffer;
  preview?: ArrayBuffer;
  constraintsSnapshot?: string;
}

export interface ProgressUpdate {
  stage: string;
  percent: number;
}

export class WorkerFailure extends Error {
  constructor(
    readonly state: ProductState,
    readonly report: ErrorReport,
  ) {
    super(report.message);
  }
}

interface WorkerPort {
  postMessage(message: WorkerRequest, transfer?: Transferable[]): void;
  terminate(): void;
  addEventListener(type: "message", listener: (event: MessageEvent<unknown>) => void): void;
  addEventListener(type: "error", listener: (event: ErrorEvent) => void): void;
}

interface Pending {
  resolve: (result: WorkerResult<unknown>) => void;
  reject: (error: WorkerFailure) => void;
  onProgress?: (progress: ProgressUpdate) => void;
}

type RequestWithoutId = WorkerRequest extends infer Request
  ? Request extends { id: number }
    ? Omit<Request, "id">
    : never
  : never;

export class ImageWorkerClient {
  private worker: WorkerPort | null = null;
  private nextId = 1;
  private readonly pending = new Map<number, Pending>();

  constructor(
    private readonly createWorker: () => WorkerPort = () =>
      new Worker(new URL("./worker.ts", import.meta.url), { type: "module", name: "fitifact-image" }),
  ) {}

  compile<T>(requirements: string, onProgress?: (progress: ProgressUpdate) => void) {
    return this.call<T>({ type: "compile", requirements }, [], onProgress);
  }

  compileConstraints<T>(constraintsJson: string, onProgress?: (progress: ProgressUpdate) => void) {
    return this.call<T>({ type: "compile_constraints", constraintsJson }, [], onProgress);
  }

  inspect<T>(file: File, onProgress?: (progress: ProgressUpdate) => void) {
    return this.call<T>({ type: "inspect", file }, [], onProgress);
  }

  plan<T>(constraintsJson: string, onProgress?: (progress: ProgressUpdate) => void) {
    return this.call<T>({ type: "plan", constraintsJson }, [], onProgress);
  }

  replan<T>(previousConstraintsJson: string, constraintsJson: string, onProgress?: (progress: ProgressUpdate) => void) {
    return this.call<T>({ type: "replan", previousConstraintsJson, constraintsJson }, [], onProgress);
  }

  adapt<T>(
    constraintsJson: string,
    crop: CropRectangle | null,
    firstFrameConsent: boolean,
    onProgress?: (progress: ProgressUpdate) => void,
  ) {
    return this.call<T>({ type: "adapt", constraintsJson, crop, firstFrameConsent }, [], onProgress);
  }

  cancel() {
    if (!this.worker) return;
    this.worker.terminate();
    this.worker = null;
    const report: ErrorReport = {
      schema: "fitifact.error/v1",
      code: "EXECUTION_CANCELLED",
      message: "Local processing was cancelled. No output was saved.",
      details: {},
    };
    for (const pending of this.pending.values()) {
      pending.reject(new WorkerFailure("cancelled", report));
    }
    this.pending.clear();
  }

  dispose() {
    this.cancel();
  }

  private call<T>(
    request: RequestWithoutId,
    transfer: Transferable[],
    onProgress?: (progress: ProgressUpdate) => void,
  ): Promise<WorkerResult<T>> {
    const worker = this.ensureWorker();
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      this.pending.set(id, {
        resolve: resolve as (result: WorkerResult<unknown>) => void,
        reject,
        onProgress,
      });
      worker.postMessage({ ...request, id } as WorkerRequest, transfer);
    });
  }

  private ensureWorker(): WorkerPort {
    if (this.worker) return this.worker;
    const worker = this.createWorker();
    worker.addEventListener("message", (event) => this.receive(event.data));
    worker.addEventListener("error", () => {
      const report: ErrorReport = {
        schema: "fitifact.error/v1",
        code: "EXECUTION_FAILED",
        message: "The local image worker stopped unexpectedly.",
        details: {},
      };
      for (const pending of this.pending.values()) {
        pending.reject(new WorkerFailure("error", report));
      }
      this.pending.clear();
      worker.terminate();
      if (this.worker === worker) this.worker = null;
    });
    this.worker = worker;
    return worker;
  }

  private receive(value: unknown) {
    if (!isWorkerResponse(value)) return;
    const pending = this.pending.get(value.id);
    if (!pending) return;
    if (value.type === "progress") {
      pending.onProgress?.({ stage: value.stage, percent: value.percent });
      return;
    }
    this.pending.delete(value.id);
    if (value.type === "failure") {
      pending.reject(new WorkerFailure(value.state, value.report));
    } else {
      pending.resolve({
        report: value.report,
        output: value.output,
        preview: value.preview,
        constraintsSnapshot: value.constraintsSnapshot,
      });
    }
  }
}

export type { WorkerResponse };
