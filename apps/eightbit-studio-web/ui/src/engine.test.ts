import { WorkerEngine } from "./engine";
import type { BrowserRequest } from "@eightbitstudio/browser-workbench/contracts";
const REQUEST: BrowserRequest = { schemaVersion: 1, operation: "build", source: "10 END\n" };
function fixture() {
  const port = { onmessage: null as ((event: MessageEvent<unknown>) => void) | null, onerror: null as ((event: ErrorEvent) => void) | null, postMessage: vi.fn(), terminate: vi.fn() };
  return { port, engine: new WorkerEngine(() => port, 100) };
}
it("terminates after one response and ignores duplicates", async () => {
  const { port, engine } = fixture();
  const pending = engine.request(REQUEST, new Uint8Array(), new AbortController().signal);
  port.onmessage?.(new MessageEvent("message", { data: { schemaVersion: 1, ok: true } }));
  await expect(pending).resolves.toEqual({ schemaVersion: 1, ok: true });
  port.onmessage?.(new MessageEvent("message", { data: "late" })); expect(port.terminate).toHaveBeenCalledTimes(1);
});
it("terminates on cancellation and refuses already cancelled work", async () => {
  const { port, engine } = fixture(); const controller = new AbortController();
  const pending = engine.request(REQUEST, new Uint8Array(), controller.signal); controller.abort();
  await expect(pending).rejects.toMatchObject({ name: "AbortError" }); expect(port.terminate).toHaveBeenCalledOnce();
  await expect(engine.request(REQUEST, new Uint8Array(), controller.signal)).rejects.toMatchObject({ name: "AbortError" });
  expect(port.postMessage).toHaveBeenCalledTimes(1);
});
it("times out hung WASM work and can recover with a fresh worker", async () => {
  vi.useFakeTimers();
  try {
    const { port, engine } = fixture();
    const pending = expect(engine.request(REQUEST, new Uint8Array(), new AbortController().signal)).rejects.toThrow("timed out");
    await vi.advanceTimersByTimeAsync(101); await pending; expect(port.terminate).toHaveBeenCalledOnce();
    const recovered = engine.request(REQUEST, new Uint8Array(), new AbortController().signal);
    port.onmessage?.(new MessageEvent("message", { data: "recovered" })); await expect(recovered).resolves.toBe("recovered");
  } finally { vi.useRealTimers(); }
});
