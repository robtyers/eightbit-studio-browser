import { originalRuntimeCapability } from "@eightbitstudio/workbench/services/originalRuntime";
import { MONITOR_REQUEST_LIMIT, MONITOR_RESPONSE_LIMIT } from "@eightbitstudio/workbench/services/monitorProtocol";
import { ORIGINAL_DEBUG_REQUEST_LIMIT, ORIGINAL_DEBUG_RESPONSE_LIMIT } from "@eightbitstudio/workbench/services/originalBasicDebug";
import { record, machineMediaIdentity, MAX_FILE_BYTES } from "@eightbitstudio/browser-workbench/contracts";
export interface MonitorCore { command: (request: string) => unknown; debugState: () => unknown; debugCommand: (request: string) => unknown }
export type MonitorLoader = (sessionId: number, firmware: "monitor" | "basic-development" | "original" | "spectrum-development" | "c64-original" | "vectrex-original", launch: unknown, debugSource: unknown, originalMedia?: unknown) => Promise<MonitorCore>;
function session(value: unknown): value is number { return typeof value === "number" && Number.isInteger(value) && value > 0 && value <= 0xffffffff; }
function bounded(json: string, limit: number) { if (json.length > limit || new TextEncoder().encode(json).length > limit) throw new Error("Request or response is too large"); return json; }
/** One worker, one fixed owner, one in-flight operation, one Rust execution owner. */
export function monitorWorkerHost(load: MonitorLoader, post: (reply: unknown) => void, capability: (target?: string) => Promise<unknown> = async () => { throw new Error("Original capability unavailable"); }) {
  let runtime: MonitorCore | undefined;
  let owner: number | undefined;
  let selected: "monitor" | "basic-development" | "original" | "spectrum-development" | "c64-original" | "vectrex-original" | undefined;
  let originalTarget: string | undefined;
  let busy = false;
  let failed = false;
  return async (data: unknown) => {
    if (!record(data) || typeof data.id !== "number" || !Number.isSafeInteger(data.id) || data.id <= 0) return;
    const id = data.id;
    if (busy || failed) { post({ id, error: "Monitor is busy or unavailable." }); return; }
    busy = true;
    try {
      if (data.originalProfile !== undefined) {
        if (owner !== undefined || Object.keys(data).some(key => !["id", "sessionId", "originalProfile"].includes(key)) || !session(data.sessionId)) throw new Error("Invalid original identity request");
        // Bind the independently loaded worker before constructing any guest.
        // A legacy worker rejects this request and cannot silently launch.
        try {
          const expected = originalRuntimeCapability({ schemaVersion: 1, profile: data.originalProfile });
          if (!expected) throw new Error("Missing selected profile");
          const actual = originalRuntimeCapability(await (expected.target === "dragon32" ? capability() : capability(expected.target)));
          if (!actual || JSON.stringify(expected) !== JSON.stringify(actual)) throw new Error("Firmware identity differs");
          owner = data.sessionId; selected = "original"; originalTarget = actual.target;
          post({ id, originalProfile: actual });
        } catch { failed = true; post({ id, error: "Original BASIC assets changed. Reload the studio before running." }); }
        return;
      }
      if (data.debug !== undefined) {
        if (!record(data.debug) || Object.keys(data).some(key => !["id", "sessionId", "debug"].includes(key)) || Object.keys(data.debug).some(key => key !== "request") || !session(data.sessionId) || data.sessionId !== owner || !runtime || (data.debug.request !== null && typeof data.debug.request !== "string")) throw new Error("Invalid debug owner");
        const encoded = data.debug.request === null ? runtime.debugState() : runtime.debugCommand(bounded(data.debug.request, ORIGINAL_DEBUG_REQUEST_LIMIT));
        if (encoded !== null && encoded !== undefined && typeof encoded !== "string") throw new Error("Invalid debug response");
        post({ id, encoded: typeof encoded === "string" ? bounded(encoded, ORIGINAL_DEBUG_RESPONSE_LIMIT) : null });
        return;
      }
      if (!record(data.request) || !session(data.request.sessionId)) throw new Error("Invalid owner");
      const { request, firmware = "monitor", launch, debugSource, originalMedia } = data;
      const sessionId = request.sessionId;
      if (!session(sessionId) || (firmware !== "monitor" && firmware !== "basic-development" && firmware !== "original" && firmware !== "spectrum-development" && firmware !== "c64-original" && firmware !== "vectrex-original")) throw new Error("Invalid firmware");
      if ((firmware === "spectrum-development" || firmware === "c64-original" || firmware === "vectrex-original") && (debugSource !== undefined || originalMedia !== undefined)) throw new Error("Invalid Spectrum launch");
      if ((owner !== undefined && owner !== sessionId) || (selected !== undefined && selected !== firmware)) throw new Error("Session ownership is fixed");
      if ((runtime || firmware === "monitor") && (launch !== undefined || debugSource !== undefined || originalMedia !== undefined)) throw new Error("Launch is fixed");
      if (firmware === "original" && selected !== "original") throw new Error("Original firmware identity must be acknowledged before launch");
      if (!runtime && firmware === "original" && ((debugSource === undefined && originalMedia === undefined) || launch !== undefined)) throw new Error("Normal original launch requires a current build or selected media");
      if ([launch, debugSource, originalMedia].filter(value => value !== undefined).length > 1) throw new Error("Select one launch");
      if (firmware === "original" && debugSource !== undefined) {
        if (!record(debugSource) || (debugSource.kind === "assembly-source" ? !record(debugSource.project) || debugSource.project.target !== originalTarget : (debugSource.target ?? "dragon32") !== originalTarget)) throw new Error("Original source target differs from its admitted firmware");
      }
      if (firmware === "original" && originalMedia !== undefined && originalTarget !== "dragon32") throw new Error("Select a target-specific BASIC project for Dragon 64 media");
      if (originalMedia !== undefined) {
        if (firmware !== "original" || !record(originalMedia) || Object.keys(originalMedia).some(key => !["kind", "identity", "bytes", "breakpointAddress"].includes(key)) || originalMedia.kind !== "machine-code" || !(originalMedia.bytes instanceof Uint8Array) || !originalMedia.bytes.length || originalMedia.bytes.length > MAX_FILE_BYTES) throw new Error("Invalid original media");
        machineMediaIdentity(originalMedia.identity);
        if (originalMedia.breakpointAddress !== undefined && (typeof originalMedia.breakpointAddress !== "number" || !Number.isInteger(originalMedia.breakpointAddress) || originalMedia.breakpointAddress < 0 || originalMedia.breakpointAddress > 65535)) throw new Error("Invalid machine breakpoint");
      }
      const json = bounded(JSON.stringify(request), firmware === "spectrum-development" ? 5 * 1024 * 1024 : MONITOR_REQUEST_LIMIT);
      const initialLaunch = launch ?? (firmware === "spectrum-development" || firmware === "c64-original" || firmware === "vectrex-original" ? undefined : { schemaVersion: 1, backend: "original-development", target: "dragon32", cassette: null });
      if (debugSource !== undefined) bounded(JSON.stringify(debugSource), 7 * 1024 * 1024);
      else if (initialLaunch !== undefined) bounded(JSON.stringify(initialLaunch), firmware === "vectrex-original" ? 1024 * 1024 : firmware === "spectrum-development" || firmware === "c64-original" ? 512 * 1024 : MONITOR_REQUEST_LIMIT);
      owner = sessionId; selected = firmware;
      if (!runtime) {
        try { runtime = await load(sessionId, firmware, initialLaunch, debugSource, originalMedia); }
        catch (error) { failed = true; throw error; }
      }
      const encoded = runtime.command(json);
      if (typeof encoded !== "string") throw new Error("Invalid monitor response");
      const frame: unknown = JSON.parse(bounded(encoded, MONITOR_RESPONSE_LIMIT));
      post({ id, frame });
    } catch (error) {
      const message = typeof error === "string" && /^(?:DME(?:100[1-7]|130[0-4]|140[0-9]|150[0-9])|S48(?:RUN|ASM|EMU|CPU)100[0-9]|C64RUN100[0-2]):/.test(error) ? error.slice(0, 256) : "Cannot process this monitor operation.";
      post({ id, error: message });
    } finally { busy = false; }
  };
}
