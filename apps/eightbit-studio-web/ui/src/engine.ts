import type { BrowserEngine, BrowserRequest } from "@eightbitstudio/browser-workbench/contracts";

interface WorkerPort {
  onmessage: ((event: MessageEvent<unknown>) => void) | null;
  onerror: ((event: ErrorEvent) => void) | null;
  postMessage(message: unknown): void;
  terminate(): void;
}
/** A disposable worker bounds each request and can stop synchronous WASM work. */
export class WorkerEngine implements BrowserEngine {
  constructor(private readonly createWorker: () => WorkerPort = () => new Worker(new URL("./worker.ts", import.meta.url), { type: "module" }), private readonly timeoutMs = 20_000) {}
  request(request: BrowserRequest, bytes: Uint8Array, signal: AbortSignal): Promise<unknown> {
    return new Promise((resolve, reject) => {
      if (signal.aborted) { reject(new DOMException("Cancelled", "AbortError")); return; }
      const worker = this.createWorker();
      let settled = false;
      const finish = (error?: Error, value?: unknown) => {
        if (settled) return;
        settled = true; clearTimeout(timer); signal.removeEventListener("abort", abort); worker.terminate();
        if (error) reject(error); else resolve(value);
      };
      const abort = () => finish(new DOMException("Cancelled", "AbortError"));
      const timer = setTimeout(() => finish(new Error("Browser operation timed out. Try a smaller input.")), this.timeoutMs);
      signal.addEventListener("abort", abort, { once: true });
      worker.onmessage = (event) => finish(undefined, event.data);
      worker.onerror = () => finish(new Error("Cannot load the browser core. Check the static assets and try again."));
      try { worker.postMessage({ request, bytes }); }
      catch { finish(new Error("Cannot start the browser operation.")); }
    });
  }
}
