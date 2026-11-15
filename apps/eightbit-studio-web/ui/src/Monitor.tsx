import type {VectrexSourceLaunch} from "@eightbitstudio/browser-workbench/contracts";
import type {VectrexMonitorProps} from "@eightbitstudio/browser-workbench";
import { type SpectrumMachine } from "@eightbitstudio/workbench/services/monitorFrame";
import { originalRuntimeCapability, type OriginalRuntimeProfile } from "@eightbitstudio/workbench/services/originalRuntime";
import { DevelopmentHostConnection } from "@eightbitstudio/workbench/services/developmentHost";
import { OriginalHostConnection, type OriginalHostReply } from "@eightbitstudio/workbench/services/originalMachineDebug";
import type { OriginalDebugReply } from "@eightbitstudio/workbench/services/originalBasicDebug";
import { useEffect, useMemo, useRef } from "react";
import type { C64MonitorProps, SpectrumMonitorProps } from "@eightbitstudio/browser-workbench";
type SpectrumControl={operation:"pause"}|{operation:"step"|"continue";stopId:number};
import { MonitorConsole, MONITOR_CASSETTE_LIMIT, type MonitorCommand } from "@eightbitstudio/workbench/components/MonitorConsole";
import { record, machineMediaIdentity, MAX_FILE_BYTES, type OriginalRuntimeSource, type OriginalRuntimeInput, type OriginalRuntimeMedia, type OriginalRuntimeAssembly, type SpectrumSourceLaunch, type C64SourceLaunch } from "@eightbitstudio/browser-workbench/contracts";

export type { OriginalRuntimeSource } from "@eightbitstudio/browser-workbench/contracts";
export interface DevelopmentDebugSource extends OriginalRuntimeSource { breakpointLine: number }
let nextSession = 0;
export class MonitorWorker {
  readonly sessionId = ++nextSession;
  constructor(private readonly firmware: "monitor" | "basic-development" | "original" | "spectrum-development" | "c64-original" | "vectrex-original" = "monitor", cassette?: Uint8Array, cassetteSha256?: string, debugSource?: OriginalRuntimeSource | OriginalRuntimeAssembly, expectedProfile?: OriginalRuntimeProfile, originalMedia?: OriginalRuntimeMedia, spectrumSource?: SpectrumSourceLaunch, spectrumTarget: SpectrumMachine = "spectrum48", c64Source?: C64SourceLaunch, vectrexSource?:VectrexSourceLaunch) {
    if (firmware === "vectrex-original") {
      if (!vectrexSource || cassette || debugSource || expectedProfile || originalMedia || spectrumSource || c64Source || JSON.stringify(vectrexSource).length>1024*1024) {this.invalid="Invalid Vectrex launch.";return;}
      this.launch=structuredClone(vectrexSource);return;
    }
    if (firmware === "c64-original") {
      if (!c64Source || cassette || debugSource || expectedProfile || originalMedia || spectrumSource || JSON.stringify(c64Source).length > 256 * 1024) { this.invalid = "Invalid C64 launch."; return; }
      this.launch = structuredClone(c64Source); return;
    }
    if (firmware === "spectrum-development") {
      if (cassette || debugSource || expectedProfile || originalMedia || spectrumSource && new TextEncoder().encode(JSON.stringify(spectrumSource)).length > 512 * 1024) { this.invalid = "Invalid Spectrum development launch."; return; }
      this.launch = spectrumSource ? structuredClone(spectrumSource) : spectrumTarget === "spectrum48" ? undefined : {schemaVersion:1,kind:"boot",target:spectrumTarget}; return;
    }
    if (firmware === "original") {
      try { this.originalProfile = originalRuntimeCapability({ schemaVersion: 1, profile: expectedProfile }) ?? undefined; }
      catch { this.invalid = "Normal original execution requires the selected firmware identity."; return; }
      if (!this.originalProfile) { this.invalid = "Normal original execution requires the selected firmware identity."; return; }
    }
    if (originalMedia) {
      if (firmware !== "original" || cassette || debugSource || !(originalMedia.bytes instanceof Uint8Array) || !originalMedia.bytes.length || originalMedia.bytes.length > MAX_FILE_BYTES) { this.invalid = "Invalid original machine-code media."; return; }
      try { this.originalMedia = { kind: "machine-code", identity: machineMediaIdentity(originalMedia.identity), bytes: originalMedia.bytes.slice(), ...(originalMedia.breakpointAddress === undefined ? {} : { breakpointAddress: originalMedia.breakpointAddress }) }; }
      catch { this.invalid = "Invalid original machine-code media identity."; }
      return;
    }
    if (debugSource) {
      if (firmware === "monitor" || cassette || new TextEncoder().encode("kind" in debugSource ? debugSource.project.source : debugSource.source).length > 1024 * 1024) { this.invalid = "Invalid development debug source."; return; }
      if ("kind" in debugSource && firmware !== "original") { this.invalid = "Assembly requires the original runtime."; return; }
      this.debugSource = "kind" in debugSource || !this.originalProfile || this.originalProfile.target === "dragon32" ? structuredClone(debugSource) : { ...structuredClone(debugSource), target: this.originalProfile.target }; return;
    }
    if (firmware === "original") { this.invalid = "Normal original execution requires a fresh source build."; return; }
    if (cassette && (!cassette.byteLength || cassette.byteLength > MONITOR_CASSETTE_LIMIT)) { this.invalid = "Select a development cassette of 1–1048576 bytes."; return; }
    if (firmware === "basic-development") this.launch = { schemaVersion: 1, backend: "original-development", target: "dragon32", cassette: cassette && cassetteSha256 ? { bytes: Array.from(cassette), sha256: cassetteSha256, buildId: null } : null };
    this.cassette = cassetteSha256 ? undefined : cassette?.slice();
  }
  private originalProfile?: OriginalRuntimeProfile;
  private originalMedia?: OriginalRuntimeMedia;
  private identityConfirmed = false;
  private launch?: VectrexSourceLaunch | C64SourceLaunch | { schemaVersion: 1; backend: "original-development"; target: "dragon32"; cassette: { bytes: number[]; sha256: string; buildId: null } | null } | SpectrumSourceLaunch | {schemaVersion:1;kind:"boot";target:SpectrumMachine};
  private cassette?: Uint8Array;
  private debugSource?: OriginalRuntimeSource | OriginalRuntimeAssembly;
  private invalid?: string;
  private requestBusy = false;
  private worker?: Worker;
  private nextRequest = 0;
  private stopped = false;
  private lifetime?: object;
  retain() {
    const identity = {}; this.lifetime = identity;
    return () => { queueMicrotask(() => { if (this.lifetime === identity) this.dispose(); }); };
  }
  private pending?: { id: number; resolve: (value: unknown) => void; reject: (reason: Error) => void; timer: ReturnType<typeof setTimeout> };
  request = (command: MonitorCommand | SpectrumControl): Promise<unknown> => {
    if (this.invalid) return Promise.reject(new Error(this.invalid));
    if (this.stopped || this.requestBusy) return Promise.reject(new Error("Monitor is stopped or busy."));
    this.requestBusy = true;
    const cassette = this.cassette; this.cassette = undefined;
    let result: Promise<unknown>;
    if (this.originalProfile && !this.identityConfirmed) result = this.send(undefined, undefined, true).then(() => this.send(command)).catch(error => { this.dispose(); throw error; });
    else if (cassette) result = this.send({ operation: "mount-cassette", bytes: Array.from(cassette) }).then(() => this.send(command)).catch(error => { this.dispose(); throw error; });
    else result = this.send(command);
    return result.finally(() => { this.requestBusy = false; });
  };
  debug = (request: string | null, signal?: AbortSignal): Promise<unknown> => {
    if (this.invalid || this.stopped || this.requestBusy || !this.worker || signal?.aborted) return Promise.reject(new Error("Development session is not ready or busy."));
    this.requestBusy = true;
    const abort = () => this.dispose();
    signal?.addEventListener("abort", abort, { once: true });
    return this.send(undefined, request).finally(() => { signal?.removeEventListener("abort", abort); this.requestBusy = false; });
  };
  private send(command?: MonitorCommand | SpectrumControl, debug?: string | null, identity = false): Promise<unknown> {
    if (this.stopped || this.pending) return Promise.reject(new Error("Monitor is stopped or busy."));
    try { this.worker ??= new Worker(new URL("./monitor-worker.ts", import.meta.url), { type: "module" }); }
    catch { this.stopped = true; return Promise.reject(new Error("Cannot start the emulator worker.")); }
    const worker = this.worker;
    return new Promise((resolve, reject) => {
      const id = ++this.nextRequest;
      const fail = () => { if (this.pending?.id !== id) return; this.dispose(); reject(new Error("Monitor operation timed out or failed.")); };
      const timer = setTimeout(fail, 5000);
      this.pending = { id, resolve, reject, timer };
      worker.onerror = fail;
      worker.onmessage = (event: MessageEvent<unknown>) => {
        const pending = this.pending;
        if (!pending || !record(event.data) || event.data.id !== pending.id) return;
        clearTimeout(pending.timer); this.pending = undefined;
        if (typeof event.data.error === "string") reject(new Error(event.data.error.slice(0, 256)));
        else if (identity) {
          try {
            const actual = originalRuntimeCapability({ schemaVersion: 1, profile: event.data.originalProfile });
            if (!actual || JSON.stringify(actual) !== JSON.stringify(this.originalProfile)) throw new Error("Firmware identity differs");
            this.identityConfirmed = true; resolve(undefined);
          } catch { this.dispose(); reject(new Error("Original BASIC assets changed. Reload the studio before running.")); }
        } else resolve(debug !== undefined ? event.data.encoded : event.data.frame);
      };
      try { worker.postMessage(identity ? { id, sessionId: this.sessionId, originalProfile: this.originalProfile } : debug !== undefined ? { id, sessionId: this.sessionId, debug: { request: debug } } : { id, firmware: this.firmware, ...(this.launch ? { launch: this.launch } : {}), ...(this.debugSource ? { debugSource: this.debugSource } : {}), ...(this.originalMedia ? { originalMedia: this.originalMedia } : {}), request: { schemaVersion: 1, sessionId: this.sessionId, command } }); if (!identity) { this.launch = undefined; this.debugSource = undefined; this.originalMedia = undefined; } }
      catch { fail(); }
    });
  };
  dispose() {
    this.stopped = true; this.cassette = undefined; this.launch = undefined; this.debugSource = undefined; this.originalMedia = undefined; this.worker?.terminate(); this.worker = undefined;
    if (this.pending) { clearTimeout(this.pending.timer); this.pending.reject(new Error("Monitor stopped.")); this.pending = undefined; }
  }
}
export function Monitor({ onStop, onRuntime, firmware = "monitor", cassette, cassetteSha256, spectrumSource }: { onStop: () => void; onRuntime?: (release?: () => void) => void; firmware?: "monitor" | "basic-development" | "spectrum-development"; cassette?: Uint8Array; cassetteSha256?: string; spectrumSource?: SpectrumSourceLaunch }) {
  const runtime = useMemo(() => new MonitorWorker(firmware, cassette, cassetteSha256, undefined, undefined, undefined, spectrumSource), [firmware, cassette, cassetteSha256, spectrumSource]);
  useEffect(() => runtime.retain(), [runtime]);
  useEffect(() => { onRuntime?.(() => runtime.dispose()); return () => onRuntime?.(); }, [runtime, onRuntime]);
  return <MonitorConsole machine={firmware === "spectrum-development" ? "spectrum48" : "dragon32"} sessionId={runtime.sessionId} transport={runtime.request} onStop={() => { runtime.dispose(); onStop(); }} />;
}
export function SpectrumMonitor({target="spectrum48",source,onStop,onRuntime,onDebugger,onInspection}: SpectrumMonitorProps) {
  const observer=useRef(onInspection);observer.current=onInspection;
  const runtime=useMemo(()=>new MonitorWorker("spectrum-development",undefined,undefined,undefined,undefined,undefined,source,target),[source,target]);
  const queue=useMemo(()=>({sequence:Promise.resolve(),pending:0,active:true,token:{}}),[runtime]);
  const request=useMemo(()=>(command:MonitorCommand|SpectrumControl):Promise<unknown>=>{
    if(!queue.active||queue.pending>=16&&command.operation!=="stop"&&command.operation!=="release")return Promise.reject(new Error("Spectrum debugger is disconnected or busy."));
    const token=queue.token;queue.pending++;
    const reply=queue.sequence.then(()=>{if(!queue.active||queue.token!==token)throw new Error("Spectrum session changed.");return runtime.request(command);}).then(frame=>{if(queue.active&&queue.token===token)observer.current?.(frame,runtime.sessionId,source);return frame;});
    queue.sequence=reply.then(()=>{},()=>{}).finally(()=>{queue.pending--;});return reply;
  },[runtime,queue,source]);
  useEffect(()=>{const token={};queue.token=token;queue.active=true;const release=runtime.retain();return()=>{if(queue.token===token)queue.active=false;release();};},[runtime,queue]);
  useEffect(()=>{onRuntime?.(()=>{queue.active=false;runtime.dispose();});return()=>onRuntime?.();},[runtime,queue,onRuntime]);
  useEffect(()=>{onDebugger?.({command:(operation,stopId)=>{
    if(!Number.isSafeInteger(stopId)||stopId<0||stopId>0xffffffff)return Promise.reject(new Error("Invalid Spectrum stop."));
    return request(operation==="pause"?{operation}:{operation,stopId});
  }});return()=>onDebugger?.();},[request,onDebugger]);
  return <MonitorConsole machine={source?.target ?? target} sessionId={runtime.sessionId} transport={request} onStop={()=>{queue.active=false;runtime.dispose();onStop();}}/>;
}

/** Explicit development Debug composition; caller owns current source identity. */
export function browserDevelopmentDebug(source: DevelopmentDebugSource, publish: (reply: OriginalDebugReply) => void, released: () => void = () => {}) {
  const runtime = new MonitorWorker("basic-development", undefined, undefined, source);
  const connection = new DevelopmentHostConnection({ sessionId: runtime.sessionId, buildId: source.sourceSha256 }, { monitor: runtime.request, debug: runtime.debug, release: () => { runtime.dispose(); released(); } }, publish);
  return { sessionId: runtime.sessionId, connection };
}

/** Explicit normal Run/Debug composition. The Rust factory admits compiled
 * firmware before building or executing. Imported bytes are independently revalidated.
 * The caller binds this immutable source to its existing project/revision owner. */
export function browserOriginalRuntime(source: OriginalRuntimeInput, publish: (reply: OriginalHostReply) => void, released: () => void, profile: OriginalRuntimeProfile) {
  const media = "kind" in source && source.kind === "machine-code" ? source : undefined;
  const assembly = "kind" in source && source.kind === "assembly-source" ? source : undefined;
  const basic = "kind" in source ? undefined : source;
  const runtime = new MonitorWorker("original", undefined, undefined, basic ?? assembly, profile, media);
  const release = () => { runtime.dispose(); released(); };
  const owner = { sessionId: runtime.sessionId, buildId: basic?.sourceSha256 ?? assembly?.buildId ?? media!.identity.cassetteSha256, backend: "original" as const };
  const connection = basic?.breakpointLine === undefined && assembly?.debug === undefined && media?.breakpointAddress === undefined ? undefined : new OriginalHostConnection(owner, { monitor: runtime.request, debug: runtime.debug, release }, publish);
  return { owner, transport: connection?.monitor ?? runtime.request, connection, dispose: () => { if (connection) connection.dispose(); else release(); } };
}

export function C64Monitor({source,onStop,onRuntime}:C64MonitorProps) {
  const runtime=useMemo(()=>new MonitorWorker("c64-original",undefined,undefined,undefined,undefined,undefined,undefined,"spectrum48",source),[source]);
  useEffect(()=>runtime.retain(),[runtime]);
  useEffect(()=>{onRuntime?.(()=>runtime.dispose());return()=>onRuntime?.();},[runtime,onRuntime]);
  return <MonitorConsole machine="c64-pal" sessionId={runtime.sessionId} transport={runtime.request} onStop={()=>{runtime.dispose();onStop();}}/>;
}

export function VectrexMonitor({source,onRuntime,onStop}:VectrexMonitorProps) {
 const runtime=useMemo(()=>new MonitorWorker("vectrex-original",undefined,undefined,undefined,undefined,undefined,undefined,"spectrum48",undefined,source),[source]);
 useEffect(()=>runtime.retain(),[runtime]);
 useEffect(()=>{onRuntime?.(()=>runtime.dispose());return()=>onRuntime?.();},[runtime,onRuntime]);
 return <MonitorConsole machine="vectrex" sessionId={runtime.sessionId} transport={runtime.request} onStop={()=>{runtime.dispose();onStop();}}/>;
}
