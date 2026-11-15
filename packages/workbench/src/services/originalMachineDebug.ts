import { monitorFrame, recordedDownload, type MonitorFrame } from "./monitorFrame";
import { originalRuntimeProfileId } from "./originalRuntime";
import { basicHostProtocol, OwnedDebugHostConnection, type DebugHostProtocol, type DevelopmentHostPorts } from "./developmentHost";
import { originalBasicDebugInspection, originalBasicDebugReply, type OriginalDebugIdentity, type OriginalDebugInspection, type OriginalDebugReply, type OriginalDebugRegisters, type OriginalDebugRevision, type OriginalDebugFailure } from "./originalBasicDebug";

export interface OriginalMachineLocation { sourceUri: string; sourceLine: number; runtimeAddress: number; byteLength: number }
export interface OriginalMachineStatus {
  state: "ready" | "running" | "stopped" | "cancelled" | "failed" | "terminated";
  message: string; stopId: string | null; breakpointAddress: number; breakpointVerified: boolean; valuesValid: boolean;
  registers?: OriginalDebugRegisters; memory?: { startAddress: number; bytes: number[] };
  disassembly: { address: number; bytes: number[]; mnemonic: string; operand: string; location?: OriginalMachineLocation }[];
  location?: OriginalMachineLocation;
}
export interface OriginalMachineInspection { schemaVersion: 1; backend: "original"; kind: "machine"; revision: OriginalDebugRevision; status: OriginalMachineStatus; failure?: OriginalDebugFailure }
export interface OriginalMachineReply extends OriginalMachineInspection { session: MonitorFrame; recordingExport?: Uint8Array<ArrayBuffer> }
export type OriginalHostInspection = OriginalDebugInspection | OriginalMachineInspection;
export type OriginalHostReply = OriginalDebugReply | OriginalMachineReply;
export function isMachineInspection(state: OriginalHostInspection): state is OriginalMachineInspection { return "kind" in state && state.kind === "machine"; }
function record(value: unknown): value is Record<string, unknown> { return typeof value === "object" && value !== null && !Array.isArray(value); }
function keys(value: Record<string, unknown>, allowed: string[]) { return Object.keys(value).every(key => allowed.includes(key)); }
function integer(value: unknown, max = 65535): value is number { return typeof value === "number" && Number.isSafeInteger(value) && value >= 0 && value <= max; }
function invalid(): never { throw new Error("Invalid or stale original machine debug response."); }
function location(value: unknown): OriginalMachineLocation {
  if (!record(value) || !keys(value, ["sourceUri", "sourceLine", "runtimeAddress", "byteLength"]) || typeof value.sourceUri !== "string" || value.sourceUri.length === 0 || value.sourceUri.length > 8192 || !integer(value.sourceLine, 1048576) || value.sourceLine === 0 || !integer(value.runtimeAddress) || !integer(value.byteLength) || value.byteLength === 0 || value.runtimeAddress + value.byteLength > 32768) return invalid();
  return { sourceUri: value.sourceUri, sourceLine: value.sourceLine, runtimeAddress: value.runtimeAddress, byteLength: value.byteLength };
}
function bytes(value: unknown, max: number): number[] { if (!Array.isArray(value) || value.length > max || !value.every(byte => integer(byte, 255))) return invalid(); return value; }
function parse(json: string, limit: number): Record<string, unknown> { if (json.length > limit || new TextEncoder().encode(json).length > limit) return invalid(); let value: unknown; try { value = JSON.parse(json); } catch { return invalid(); } return record(value) ? value : invalid(); }
/** Shape and ownership validation only; the portable Rust controller owns execution. */
export function originalMachineDebugInspection(json: string, owner: OriginalDebugIdentity): OriginalMachineInspection {
  const value = parse(json, 128 * 1024);
  if (!keys(value, ["schemaVersion", "backend", "kind", "revision", "status", "failure"]) || value.schemaVersion !== 1 || value.backend !== "original" || owner.backend !== "original" || value.kind !== "machine" || !record(value.revision) || !keys(value.revision, ["sessionId", "buildId", "revision"]) || value.revision.sessionId !== owner.sessionId || !integer(owner.sessionId, 0xffffffff) || !owner.sessionId || value.revision.buildId !== owner.buildId || !/^[a-f0-9]{64}$/.test(owner.buildId) || typeof value.revision.revision !== "string" || !/^(0|[1-9][0-9]{0,19})$/.test(value.revision.revision) || BigInt(value.revision.revision) > 18446744073709551615n || !record(value.status)) return invalid();
  const revision = { sessionId: owner.sessionId, buildId: owner.buildId, revision: value.revision.revision };
  const s = value.status;
  if (!keys(s, ["state", "message", "stopId", "breakpointAddress", "breakpointVerified", "valuesValid", "registers", "memory", "disassembly", "location"]) || (s.state !== "ready" && s.state !== "running" && s.state !== "stopped" && s.state !== "cancelled" && s.state !== "failed" && s.state !== "terminated") || typeof s.message !== "string" || s.message.length > 256 || !integer(s.breakpointAddress, 32767) || typeof s.breakpointVerified !== "boolean" || typeof s.valuesValid !== "boolean" || !Array.isArray(s.disassembly) || s.disassembly.length > 8) return invalid();
  if (s.valuesValid ? ((s.state !== "stopped" && s.state !== "cancelled") || s.stopId !== revision.revision) : (s.stopId !== null || s.registers !== undefined || s.memory !== undefined || s.location !== undefined || s.disassembly.length !== 0)) return invalid();
  if (s.breakpointVerified && (!s.valuesValid || s.state !== "stopped")) return invalid();
  const status: OriginalMachineStatus = { state: s.state, message: s.message, stopId: s.valuesValid ? revision.revision : null, breakpointAddress: s.breakpointAddress, breakpointVerified: s.breakpointVerified, valuesValid: s.valuesValid, disassembly: [] };
  if (s.valuesValid) {
    const r = s.registers;
    if (!record(r) || !keys(r, ["a", "b", "cc", "dp", "x", "y", "u", "s", "pc"]) || !integer(r.a, 255) || !integer(r.b, 255) || !integer(r.cc, 255) || !integer(r.dp, 255) || !integer(r.x) || !integer(r.y) || !integer(r.u) || !integer(r.s) || !integer(r.pc)) return invalid();
    status.registers = { a: r.a, b: r.b, cc: r.cc, dp: r.dp, x: r.x, y: r.y, u: r.u, s: r.s, pc: r.pc };
    if (s.memory !== undefined) { const m = s.memory; if (!record(m) || !keys(m, ["startAddress", "bytes"]) || !integer(m.startAddress)) return invalid(); const data = bytes(m.bytes, 4096); if (!data.length || m.startAddress + data.length > 65536) return invalid(); status.memory = { startAddress: m.startAddress, bytes: data }; }
    if (s.location !== undefined) status.location = location(s.location);
    status.disassembly = s.disassembly.map(item => { if (!record(item) || !keys(item, ["address", "bytes", "mnemonic", "operand", "location"]) || !integer(item.address) || typeof item.mnemonic !== "string" || item.mnemonic.length > 32 || typeof item.operand !== "string" || item.operand.length > 128) return invalid(); const data = bytes(item.bytes, 8); if (!data.length || item.address + data.length > 65536) return invalid(); return { address: item.address, bytes: data, mnemonic: item.mnemonic, operand: item.operand, ...(item.location == null ? {} : { location: location(item.location) }) }; });
  }
  let failure: OriginalDebugFailure | undefined;
  if (value.failure !== undefined) { const f = value.failure; if (!record(f) || !keys(f, ["code", "message"]) || typeof f.code !== "string" || !/^DME(?:100[1-7]|1301|150[0-2])$/.test(f.code) || typeof f.message !== "string" || f.message.length > 256) return invalid(); failure = { code: f.code, message: f.message }; }
  return { schemaVersion: 1, backend: "original", kind: "machine", revision, status, ...(failure ? { failure } : {}) };
}
export function originalMachineDebugReply(json: string, owner: OriginalDebugIdentity): OriginalMachineReply {
  const value = parse(json, 12 * 1024 * 1024);
  if (!keys(value, ["schemaVersion", "backend", "kind", "revision", "status", "failure", "session"]) || !record(value.session) || !originalRuntimeProfileId(value.session.firmwareId)) return invalid();
  const { session: raw, ...inspection } = value;
  const state = originalMachineDebugInspection(JSON.stringify(inspection), owner);
  const session = monitorFrame(raw, owner.sessionId);
  if (state.status.state === "terminated" && session.state !== "stopped") return invalid();
  return { ...state, session, ...(raw.recordingExport === undefined ? {} : { recordingExport: recordedDownload(raw) }) };
}
export function originalHostInspection(json: string, owner: OriginalDebugIdentity): OriginalHostInspection { return parse(json, 2 * 1024 * 1024).kind === "machine" ? originalMachineDebugInspection(json, owner) : originalBasicDebugInspection(json, owner); }
export function originalHostReply(json: string, owner: OriginalDebugIdentity): OriginalHostReply { return parse(json, 12 * 1024 * 1024).kind === "machine" ? originalMachineDebugReply(json, owner) : originalBasicDebugReply(json, owner); }
export class OriginalMachineCommandError extends Error { constructor(readonly reply: OriginalMachineReply, readonly failure: OriginalDebugFailure) { super(failure.message); } }
function ownedHostProtocol(): DebugHostProtocol<OriginalHostReply> {
  let machine: boolean | undefined;
  return {
  read(json, owner) {
    const reply = originalHostReply(json, owner);
    const next = isMachineInspection(reply);
    if (machine !== undefined && next !== machine) return invalid();
    machine = next;
    return reply;
  },
  async control(initial, owner, operation, call, release, signal) {
    const before = originalHostReply(initial, owner);
    if (!isMachineInspection(before)) return basicHostProtocol.control(initial, owner, operation, call, release, signal);
    try {
      const json = await call(JSON.stringify({ schemaVersion: 1, revision: before.revision, command: { operation } }), signal);
      if (signal.aborted || typeof json !== "string") return invalid();
      const reply = originalMachineDebugReply(json, owner);
      const difference = BigInt(reply.revision.revision) - BigInt(before.revision.revision);
      if (difference < 0n || difference > 1n || (reply.failure && difference !== 0n)) return invalid();
      if (reply.failure) throw new OriginalMachineCommandError(reply, reply.failure);
      return reply;
    } catch (error) { if (!(error instanceof OriginalMachineCommandError)) release(); throw error; }
  },
  };
}
export class OriginalHostConnection extends OwnedDebugHostConnection<OriginalHostReply> {
  constructor(owner: OriginalDebugIdentity, ports: DevelopmentHostPorts, publish: (reply: OriginalHostReply) => void) { super(owner, ports, publish, ownedHostProtocol()); }
}
