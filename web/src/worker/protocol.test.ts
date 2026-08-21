import { describe, expect, it } from "vitest";
import { ImageWorkerClient } from "./client";
import type { WorkerFailure } from "./client";
import { isWorkerResponse, productStateForError, type WorkerRequest } from "./protocol";

class FakeWorker {
  messages: Array<{ request: WorkerRequest; transfer: Transferable[] }> = [];
  terminated = false;
  private messageListeners: Array<(event: MessageEvent<unknown>) => void> = [];
  private errorListeners: Array<(event: ErrorEvent) => void> = [];

  postMessage(request: WorkerRequest, transfer: Transferable[] = []) {
    this.messages.push({ request, transfer });
  }
  terminate() { this.terminated = true; }
  addEventListener(type: "message" | "error", listener: ((event: MessageEvent<unknown>) => void) | ((event: ErrorEvent) => void)) {
    if (type === "message") this.messageListeners.push(listener as (event: MessageEvent<unknown>) => void);
    else this.errorListeners.push(listener as (event: ErrorEvent) => void);
  }
  respond(data: unknown) { for (const listener of this.messageListeners) listener({ data } as MessageEvent<unknown>); }
}

describe("worker protocol", () => {
  it("maps engine failures into every explicit product failure class", () => {
    expect(productStateForError({ code: "NO_VALID_PLAN" })).toBe("cannot_satisfy");
    expect(productStateForError({ code: "VALIDATION_FAILED" })).toBe("validation_failure");
    expect(productStateForError({ code: "INSPECTION_LIMIT" })).toBe("resource_limit");
    expect(productStateForError({ code: "EXECUTION_CANCELLED" })).toBe("cancelled");
    expect(productStateForError({ code: "UNSUPPORTED_HEIC" })).toBe("unsupported_heic");
  });

  it("accepts only well-shaped worker responses", () => {
    expect(isWorkerResponse({ id: 1, type: "progress", stage: "Inspecting", percent: 30 })).toBe(true);
    expect(isWorkerResponse({ id: "1", type: "result", report: {} })).toBe(false);
    expect(isWorkerResponse({ id: 1, type: "unknown" })).toBe(false);
  });

  it("transfers the source buffer and resolves the real response envelope", async () => {
    const fake = new FakeWorker();
    const client = new ImageWorkerClient(() => fake);
    const buffer = new ArrayBuffer(8);
    const pending = client.analyze<{ schema: string }>("photo.png", buffer, "{}", () => undefined);
    expect(fake.messages[0].transfer).toEqual([buffer]);
    const id = fake.messages[0].request.id;
    fake.respond({ id, type: "result", report: { schema: "fitifact.web-plan/v1" } });
    await expect(pending).resolves.toEqual({ report: { schema: "fitifact.web-plan/v1" }, output: undefined });
  });

  it("terminates work and rejects outstanding operations on cancellation", async () => {
    const fake = new FakeWorker();
    const client = new ImageWorkerClient(() => fake);
    const pending = client.compile("JPEG");
    client.cancel();
    expect(fake.terminated).toBe(true);
    await expect(pending).rejects.toMatchObject({ state: "cancelled" } satisfies Partial<WorkerFailure>);
  });
});
