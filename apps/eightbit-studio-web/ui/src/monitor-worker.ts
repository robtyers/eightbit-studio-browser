import { record } from "@eightbitstudio/browser-workbench/contracts";
import { monitorWorkerHost, type MonitorLoader } from "./monitorWorkerHost";

// The capability handshake and construction use the same initialised module.
let loaded: Promise<Record<string, unknown>> | undefined;
function moduleForOwner() {
  loaded ??= (async () => {
    const module: unknown = await import(/* @vite-ignore */ `${import.meta.env.BASE_URL}generated/dragon_core.js`);
    if (!record(module) || typeof module.default !== "function" || typeof module.MonitorRuntime !== "function" || typeof module.browser_request !== "function") throw new Error("Monitor core is unavailable");
    await module.default({ module_or_path: `${import.meta.env.BASE_URL}generated/dragon_core_bg.wasm` });
    return module;
  })();
  return loaded;
}
const load: MonitorLoader = async (sessionId, firmware, launch, debugSource, originalMedia) => {
  const module = await moduleForOwner();
  if (firmware === "vectrex-original") {
    const constructor = module.VectrexOriginalRuntime;
    if (typeof constructor !== "function") throw new Error("Original Vectrex core is unavailable");
    const factory: unknown = Reflect.get(constructor,"fromSource");
    if (typeof factory !== "function" || !record(launch)) throw new Error("Invalid original Vectrex launch");
    const runtime: unknown = factory.call(constructor,sessionId,JSON.stringify(launch));
    if (!record(runtime) || typeof runtime.command !== "function") throw new Error("Invalid Vectrex runtime");
    const {command} = runtime;
    return {command: request => command.call(runtime,request),debugState:()=>null,debugCommand:()=>{throw new Error("Vectrex debugging is unavailable");}};
  }
  if (firmware === "c64-original") {
    const constructor = module.C64OriginalRuntime;
    if (typeof constructor !== "function" || !record(launch)) throw new Error("Original C64 core is unavailable");
    const factory: unknown = Reflect.get(constructor, "fromSource");
    if (typeof factory !== "function") throw new Error("Original C64 source factory is unavailable");
    const instance: unknown = factory.call(constructor, sessionId, JSON.stringify(launch));
    if (!record(instance) || typeof instance.command !== "function") throw new Error("Original C64 core is unavailable");
    const {command} = instance;
    return {command: request => command.call(instance, request), debugState: () => null, debugCommand: () => {throw new Error("C64 debugging is unavailable");}};
  }
  if (firmware === "spectrum-development") {
    const constructor = module.SpectrumDevelopmentRuntime;
    if (typeof constructor !== "function") throw new Error("Original Spectrum core is unavailable");
    if (record(launch) && launch.kind === "boot") {
      if (launch.schemaVersion !== 1 || typeof launch.target !== "string" || Object.keys(launch).sort().join() !== "kind,schemaVersion,target") throw new Error("Invalid Spectrum boot request");
      const boot: unknown = Reflect.get(constructor, "forTarget");
      if (typeof boot !== "function") throw new Error("Original Spectrum model factory is unavailable");
      const instance: unknown = boot.call(constructor, sessionId, launch.target);
      if (!record(instance) || typeof instance.command !== "function") throw new Error("Original Spectrum core is unavailable");
      const {command} = instance;
      return { command: request => command.call(instance, request), debugState: () => null, debugCommand: () => {throw new Error("Use the Spectrum debugger command contract");} };
    }
    const factory: unknown = Reflect.get(constructor, "fromSource");
    if (launch !== undefined && typeof factory !== "function") throw new Error("Original Spectrum source factory is unavailable");
    const instance: unknown = launch !== undefined && typeof factory === "function" ? factory.call(constructor, sessionId, JSON.stringify(launch)) : Reflect.construct(constructor, [sessionId]);
    if (!record(instance) || typeof instance.command !== "function") throw new Error("Original Spectrum core is unavailable");
    const { command } = instance;
    return { command: request => command.call(instance, request), debugState: () => null, debugCommand: () => { throw new Error("Use the Spectrum debugger command contract"); } };
  }
  if (typeof module.MonitorRuntime !== "function") throw new Error("Monitor core is unavailable");
  const factory: unknown = Reflect.get(module.MonitorRuntime, firmware === "original" ? originalMedia !== undefined ? "fromOriginalMedia" : record(debugSource) && debugSource.kind === "assembly-source" ? "fromOriginalAssembly" : "fromOriginalSource" : debugSource === undefined ? "fromDevelopmentLaunch" : "fromDevelopmentDebugSource");
  if (firmware !== "monitor" && typeof factory !== "function") throw new Error("Development core is unavailable");
  const instance: unknown = firmware !== "monitor" && typeof factory === "function"
    ? record(originalMedia) ? factory.call(module.MonitorRuntime, sessionId, JSON.stringify({ ...(record(originalMedia.identity) ? originalMedia.identity : {}), ...(originalMedia.breakpointAddress === undefined ? {} : { breakpointAddress: originalMedia.breakpointAddress }) }), originalMedia.bytes) : factory.call(module.MonitorRuntime, sessionId, JSON.stringify(debugSource ?? launch))
    : Reflect.construct(module.MonitorRuntime, [sessionId]);
  if (!record(instance) || typeof instance.command !== "function" || typeof instance.debugState !== "function" || typeof instance.debugCommand !== "function") throw new Error("Monitor core is unavailable");
  const { command, debugState, debugCommand } = instance;
  return { command: request => command.call(instance, request), debugState: () => debugState.call(instance), debugCommand: request => debugCommand.call(instance, request) };
};
const receive = monitorWorkerHost(load, reply => self.postMessage(reply), async (target) => {
  const module = await moduleForOwner();
  if (typeof module.browser_request !== "function") throw new Error("Original capability unavailable");
  const encoded: unknown = module.browser_request('{"schemaVersion":1,"operation":"machines"}', new Uint8Array());
  if (typeof encoded !== "string" || encoded.length > 64 * 1024) throw new Error("Invalid original capability response");
  const response: unknown = JSON.parse(encoded);
  if (!record(response) || response.schemaVersion !== 1 || response.ok !== true || !record(response.result)) throw new Error("Invalid original capability response");
  return target && target !== "dragon32" && record(response.result.originalRuntimes) ? response.result.originalRuntimes[target] : response.result.originalRuntime;
});
self.addEventListener("message", (event: MessageEvent<unknown>) => { void receive(event.data); });
