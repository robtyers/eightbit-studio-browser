import { buildResult, decompiledSource, highlights, result, verifiedFirmware } from "./contracts";

describe("Rust browser presentation boundary", () => {
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
