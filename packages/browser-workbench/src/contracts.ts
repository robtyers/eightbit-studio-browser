import { originalSpectrumTarget, type SpectrumMachine } from "@eightbitstudio/workbench/services/monitorFrame";
/** Typed presentation boundary; BASIC, firmware and media rules remain in Rust. */
export type BrowserRequest =
  | { schemaVersion: 1; operation: "machines" }
  | { schemaVersion: 1; operation: "create-project"; name: string; kind: "basic" | "assembly"; source: string; target?: string; assembly?: { loadAddress: number; entrySymbol: string } }
  | { schemaVersion: 1; operation: "import-project" }
  | { schemaVersion: 1; operation: "export-project" | "test-project"; project: import("./projects").ProjectSnapshot }
  | { schemaVersion: 1; operation: "cassette-audio"; target: "dragon32" | "dragon64" | "dragon64-native"; cassetteSha256: string }
  | { schemaVersion: 1; operation: "build" | "test" | "presentation"; source: string; target?: string }
  | { schemaVersion: 1; operation: "firmware"; target?: string }
  | { schemaVersion: 1; operation: "decompile"; target?: string; input: MediaInput | { format: "cartridge" } };
export interface BrowserEngine {
  request(request: BrowserRequest, bytes: Uint8Array, signal: AbortSignal): Promise<unknown>;
}
export interface BuildResult {
  cassette: Uint8Array;
  tokenBytes: number;
  cassetteSha256: string;
  sourceSha256?: string;
  roundTripVerified: boolean;
  cartridgeVerified?: boolean;
  assembly?: { buildId: string; loadAddress: number; entryAddress: number };
}
export type MediaInput = { format: "cassette" | "decb" } | { format: "raw"; loadAddress: number; executionAddress: number };
export interface MachineMediaIdentity { schemaVersion: 1; input: MediaInput; inputSha256: string; cassetteSha256: string }
export interface OriginalRuntimeSource { schemaVersion: 1; target?: "dragon32" | "dragon64" | "dragon64-native"; source: string; sourceSha256: string; cassetteSha256: string; breakpointLine?: number }
export interface SpectrumSourceLaunch { schemaVersion: 1; target: SpectrumMachine; kind: "basic" | "assembly"; source: string; sourcePath: string; revision: string; sourceSha256: string; cassetteSha256: string; breakpointLine?: number }
export interface C64SourceLaunch { schemaVersion: 1; target: "c64-pal"; source: string; sourceSha256: string; prgSha256: string }
export interface OriginalRuntimeMedia { kind: "machine-code"; identity: MachineMediaIdentity; bytes: Uint8Array; breakpointAddress?: number }
export interface OriginalRuntimeAssembly { kind: "assembly-source"; schemaVersion: 1; project: import("./projects").ProjectSnapshot; buildId: string; cassetteSha256: string; debug?: { kind: "address"; address: number } | { kind: "source"; sourceLine: number; sourceUri?: string } }
export type OriginalRuntimeInput = OriginalRuntimeSource | OriginalRuntimeMedia | OriginalRuntimeAssembly;
export interface DecompiledSource { kind: "basic" | "assembly"; source: string; warningCount: number; target?: string; machineCode?: MachineMediaIdentity; originalMedia?: Uint8Array; assembly?: { loadAddress: number; entryAddress: number } }
export interface BrowserDiagnostic { sourceUri?: string; row: number; column: number; code: string; message: string }
export interface Highlight { start: number; end: number; kind: "number" | "keyword" | "string" | "comment" }
export class BrowserCoreError extends Error {
  constructor(readonly summary: string, readonly diagnostics: BrowserDiagnostic[]) {
    super([summary, ...diagnostics.map((item) => `${item.sourceUri ? `${item.sourceUri} · ` : ""}Row ${item.row}, column ${item.column}: ${item.code} ${item.message}`)].join("\n").slice(0, 2048));
  }
}
export const MAX_FILE_BYTES = 8 * 1024 * 1024;
export const MAX_CASSETTE_AUDIO_BYTES = 64 * 1024 * 1024;
export const MAX_SOURCE_BYTES = 1024 * 1024;
export function record(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
export function result(value: unknown): Record<string, unknown> {
  if (!record(value) || value.schemaVersion !== 1 || typeof value.ok !== "boolean") throw new Error("Invalid browser core response.");
  if (!value.ok) {
    const message = typeof value.message === "string" ? value.message.slice(0, 1024) : "The browser core rejected this input.";
    const diagnostics: BrowserDiagnostic[] = [];
    if (Array.isArray(value.diagnostics)) for (const diagnostic of value.diagnostics.slice(0, 12)) {
      if (record(diagnostic) && Number.isSafeInteger(diagnostic.row) && Number(diagnostic.row) > 0 && Number.isSafeInteger(diagnostic.column) && Number(diagnostic.column) > 0 && typeof diagnostic.message === "string") diagnostics.push({ ...(typeof diagnostic.sourceUri === "string" && diagnostic.sourceUri.length <= 256 ? { sourceUri: diagnostic.sourceUri } : {}), row: Number(diagnostic.row), column: Number(diagnostic.column), code: typeof diagnostic.code === "string" ? diagnostic.code.slice(0, 32) : "BASIC", message: diagnostic.message.slice(0, 256) });
    }
    throw new BrowserCoreError(message, diagnostics);
  }
  if (!record(value.result)) throw new Error("Invalid browser core result.");
  return value.result;
}
/** Rust ranges are ASCII byte offsets; non-ASCII listings use plain text. */
export function highlights(value: unknown, source: string): Highlight[] {
  const data = result(value);
  if (!Array.isArray(data.highlights) || data.highlights.length > 6000) throw new Error("Invalid editor presentation response.");
  const output: Highlight[] = [];
  let end = 0;
  for (const item of data.highlights) {
    if (!record(item) || !Number.isSafeInteger(item.start) || !Number.isSafeInteger(item.end) || Number(item.start) < end || Number(item.end) <= Number(item.start) || Number(item.end) > source.length || (item.kind !== "number" && item.kind !== "keyword" && item.kind !== "string" && item.kind !== "comment")) throw new Error("Invalid editor presentation range.");
    output.push({ start: Number(item.start), end: Number(item.end), kind: item.kind }); end = Number(item.end);
  }
  if (output.length && /[^\x00-\x7f]/.test(source)) throw new Error("Editor presentation requires ASCII source.");
  return output;
}
export function buildResult(value: unknown): BuildResult {
  const data = result(value);
  if (!record(data.summary) || !Number.isSafeInteger(data.summary.tokenBytes) || typeof data.summary.cassetteSha256 !== "string" || !/^[0-9a-f]{64}$/.test(data.summary.cassetteSha256) || !Array.isArray(data.cassette) || data.cassette.length > MAX_FILE_BYTES || !data.cassette.every((byte: unknown) => typeof byte === "number" && Number.isInteger(byte) && byte >= 0 && byte <= 255) || typeof data.roundTripVerified !== "boolean") throw new Error("Invalid build response.");
  const sourceSha256 = data.summary.sourceSha256;
  if (sourceSha256 !== undefined && (typeof sourceSha256 !== "string" || !/^[0-9a-f]{64}$/.test(sourceSha256))) throw new Error("Invalid source identity.");
  let assembly: BuildResult["assembly"];
  if (data.assembly !== undefined) {
    if (!record(data.assembly) || typeof data.assembly.buildId !== "string" || !/^[a-f0-9]{64}$/.test(data.assembly.buildId)) throw new Error("Invalid assembly build identity.");
    assembly = { buildId: data.assembly.buildId, ...assemblyAddresses(data.assembly) };
  }
  if (data.cartridgeVerified !== undefined && typeof data.cartridgeVerified !== "boolean") throw new Error("Invalid cartridge validation receipt.");
  return { ...(data.cartridgeVerified === undefined ? {} : {cartridgeVerified:data.cartridgeVerified}), sourceSha256, cassette: Uint8Array.from(data.cassette), tokenBytes: Number(data.summary.tokenBytes), cassetteSha256: data.summary.cassetteSha256, roundTripVerified: data.roundTripVerified, ...(assembly ? { assembly } : {}) };
}
export function cassetteAudio(value: unknown): Uint8Array {
  const data = result(value);
  if (!(data.audio instanceof Uint8Array) || data.audio.length < 44 || data.audio.length > MAX_CASSETTE_AUDIO_BYTES) throw new Error("Invalid cassette audio response.");
  return data.audio;
}
export function decompiledSource(value: unknown): DecompiledSource {
  const data = result(value);
  if ((data.kind !== "basic" && data.kind !== "assembly") || typeof data.source !== "string" || new TextEncoder().encode(data.source).length > MAX_SOURCE_BYTES || !Array.isArray(data.warnings)) throw new Error("Invalid or oversized decompilation response.");
  const machineCode = data.machineCode === undefined ? undefined : machineMediaIdentity(data.machineCode);
  if (machineCode && data.kind !== "assembly") throw new Error("Invalid machine-code preview.");
  return { kind: data.kind, source: data.source, warningCount: data.warnings.length, ...(machineCode ? { machineCode, assembly: assemblyAddresses(record(data.preview) ? { loadAddress: data.preview.loadAddress, entryAddress: data.preview.executionAddress } : undefined) } : {}) };
}
function assemblyAddresses(value: unknown): { loadAddress: number; entryAddress: number } {
  if (!record(value) || !Number.isInteger(value.loadAddress) || Number(value.loadAddress) < 0 || Number(value.loadAddress) > 65535 || !Number.isInteger(value.entryAddress) || Number(value.entryAddress) < 0 || Number(value.entryAddress) > 65535) throw new Error("Invalid machine-code addresses.");
  return { loadAddress: Number(value.loadAddress), entryAddress: Number(value.entryAddress) };
}
/** The Rust core remains authoritative for media format and address admission. */
export function machineMediaIdentity(value: unknown): MachineMediaIdentity {
  if (!record(value) || Object.keys(value).some(key => !["schemaVersion", "input", "inputSha256", "cassetteSha256"].includes(key)) || value.schemaVersion !== 1 || typeof value.inputSha256 !== "string" || !/^[a-f0-9]{64}$/.test(value.inputSha256) || typeof value.cassetteSha256 !== "string" || !/^[a-f0-9]{64}$/.test(value.cassetteSha256) || !record(value.input)) throw new Error("Invalid machine-code media identity.");
  const input = value.input;
  let selected: MediaInput;
  if ((input.format === "cassette" || input.format === "decb") && Object.keys(input).length === 1) selected = { format: input.format };
  else if (input.format === "raw" && Object.keys(input).length === 3 && Number.isInteger(input.loadAddress) && Number(input.loadAddress) >= 0 && Number(input.loadAddress) <= 65535 && Number.isInteger(input.executionAddress) && Number(input.executionAddress) >= 0 && Number(input.executionAddress) <= 65535) selected = { format: "raw", loadAddress: Number(input.loadAddress), executionAddress: Number(input.executionAddress) };
  else throw new Error("Invalid machine-code media interpretation.");
  return { schemaVersion: 1, input: selected, inputSha256: value.inputSha256, cassetteSha256: value.cassetteSha256 };
}
export function verifiedFirmware(value: unknown): boolean {
  const data = result(value); return data.matchesVerifiedDragon32 === true || data.matchesVerifiedFirmware === true;
}

export interface BrowserMachine { id: string; label: string; cpu: "6809" | "6502" | "Z80"; extension: "cas" | "prg" | "tap" | "p" | "ssd" | "bin"; manifest: string; defaultRuntime?: "original" | "xroar" | "mame-z80" | "mame-bbc" | "chips-c64" | "original-vectrex"; runtime: "xroar" | "mame-z80" | "mame-bbc" | "chips-c64" | "original-vectrex"; romSize: number; romNames: string[]; sample: string; assemblySample?: string }
export function machineCatalog(value: unknown): BrowserMachine[] {
  const data = result(value);
  if (!Array.isArray(data.machines) || data.machines.length < 1 || data.machines.length > 16) throw new Error("Invalid computer catalogue.");
  const machines: BrowserMachine[] = [];
  for (const item of data.machines) {
    if (!record(item) || typeof item.id !== "string" || !/^[a-z0-9-]{1,64}$/.test(item.id) || machines.some((m) => m.id === item.id) || typeof item.label !== "string" || item.label.length > 80 || !["6809", "6502", "Z80"].includes(String(item.cpu)) || !["cas", "prg", "tap", "p", "ssd", "bin"].includes(String(item.extension)) || !["vectrex.toml", "dragon.toml", "c64.toml", "bbc.toml", "spectrum.toml", "zx81.toml"].includes(String(item.manifest)) || !["xroar", "mame-z80", "mame-bbc", "chips-c64", "original-vectrex"].includes(String(item.runtime)) || !Number.isSafeInteger(item.romSize) || Number(item.romSize) < 4096 || Number(item.romSize) > 66496 || !Array.isArray(item.romNames) || item.romNames.length < 1 || item.romNames.length > 5 || !item.romNames.every((name: unknown) => typeof name === "string" && /^[a-z0-9_.-]{1,64}$/.test(name)) || typeof item.sample !== "string" || item.sample.length > MAX_SOURCE_BYTES) throw new Error("Invalid computer metadata.");
    if ((item.cpu !== "6809" && item.cpu !== "6502" && item.cpu !== "Z80") || (item.extension !== "cas" && item.extension !== "prg" && item.extension !== "tap" && item.extension !== "p" && item.extension !== "ssd" && item.extension !== "bin") || (item.runtime !== "xroar" && item.runtime !== "mame-z80" && item.runtime !== "mame-bbc" && item.runtime !== "chips-c64" && item.runtime !== "original-vectrex") || typeof item.manifest !== "string") throw new Error("Invalid computer metadata.");
    if (item.defaultRuntime !== undefined && item.defaultRuntime !== item.runtime && !((["dragon32", "dragon64", "dragon64-native"].includes(item.id) || originalSpectrumTarget(item.id)) && item.defaultRuntime === "original")) throw new Error("Invalid default runtime.");
    const defaultRuntime = item.defaultRuntime === "original" ? "original" : item.runtime;
    const romNames: string[] = [];
    if (item.assemblySample !== undefined && (item.id !== "vectrex" && !originalSpectrumTarget(item.id) || typeof item.assemblySample !== "string" || item.assemblySample.length > 128 * 1024)) throw new Error("Invalid assembly template.");
    for (const name of item.romNames) { if (typeof name !== "string") throw new Error("Invalid firmware name."); romNames.push(name); }
    machines.push({ id: item.id, label: item.label, cpu: item.cpu, extension: item.extension, manifest: item.manifest, runtime: item.runtime, ...(item.defaultRuntime === undefined ? {} : { defaultRuntime }), romSize: Number(item.romSize), romNames, sample: item.sample, ...(typeof item.assemblySample === "string" ? {assemblySample:item.assemblySample} : {}) });
  }
  return machines;
}

export interface VectrexSourceLaunch {schemaVersion:1;target:"vectrex";source:string;sourceSha256:string;cartridgeSha256:string}
