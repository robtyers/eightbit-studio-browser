import { MAX_CASSETTE_AUDIO_BYTES, MAX_FILE_BYTES, record } from "@eightbitstudio/browser-workbench/contracts";

self.addEventListener("message", async (event: MessageEvent<unknown>) => {
  try {
    if (!record(event.data) || !record(event.data.request) || !(event.data.bytes instanceof Uint8Array) || event.data.bytes.length > MAX_FILE_BYTES) throw new Error("Invalid worker request");
    const module: unknown = await import(/* @vite-ignore */ `${import.meta.env.BASE_URL}generated/dragon_core.js`);
    if (!record(module) || typeof module.default !== "function" || typeof module.browser_request !== "function") throw new Error("Invalid core module");
    await module.default({ module_or_path: `${import.meta.env.BASE_URL}generated/dragon_core_bg.wasm` });
    if (event.data.request.operation === "cassette-audio") {
      if (typeof module.cassette_audio !== "function") throw new Error("Invalid audio module");
      const audio: unknown = module.cassette_audio(JSON.stringify(event.data.request), event.data.bytes);
      if (!(audio instanceof Uint8Array) || audio.length < 44 || audio.length > MAX_CASSETTE_AUDIO_BYTES) throw new Error("Invalid audio response");
      self.postMessage({ schemaVersion: 1, ok: true, result: { audio } }, { transfer: [audio.buffer] });
      return;
    }
    const response: unknown = module.browser_request(JSON.stringify(event.data.request), event.data.bytes);
    if (typeof response !== "string" || response.length > 40 * 1024 * 1024) throw new Error("Invalid core response");
    const value: unknown = JSON.parse(response); self.postMessage(value);
  } catch {
    self.postMessage({ schemaVersion: 1, ok: false, message: "Cannot process this input with the browser core. Check and build again before exporting." });
  }
}, { once: true });
