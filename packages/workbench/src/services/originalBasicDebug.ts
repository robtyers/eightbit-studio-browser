import { originalRuntimeProfileId } from "./originalRuntime";
import { monitorFrame, recordedDownload, type MonitorFrame } from "./monitorFrame";
import type { MonitorCommand } from "./monitorProtocol";

/** Development library protocol; this does not register a normal runtime. */
export const ORIGINAL_DEBUG_REQUEST_LIMIT = 4 * 1024 * 1024 + 4096;
export const ORIGINAL_DEBUG_RESPONSE_LIMIT = 12 * 1024 * 1024;
export interface OriginalDebugIdentity { readonly sessionId: number; readonly buildId: string; readonly backend?: "original-development" | "original" }
export interface OriginalDebugRevision extends OriginalDebugIdentity { readonly revision: string }
export type OriginalDebugCommand = { operation: "state" | "continue" | "cancel" } | { operation: "poll"; cycles: number } | { operation: "session"; command: MonitorCommand };
export interface OriginalDebugVariable { name: string; runtimeName: string; kind: "number" | "string"; displayValue: string; numericValue?: number; stringValue?: string }
export interface OriginalDebugRegisters { a: number; b: number; cc: number; dp: number; x: number; y: number; u: number; s: number; pc: number }
export interface OriginalDebugLocation { basicLine: number; ordinal: number; sourceStart: number; sourceEnd: number; runtimeAddress: number; tokenLength: number }
export interface OriginalDebugStatus {
  state: "ready" | "running" | "stopped" | "cancelled" | "failed" | "terminated";
  message: string;
  stopId?: string;
  breakpointVerified: boolean;
  valuesValid: boolean;
  variables: OriginalDebugVariable[];
  registers?: OriginalDebugRegisters;
  location?: OriginalDebugLocation;
}
export interface OriginalDebugFailure { code: string; message: string }
export interface OriginalDebugInspection {
  schemaVersion: 1;
  backend: "original-development" | "original";
  revision: OriginalDebugRevision;
  status: OriginalDebugStatus;
  failure?: OriginalDebugFailure;
}
export interface OriginalDebugReply extends OriginalDebugInspection {
  schemaVersion: 1;
  backend: "original-development" | "original";
  revision: OriginalDebugRevision;
  status: OriginalDebugStatus;
  session: MonitorFrame;
  recordingExport?: Uint8Array<ArrayBuffer>;
  failure?: OriginalDebugFailure;
}
function record(value: unknown): value is Record<string, unknown> { return typeof value === "object" && value !== null && !Array.isArray(value); }
function integer(value: unknown, max: number): value is number { return typeof value === "number" && Number.isSafeInteger(value) && value >= 0 && value <= max; }
function counter(value: unknown): value is string { return typeof value === "string" && /^(0|[1-9][0-9]{0,19})$/.test(value) && BigInt(value) <= 18446744073709551615n; }
function keys(value: Record<string, unknown>, allowed: string[]) { return Object.keys(value).every(key => allowed.includes(key)); }
function invalid(): never { throw new Error("Invalid or stale original BASIC debug response."); }
function revision(value: unknown, owner: OriginalDebugIdentity): OriginalDebugRevision {
  if (!record(value) || !keys(value, ["sessionId", "buildId", "revision"]) || !integer(value.sessionId, 0xffffffff) || value.sessionId === 0 || value.sessionId !== owner.sessionId || typeof value.buildId !== "string" || !/^[a-f0-9]{64}$/.test(value.buildId) || value.buildId !== owner.buildId || !counter(value.revision)) return invalid();
  return { sessionId: value.sessionId, buildId: value.buildId, revision: value.revision };
}
function variable(value: unknown): OriginalDebugVariable {
  if (!record(value) || !keys(value, ["name", "runtimeName", "kind", "displayValue", "numericValue", "stringValue"]) || typeof value.name !== "string" || value.name.length > 8192 || new TextEncoder().encode(value.name).length > 8192 || typeof value.runtimeName !== "string" || !/^[A-Z][A-Z0-9]?\$?$/.test(value.runtimeName) || typeof value.displayValue !== "string" || value.displayValue.length > 512) return invalid();
  const common = { name: value.name, runtimeName: value.runtimeName, displayValue: value.displayValue };
  if (value.kind === "number" && typeof value.numericValue === "number" && Number.isFinite(value.numericValue) && value.stringValue === undefined && !value.runtimeName.endsWith("$")) return { ...common, kind: "number", numericValue: value.numericValue };
  if (value.kind === "string" && typeof value.stringValue === "string" && /^[\x00-\x7f]{0,255}$/.test(value.stringValue) && value.numericValue === undefined && value.runtimeName.endsWith("$")) return { ...common, kind: "string", stringValue: value.stringValue };
  return invalid();
}
function registers(value: unknown): OriginalDebugRegisters {
  if (!record(value) || !keys(value, ["a", "b", "cc", "dp", "x", "y", "u", "s", "pc"]) || !integer(value.a, 255) || !integer(value.b, 255) || !integer(value.cc, 255) || !integer(value.dp, 255) || !integer(value.x, 65535) || !integer(value.y, 65535) || !integer(value.u, 65535) || !integer(value.s, 65535) || !integer(value.pc, 65535)) return invalid();
  return { a: value.a, b: value.b, cc: value.cc, dp: value.dp, x: value.x, y: value.y, u: value.u, s: value.s, pc: value.pc };
}
function location(value: unknown): OriginalDebugLocation {
  if (!record(value) || !keys(value, ["basicLine", "ordinal", "sourceStart", "sourceEnd", "runtimeAddress", "tokenLength"]) || !integer(value.basicLine, 65534) || !integer(value.ordinal, 65535) || !integer(value.sourceStart, 1048576) || !integer(value.sourceEnd, 1048576) || value.sourceEnd < value.sourceStart || !integer(value.runtimeAddress, 65535) || !integer(value.tokenLength, 65536) || value.tokenLength === 0 || value.runtimeAddress + value.tokenLength > 65536) return invalid();
  return { basicLine: value.basicLine, ordinal: value.ordinal, sourceStart: value.sourceStart, sourceEnd: value.sourceEnd, runtimeAddress: value.runtimeAddress, tokenLength: value.tokenLength };
}

/** Validate only the wire shape/ownership; Rust owns BASIC and execution rules. */
export function originalBasicDebugInspection(json: string, owner: OriginalDebugIdentity): OriginalDebugInspection {
  if (json.length > 2 * 1024 * 1024 || new TextEncoder().encode(json).length > 2 * 1024 * 1024) return invalid();
  let value: unknown;
  try { value = JSON.parse(json); } catch { return invalid(); }
  if (!record(value) || !keys(value, ["schemaVersion", "backend", "revision", "status", "failure"]) || value.schemaVersion !== 1 || value.backend !== (owner.backend ?? "original-development") || !record(value.status)) return invalid();
  const owned = revision(value.revision, owner);
  const status = value.status;
  if (!keys(status, ["state", "message", "stopId", "breakpointVerified", "valuesValid", "variables", "registers", "location"]) || (status.state !== "ready" && status.state !== "running" && status.state !== "stopped" && status.state !== "cancelled" && status.state !== "failed" && status.state !== "terminated") || typeof status.message !== "string" || status.message.length > 256 || typeof status.breakpointVerified !== "boolean" || typeof status.valuesValid !== "boolean" || !Array.isArray(status.variables) || status.variables.length > 128) return invalid();
  const stopped = status.state === "stopped";
  if (status.breakpointVerified !== stopped || status.valuesValid !== stopped || (!stopped && (status.variables.length !== 0 || status.registers !== undefined || status.location !== undefined || status.stopId !== undefined))) return invalid();
  const result: OriginalDebugStatus = { state: status.state, message: status.message, breakpointVerified: status.breakpointVerified, valuesValid: status.valuesValid, variables: status.variables.map(variable) };
  if (stopped) {
    if (status.stopId !== owned.revision) return invalid();
    result.stopId = owned.revision;
    result.registers = registers(status.registers);
    result.location = location(status.location);
  }
  let failure: OriginalDebugFailure | undefined;
  if (value.failure !== undefined) {
    if (!record(value.failure) || !keys(value.failure, ["code", "message"]) || typeof value.failure.code !== "string" || !/^DME(?:100[1-7]|130[0-4])$/.test(value.failure.code) || typeof value.failure.message !== "string" || value.failure.message.length > 256) return invalid();
    failure = { code: value.failure.code, message: value.failure.message };
  }
  return { schemaVersion: 1, backend: owner.backend ?? "original-development", revision: owned, status: result, ...(failure ? { failure } : {}) };
}

export function originalBasicDebugReply(json: string, owner: OriginalDebugIdentity): OriginalDebugReply {
  if (json.length > ORIGINAL_DEBUG_RESPONSE_LIMIT || new TextEncoder().encode(json).length > ORIGINAL_DEBUG_RESPONSE_LIMIT) return invalid();
  let value: unknown;
  try { value = JSON.parse(json); } catch { return invalid(); }
  if (!record(value) || !keys(value, ["schemaVersion", "backend", "revision", "status", "session", "failure"]) || !record(value.session) || (owner.backend === "original" ? !originalRuntimeProfileId(value.session.firmwareId) : !["eightbit-dragon32-basic-0.1-dev", "eightbit-dragon32-basic-1"].includes(String(value.session.firmwareId)))) return invalid();
  const { session: raw, ...inspection } = value;
  const state = originalBasicDebugInspection(JSON.stringify(inspection), owner);
  const session = monitorFrame(raw, owner.sessionId);
  if (state.status.state === "terminated" && session.state !== "stopped") return invalid();
  return { ...state, session, ...(raw.recordingExport !== undefined ? { recordingExport: recordedDownload(raw) } : {}) };
}

/** A revision-matched rejected action retains the usable owned frame/session. */
export class OriginalBasicDebugCommandError extends Error {
  constructor(readonly reply: OriginalDebugReply, readonly failure: OriginalDebugFailure) { super(failure.message); }
}

export type OriginalDebugCall = (request: string, signal: AbortSignal) => Promise<string>;

/** One bounded in-flight request. Explicit Cancel is sent between polls; dispose
 * aborts an uncertain transport and releases its worker/window owner immediately.
 * No automatic polling, Continue, load or RUN is performed by this service. */
export class OriginalBasicDebugTransport {
  private current: OriginalDebugRevision;
  private closed = false;
  private pending?: { abort: AbortController; reject: (error: Error) => void; timer: ReturnType<typeof setTimeout> };
  constructor(initial: string, private readonly owner: OriginalDebugIdentity, private readonly call: OriginalDebugCall, private readonly release: () => void) {
    this.owner = { ...owner };
    try { this.current = originalBasicDebugReply(initial, this.owner).revision; }
    catch (error) { this.dispose(); throw error; }
  }
  request(command: OriginalDebugCommand): Promise<OriginalDebugReply> {
    if (this.closed || this.pending) return Promise.reject(new Error("Original BASIC debugger is disconnected or busy."));
    const expected = { ...this.current };
    let json: string;
    try { json = JSON.stringify({ schemaVersion: 1, revision: expected, command }); }
    catch { return Promise.reject(new Error("Invalid original BASIC debug request.")); }
    if (json.length > ORIGINAL_DEBUG_REQUEST_LIMIT || new TextEncoder().encode(json).length > ORIGINAL_DEBUG_REQUEST_LIMIT) return Promise.reject(new Error("Original BASIC debug request is too large."));
    return new Promise((resolve, reject) => {
      const abort = new AbortController();
      const pending = { abort, reject, timer: setTimeout(() => this.dispose("Original BASIC debug operation timed out."), 5000) };
      this.pending = pending;
      void Promise.resolve().then(() => {
        if (this.pending !== pending || this.closed) throw new Error("Original BASIC debugger disconnected.");
        return this.call(json, abort.signal);
      }).then(encoded => {
        if (this.pending !== pending || this.closed) return;
        try {
          if (typeof encoded !== "string") return invalid();
          const reply = originalBasicDebugReply(encoded, { ...expected, backend: this.owner.backend });
          const difference = BigInt(reply.revision.revision) - BigInt(expected.revision);
          if (difference < 0n || difference > 1n || (reply.failure && difference !== 0n)) return invalid();
          this.current = { ...reply.revision };
          clearTimeout(pending.timer); this.pending = undefined;
          if (reply.failure) reject(new OriginalBasicDebugCommandError(reply, reply.failure));
          else resolve(reply);
        } catch { this.dispose("Invalid original BASIC debug response. Start a new session."); }
      }, () => {
        if (this.pending === pending && !this.closed) this.dispose("Original BASIC debug transport failed. Start a new session.");
      });
    });
  }
  dispose(message = "Original BASIC debugger disconnected.") {
    if (this.closed) return;
    this.closed = true;
    const pending = this.pending; this.pending = undefined;
    if (pending) { clearTimeout(pending.timer); pending.abort.abort(); pending.reject(new Error(message)); }
    // Transport failure cannot reopen the session even if host teardown fails.
    try { this.release(); } catch { /* The host retains its separate teardown policy. */ }
  }
}
