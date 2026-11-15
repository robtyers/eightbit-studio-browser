import { buildResult, cassetteAudio, decompiledSource, highlights, machineCatalog, result, verifiedFirmware } from "./contracts";

describe("Rust browser presentation boundary", () => {
  it("accepts bounded binary cassette audio and rejects JSON sample arrays", () => {
    const response = (audio: unknown) => ({ schemaVersion: 1, ok: true, result: { audio } });
    const audio = new Uint8Array(44);
    expect(cassetteAudio(response(audio))).toBe(audio);
    for (const invalid of [[], Array(44).fill(0), new Uint8Array(43), new Uint8Array(64 * 1024 * 1024 + 1), null, "audio"]) expect(() => cassetteAudio(response(invalid))).toThrow();
  });
  it("accepts the bounded BBC machine profile and rejects oversized ROM catalogues", () => {
    const machine = { id: "bbc-model-b", label: "BBC Micro Model B (32 KB)", cpu: "6502", extension: "ssd", manifest: "bbc.toml", runtime: "mame-bbc", romSize: 66496, romNames: ["os12.rom", "basic2.rom", "dnfs120.rom", "cm62024.bin", "saa5050"], sample: "10 END\n" };
    const value = (item: unknown) => ({ schemaVersion: 1, ok: true, result: { machines: [item] } });
    expect(machineCatalog(value(machine))[0]).toEqual(machine);
    for (const change of [{ romSize: 66497 }, { romNames: [...machine.romNames, "extra.rom"] }, { manifest: "unknown.toml" }, { runtime: "external" }]) expect(() => machineCatalog(value({ ...machine, ...change }))).toThrow();
  });
  it("rejects malformed and failed responses without artefacts", () => {
    for (const value of [null, {}, { schemaVersion: 2, ok: true, result: {} }, { schemaVersion: 1, ok: true, result: [] }]) expect(() => result(value)).toThrow();
    expect(() => result({ schemaVersion: 1, ok: false, message: "DSW1002" })).toThrow("DSW1002");
    expect(() => result({ schemaVersion: 1, ok: false, message: "Correct BASIC", diagnostics: [{ row: 2, column: 4, code: "DSB1001", message: "Missing quote" }] })).toThrow("Row 2, column 4: DSB1001 Missing quote");
    expect(() => buildResult({ schemaVersion: 1, ok: true, result: { cassette: [256] } })).toThrow();
  });
  it("accepts only byte-safe builds, typed listings and a verified receipt", () => {
    expect(buildResult({ schemaVersion: 1, ok: true, result: { summary: { tokenBytes: 3, cassetteSha256: "a".repeat(64) }, cassette: [0, 255], roundTripVerified: true } }).cassette).toEqual(new Uint8Array([0, 255]));
    expect(decompiledSource({ schemaVersion: 1, ok: true, result: { kind: "assembly", source: "fcb $00", warnings: ["unknown"] } }).warningCount).toBe(1);
    expect(() => decompiledSource({ schemaVersion: 1, ok: true, result: { kind: "shell", source: "", warnings: [] } })).toThrow();
    expect(verifiedFirmware({ schemaVersion: 1, ok: true, result: { matchesVerifiedDragon32: false } })).toBe(false);
  });
  it("rejects overlapping, oversized and untrusted colouring ranges", () => {
    const value = (ranges: unknown[]) => ({ schemaVersion: 1, ok: true, result: { highlights: ranges } });
    expect(highlights(value([{ start: 0, end: 2, kind: "number" }]), "10 END")).toEqual([{ start: 0, end: 2, kind: "number" }]);
    for (const ranges of [
      [{ start: -1, end: 2, kind: "number" }],
      [{ start: 0, end: 100, kind: "number" }],
      [{ start: 0, end: 2, kind: "html" }],
      [{ start: 0, end: 2, kind: "number" }, { start: 1, end: 3, kind: "string" }],
      Array.from({ length: 6001 }, () => ({ start: 0, end: 2, kind: "number" })),
    ]) expect(() => highlights(value(ranges), "10 END")).toThrow();
    expect(() => highlights(value([{ start: 0, end: 2, kind: "number" }]), "10 £")).toThrow();
  });
});

it("validates imported identities and assembly build addresses without accepting runtime authority", () => {
  const identity = { schemaVersion: 1, input: { format: "raw", loadAddress: 28672, executionAddress: 28672 }, inputSha256: "a".repeat(64), cassetteSha256: "b".repeat(64) };
  const response = (machineCode: unknown) => ({ schemaVersion: 1, ok: true, result: { kind: "assembly", source: "START\n rts\n", warnings: [], machineCode, preview: { loadAddress: 28672, executionAddress: 28672 } } });
  expect(decompiledSource(response(identity)).machineCode).toEqual(identity);
  for (const bad of [{ ...identity, input: { format: "cartridge" } }, { ...identity, input: { ...identity.input, command: "run" } }, { ...identity, inputSha256: "x" }, { ...identity, schemaVersion: 2 }, { ...identity, input: { ...identity.input, loadAddress: 65536 } }, { ...identity, bytes: [0x39] }]) expect(() => decompiledSource(response(bad))).toThrow();
});

it("distinguishes the original Vectrex cartridge receipt from a BASIC round trip", () => {
  const response = (cartridgeVerified: unknown) => ({ schemaVersion: 1, ok: true, result: { summary: { tokenBytes: 204, cassetteSha256: "a".repeat(64) }, cassette: [103, 32], roundTripVerified: false, cartridgeVerified } });
  expect(buildResult(response(true))).toMatchObject({ cartridgeVerified: true, roundTripVerified: false });
  for (const bad of ["true", 1, {}, null]) expect(() => buildResult(response(bad))).toThrow();
  const machine = { id: "vectrex", label: "Vectrex", cpu: "6809", extension: "bin", manifest: "vectrex.toml", runtime: "original-vectrex", romSize: 8192, romNames: ["executive.rom"], sample: " org 0\n", assemblySample: " org 0\n" };
  expect(machineCatalog({ schemaVersion: 1, ok: true, result: { machines: [machine] } })[0]).toEqual(machine);
});
