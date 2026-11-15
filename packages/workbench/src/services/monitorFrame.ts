import { originalRuntimeProfileId } from "./originalRuntime";
import { MONITOR_CASSETTE_LIMIT } from "./monitorProtocol";

export interface Cassette { filename: string; format: "tokenised-basic" | "ascii-basic" | "machine-code" | "ascii-data" | "cassette-stream"; bytes: number; sha256: string; pending: boolean }
export interface Recording { state: "idle" | "recording" | "ready" | "failed"; samples: number; sampleRate: number; message: string; cassette?: Cassette }
export interface MonitorFrame { frameCycles?: number; recording?: Recording; cassette?: Cassette; basic: boolean; qualifiedBasic?: boolean; extendedBasic?: boolean; state: "running" | "failed" | "stopped"; message: string; pixels: Uint8ClampedArray<ArrayBuffer>; text: string; audio: number[] }
export type SpectrumMachine = "spectrum48" | "spectrum128" | "spectrum-plus2";
export type DragonMachine = "dragon32" | "dragon64" | "dragon64-native";
export type MonitorMachine = DragonMachine | SpectrumMachine | "c64-pal" | "vectrex";
export function originalDragonTarget(value: unknown): value is DragonMachine { return value === "dragon32" || value === "dragon64" || value === "dragon64-native"; }
export function originalSpectrumTarget(value: unknown): value is SpectrumMachine {
  return value === "spectrum48" || value === "spectrum128" || value === "spectrum-plus2";
}
export function spectrumModelName(target: SpectrumMachine): string {
  return target === "spectrum48" ? "Spectrum 48K" : target === "spectrum128" ? "Spectrum 128K" : "Spectrum +2";
}
export function spectrumFirmwareId(target: SpectrumMachine): string {
  return target === "spectrum48" ? "eightbit-spectrum48-basic-0.1-dev" : target === "spectrum128" ? "eightbit-spectrum128-basic-0.1-dev" : "eightbit-spectrum-plus2-basic-0.1-dev";
}
function record(value: unknown): value is Record<string, unknown> { return typeof value === "object" && value !== null && !Array.isArray(value); }
function cassetteReceipt(value: unknown): Cassette | undefined {
  if (value === undefined || value === null) return undefined;
  if (!record(value) || typeof value.filename !== "string" || !/^[ -~]{0,8}$/.test(value.filename) || (value.format !== "tokenised-basic" && value.format !== "ascii-basic" && value.format !== "machine-code" && value.format !== "ascii-data" && value.format !== "cassette-stream") || typeof value.bytes !== "number" || !Number.isInteger(value.bytes) || value.bytes < 1 || value.bytes > MONITOR_CASSETTE_LIMIT || typeof value.sha256 !== "string" || !/^[a-f0-9]{64}$/.test(value.sha256) || typeof value.pending !== "boolean") throw new Error("Invalid cassette receipt.");
  return { filename: value.filename, format: value.format, bytes: value.bytes, sha256: value.sha256, pending: value.pending };
}
export function recordedDownload(value: unknown, machine: MonitorMachine = "dragon32"): Uint8Array<ArrayBuffer> {
  if (!record(value) || !record(value.recordingExport) || value.recordingExport.filename !== (originalSpectrumTarget(machine) ? "spectrum-recording.tap" : "dragon-recording.cas") || !Array.isArray(value.recordingExport.bytes) || !value.recordingExport.bytes.length || value.recordingExport.bytes.length > MONITOR_CASSETTE_LIMIT || !value.recordingExport.bytes.every((byte: unknown) => typeof byte === "number" && Number.isInteger(byte) && byte >= 0 && byte <= 255)) throw new Error("Invalid recorded cassette export.");
  return Uint8Array.from(value.recordingExport.bytes);
}
function recordingReceipt(value: unknown): Recording | undefined {
  if (value === undefined) return undefined;
  if (!record(value) || (value.state !== "idle" && value.state !== "recording" && value.state !== "ready" && value.state !== "failed") || typeof value.samples !== "number" || !Number.isSafeInteger(value.samples) || value.samples < 0 || value.samples > 8_640_000 || value.sampleRate !== 9600 || typeof value.message !== "string" || value.message.length > 256) throw new Error("Invalid cassette recording status.");
  const result: Recording = { state: value.state, samples: value.samples, sampleRate: value.sampleRate, message: value.message, cassette: cassetteReceipt(value.cassette) };
  if ((result.state === "ready") !== Boolean(result.cassette)) throw new Error("Invalid cassette recording receipt.");
  return result;
}
export function monitorFrame(value: unknown, sessionId: number, machine: MonitorMachine = "dragon32"): MonitorFrame {
  if (machine === "vectrex") {
    if (!record(value) || value.schemaVersion !== 1 || value.sessionId !== sessionId || (value.firmwareId !== "eightbit-vectrex-original-development-0.1" && value.firmwareId !== "eightbit-vectrex-original-development-0.2" && value.firmwareId !== "eightbit-vectrex-original-development-0.3" && value.firmwareId !== "eightbit-vectrex-original-development-0.4" && value.firmwareId !== "eightbit-vectrex-original-development-0.5") || value.width !== 256 || value.height !== 320 || value.frameTStates !== 30000 || !["running","failed","stopped"].includes(String(value.state)) || typeof value.message !== "string" || value.message.length > 256 || value.text !== "" || !Array.isArray(value.pixels) || value.pixels.length !== 256 * 320 * 4 || !value.pixels.every((byte: unknown) => typeof byte === "number" && Number.isInteger(byte) && byte >= 0 && byte <= 255)) throw new Error("Invalid or stale Vectrex frame.");
    if (value.audioSampleRate !== 44100 || !Array.isArray(value.audio) || value.audio.length > 8192 || !value.audio.every((sample: unknown) => typeof sample === "number" && Number.isInteger(sample) && sample >= -32768 && sample <= 32767)) throw new Error("Invalid Vectrex audio frame.");
    if (value.state !== "running" && value.state !== "failed" && value.state !== "stopped") throw new Error("Invalid Vectrex state.");
    return { frameCycles:30000,basic:false,state:value.state,message:value.message,text:"",pixels:new Uint8ClampedArray(value.pixels),audio:value.audio };
  }
  if (machine === "c64-pal") {
    // Native presentation retains at most 8192 PCM samples; browser slices are shorter.
    if (!record(value) || value.schemaVersion !== 1 || value.sessionId !== sessionId || value.firmwareId !== "eightbit-c64-original-development-0.5" || value.width !== 384 || value.height !== 272 || value.frameTStates !== 19656 || (value.state !== "running" && value.state !== "failed" && value.state !== "stopped") || typeof value.message !== "string" || value.message.length > 256 || typeof value.text !== "string" || value.text.length !== 1025 || !Array.isArray(value.pixels) || value.pixels.length !== 384 * 272 * 4 || !value.pixels.every((byte: unknown) => typeof byte === "number" && Number.isInteger(byte) && byte >= 0 && byte <= 255)) throw new Error("Invalid or stale C64 frame.");
    if (value.audioSampleRate !== 44100 || !Array.isArray(value.audio) || value.audio.length > 8192 || !value.audio.every((sample: unknown) => typeof sample === "number" && Number.isInteger(sample) && sample >= -32768 && sample <= 32767)) throw new Error("Invalid C64 audio frame.");
    return {frameCycles: 19656, basic: true, state: value.state, message: value.message, text: value.text, pixels: new Uint8ClampedArray(value.pixels), audio: value.audio};
  }
  if (originalSpectrumTarget(machine)) {
    if (!record(value) || value.schemaVersion !== 1 || value.sessionId !== sessionId || value.firmwareId !== spectrumFirmwareId(machine) || value.width !== 320 || value.height !== 256 || (value.state !== "running" && value.state !== "failed" && value.state !== "stopped") || typeof value.message !== "string" || value.message.length > 256 || typeof value.text !== "string" || value.text.length !== 768 || !Array.isArray(value.pixels) || value.pixels.length !== 320 * 256 * 4 || !value.pixels.every((byte: unknown) => typeof byte === "number" && Number.isInteger(byte) && byte >= 0 && byte <= 255)) throw new Error("Invalid or stale Spectrum frame.");
    if (value.audioSampleRate !== 44100 || !Array.isArray(value.audio) || value.audio.length > 8192 || !value.audio.every((sample: unknown) => typeof sample === "number" && Number.isInteger(sample) && sample >= -32768 && sample <= 32767)) throw new Error("Invalid Spectrum audio frame.");
    if (value.frameTStates !== undefined && (typeof value.frameTStates !== "number" || !Number.isInteger(value.frameTStates) || value.frameTStates < 32000 || value.frameTStates > 100000)) throw new Error("Invalid Spectrum frame timing.");
    return { frameCycles: value.frameTStates, basic: true, state: value.state, message: value.message, text: value.text, pixels: new Uint8ClampedArray(value.pixels), audio: value.audio, cassette: cassetteReceipt(value.cassette), recording: recordingReceipt(value.recording) };
  }
  if (!record(value) || value.schemaVersion !== 1 || value.sessionId !== sessionId || value.width !== 256 || value.height !== 192 || (value.state !== "running" && value.state !== "failed" && value.state !== "stopped") || typeof value.message !== "string" || value.message.length > 256 || typeof value.text !== "string" || value.text.length !== 528 || !Array.isArray(value.pixels) || value.pixels.length !== 256 * 192 * 4 || !value.pixels.every((byte: unknown) => typeof byte === "number" && Number.isInteger(byte) && byte >= 0 && byte <= 255)) throw new Error("Invalid or stale monitor frame.");
  if (machine === "dragon64" && value.firmwareId !== "eightbit-dragon64-basic-32k-dev-1" && value.firmwareId !== "eightbit-dragon64-basic-64k-dev-1" || machine === "dragon64-native" && value.firmwareId !== "eightbit-dragon64-basic-64k-dev-1") throw new Error("Dragon 64 frame does not match its target.");
  if (value.firmwareId !== undefined && value.firmwareId !== "eightbit-dragon32-monitor-0.1" && value.firmwareId !== "eightbit-dragon32-basic-0.1-dev" && !originalRuntimeProfileId(value.firmwareId)) throw new Error("Invalid firmware identity.");
  const audio = value.audio === undefined ? [] : value.audio;
  if (!Array.isArray(audio) || audio.length > 4096 || (audio.length > 0 && value.audioSampleRate !== 48_000) || (value.audioSampleRate !== undefined && value.audioSampleRate !== 48_000) || !audio.every((sample: unknown) => typeof sample === "number" && Number.isInteger(sample) && sample >= -32768 && sample <= 32767)) throw new Error("Invalid monitor audio frame.");
  const cassette = cassetteReceipt(value.cassette);
  if (cassette && value.firmwareId !== "eightbit-dragon32-basic-0.1-dev" && !originalRuntimeProfileId(value.firmwareId)) throw new Error("Invalid cassette receipt.");
  const recording = recordingReceipt(value.recording);
  if (recording && recording.state !== "idle" && value.firmwareId !== "eightbit-dragon32-basic-0.1-dev" && !originalRuntimeProfileId(value.firmwareId)) throw new Error("Invalid cassette recording status.");
  return { recording, cassette, extendedBasic: value.firmwareId === "eightbit-dragon32-basic-extended-1", qualifiedBasic: value.firmwareId === "eightbit-dragon32-basic-1", basic: value.firmwareId === "eightbit-dragon32-basic-0.1-dev" || originalRuntimeProfileId(value.firmwareId), state: value.state, message: value.message, pixels: new Uint8ClampedArray(value.pixels), text: value.text, audio };
}

