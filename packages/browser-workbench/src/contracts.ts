/** Typed presentation boundary; BASIC, firmware and media rules remain in Rust. */
export type BrowserRequest =
  | { schemaVersion: 1; operation: "machines" }
  | { schemaVersion: 1; operation: "create-project"; name: string; kind: "basic" | "assembly"; source: string; target?: string }
  | { schemaVersion: 1; operation: "import-project" }
  | { schemaVersion: 1; operation: "export-project" | "test-project"; project: import("./projects").ProjectSnapshot }
  | { schemaVersion: 1; operation: "build" | "test" | "presentation"; source: string; target?: string }
  | { schemaVersion: 1; operation: "firmware"; target?: string }
  | { schemaVersion: 1; operation: "decompile"; target?: string; input: { format: "cassette" } | { format: "raw"; loadAddress: number; executionAddress: number } };
export interface BrowserEngine {
  request(request: BrowserRequest, bytes: Uint8Array, signal: AbortSignal): Promise<unknown>;
}
export interface BuildResult {
  cassette: Uint8Array;
  tokenBytes: number;
  cassetteSha256: string;
  roundTripVerified: boolean;
}
export interface DecompiledSource { kind: "basic" | "assembly"; source: string; warningCount: number; target?: string }
export interface BrowserDiagnostic { row: number; column: number; code: string; message: string }
export interface Highlight { start: number; end: number; kind: "number" | "keyword" | "string" | "comment" }
export class BrowserCoreError extends Error {
  constructor(readonly summary: string, readonly diagnostics: BrowserDiagnostic[]) {
    super([summary, ...diagnostics.map((item) => `Row ${item.row}, column ${item.column}: ${item.code} ${item.message}`)].join("\n").slice(0, 2048));
  }
}
export const MAX_FILE_BYTES = 8 * 1024 * 1024;
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
      if (record(diagnostic) && Number.isSafeInteger(diagnostic.row) && Number(diagnostic.row) > 0 && Number.isSafeInteger(diagnostic.column) && Number(diagnostic.column) > 0 && typeof diagnostic.message === "string") diagnostics.push({ row: Number(diagnostic.row), column: Number(diagnostic.column), code: typeof diagnostic.code === "string" ? diagnostic.code.slice(0, 32) : "BASIC", message: diagnostic.message.slice(0, 256) });
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
  return { cassette: Uint8Array.from(data.cassette), tokenBytes: Number(data.summary.tokenBytes), cassetteSha256: data.summary.cassetteSha256, roundTripVerified: data.roundTripVerified };
}
export function decompiledSource(value: unknown): DecompiledSource {
  const data = result(value);
  if ((data.kind !== "basic" && data.kind !== "assembly") || typeof data.source !== "string" || new TextEncoder().encode(data.source).length > MAX_SOURCE_BYTES || !Array.isArray(data.warnings)) throw new Error("Invalid or oversized decompilation response.");
  return { kind: data.kind, source: data.source, warningCount: data.warnings.length };
}
export function verifiedFirmware(value: unknown): boolean {
  const data = result(value); return data.matchesVerifiedDragon32 === true || data.matchesVerifiedFirmware === true;
}

export interface BrowserMachine { id: string; label: string; cpu: "6809" | "6502" | "Z80"; extension: "cas" | "prg" | "tap" | "p" | "ssd"; manifest: string; runtime: "xroar" | "mame-z80" | "mame-bbc" | "chips-c64"; romSize: number; romNames: string[]; sample: string }
export function machineCatalog(value: unknown): BrowserMachine[] {
  const data = result(value);
  if (!Array.isArray(data.machines) || data.machines.length < 1 || data.machines.length > 16) throw new Error("Invalid computer catalogue.");
  const machines: BrowserMachine[] = [];
  for (const item of data.machines) {
    if (!record(item) || typeof item.id !== "string" || !/^[a-z0-9-]{1,64}$/.test(item.id) || machines.some((m) => m.id === item.id) || typeof item.label !== "string" || item.label.length > 80 || !["6809", "6502", "Z80"].includes(String(item.cpu)) || !["cas", "prg", "tap", "p", "ssd"].includes(String(item.extension)) || !["dragon.toml", "c64.toml", "bbc.toml", "spectrum.toml", "zx81.toml"].includes(String(item.manifest)) || !["xroar", "mame-z80", "mame-bbc", "chips-c64"].includes(String(item.runtime)) || !Number.isSafeInteger(item.romSize) || Number(item.romSize) < 4096 || Number(item.romSize) > 66496 || !Array.isArray(item.romNames) || item.romNames.length < 1 || item.romNames.length > 5 || !item.romNames.every((name: unknown) => typeof name === "string" && /^[a-z0-9_.-]{1,64}$/.test(name)) || typeof item.sample !== "string" || item.sample.length > MAX_SOURCE_BYTES) throw new Error("Invalid computer metadata.");
    if ((item.cpu !== "6809" && item.cpu !== "6502" && item.cpu !== "Z80") || (item.extension !== "cas" && item.extension !== "prg" && item.extension !== "tap" && item.extension !== "p" && item.extension !== "ssd") || (item.runtime !== "xroar" && item.runtime !== "mame-z80" && item.runtime !== "mame-bbc" && item.runtime !== "chips-c64") || typeof item.manifest !== "string") throw new Error("Invalid computer metadata.");
    const romNames: string[] = [];
    for (const name of item.romNames) { if (typeof name !== "string") throw new Error("Invalid firmware name."); romNames.push(name); }
    machines.push({ id: item.id, label: item.label, cpu: item.cpu, extension: item.extension, manifest: item.manifest, runtime: item.runtime, romSize: Number(item.romSize), romNames, sample: item.sample });
  }
  return machines;
}
