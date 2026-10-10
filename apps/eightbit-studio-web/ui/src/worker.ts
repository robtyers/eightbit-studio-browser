import { MAX_FILE_BYTES, record } from "@eightbitstudio/browser-workbench/contracts";

self.addEventListener("message", async (event: MessageEvent<unknown>) => {
  try {
    if (!record(event.data) || !record(event.data.request) || !(event.data.bytes instanceof Uint8Array) || event.data.bytes.length > MAX_FILE_BYTES) throw new Error("Invalid worker request");
    const module: unknown = await import(/* @vite-ignore */ `${import.meta.env.BASE_URL}generated/dragon_core.js`);
    if (!record(module) || typeof module.default !== "function" || typeof module.browser_request !== "function") throw new Error("Invalid core module");
    await module.default({ module_or_path: `${import.meta.env.BASE_URL}generated/dragon_core_bg.wasm` });
    const response: unknown = module.browser_request(JSON.stringify(event.data.request), event.data.bytes);
    if (typeof response !== "string" || response.length > 40 * 1024 * 1024) throw new Error("Invalid core response");
    const value: unknown = JSON.parse(response); self.postMessage(value);
  } catch {
    self.postMessage({ schemaVersion: 1, ok: false, message: "Cannot process this input with the browser core." });
  }
}, { once: true });
