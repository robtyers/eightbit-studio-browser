import { monitorWorkerHost, type MonitorCore } from "./monitorWorkerHost";
const request = (id: number, sessionId = 7) => ({ id, firmware: "basic-development", request: { schemaVersion: 1, sessionId, command: { operation: "state" } } });
function core(): MonitorCore { return { command: vi.fn(() => '{"state":"running"}'), debugState: vi.fn(() => '{"revision":"0"}'), debugCommand: vi.fn(() => '{"revision":"1"}') }; }
it("fixes the explicit C64 source owner and rejects replacement and oversized launches",async()=>{
 const runtime=core(),load=vi.fn(async()=>runtime),post=vi.fn(),receive=monitorWorkerHost(load,post);
 await receive({...request(1),firmware:"c64-original",launch:{source:"x".repeat(512*1024)}});
 expect(load).not.toHaveBeenCalled();
 const launch={schemaVersion:1,target:"c64-pal",source:"10 END",sourceSha256:"a".repeat(64),prgSha256:"b".repeat(64)};
 await receive({...request(2),firmware:"c64-original",launch});
 expect(load).toHaveBeenCalledWith(7,"c64-original",launch,undefined,undefined);
 await receive({...request(3),firmware:"c64-original",launch});
 await receive({...request(4,8),firmware:"c64-original"});
 await receive(request(5));
 expect(load).toHaveBeenCalledTimes(1);expect(runtime.command).toHaveBeenCalledTimes(1);
 expect(post.mock.calls.slice(-3).every(([value])=>typeof value.error==="string")).toBe(true);
});
it("fixes the Spectrum development owner and rejects launch replacement or foreign profiles",async()=>{
 const runtime=core(),load=vi.fn(async()=>runtime),post=vi.fn();const receive=monitorWorkerHost(load,post);
 const launch={schemaVersion:1,target:"spectrum48",kind:"basic",source:"10 STOP",sourcePath:"src/main.bas",revision:"a".repeat(64),sourceSha256:"b".repeat(64),cassetteSha256:"c".repeat(64)};
 await receive({...request(1),firmware:"spectrum-development",launch});
 expect(load).toHaveBeenCalledWith(7,"spectrum-development",launch,undefined,undefined);
 await receive({...request(2),firmware:"spectrum-development",launch});
 await receive({...request(3,8),firmware:"spectrum-development"});
 await receive(request(4));
 expect(runtime.command).toHaveBeenCalledTimes(1);expect(load).toHaveBeenCalledTimes(1);
 expect(post.mock.calls.slice(-3).every(([value])=>typeof value.error==="string")).toBe(true);
});
it("boots a blank Spectrum development session and bounds source before construction",async()=>{
 const load=vi.fn(async()=>core()),post=vi.fn();const receive=monitorWorkerHost(load,post);
 await receive({...request(1),firmware:"spectrum-development",launch:{source:"x".repeat(512*1024)}});
 expect(load).not.toHaveBeenCalled();
 await receive({...request(2),firmware:"spectrum-development"});
 expect(load).toHaveBeenCalledWith(7,"spectrum-development",undefined,undefined,undefined);
});
it("fixes owner and fresh source construction while keeping one execution route", async () => {
  const runtime = core(); const load = vi.fn(async () => runtime); const post = vi.fn(); const receive = monitorWorkerHost(load, post);
  const debugSource = { schemaVersion: 1, source: "10 END", sourceSha256: "a".repeat(64), cassetteSha256: "b".repeat(64), breakpointLine: 10 };
  await receive({ ...request(1), debugSource });
  expect(load).toHaveBeenCalledWith(7, "basic-development", expect.any(Object), debugSource, undefined);
  await receive({ id: 2, sessionId: 7, debug: { request: null } });
  await receive({ id: 3, sessionId: 7, debug: { request: '{"command":{"operation":"continue"}}' } });
  expect(runtime.debugState).toHaveBeenCalledTimes(1); expect(runtime.debugCommand).toHaveBeenCalledTimes(1); expect(runtime.command).toHaveBeenCalledTimes(1);
  await receive(request(4, 8)); await receive({ ...request(5), debugSource }); await receive({ id: 6, sessionId: 8, debug: { request: null } });
  expect(post.mock.calls.slice(-3).every(([value]) => typeof value.error === "string")).toBe(true);
  expect(load).toHaveBeenCalledTimes(1);
});
it("rejects concurrency and failed initialisation cannot retry with another payload", async () => {
  let reject: (error: Error) => void = () => {};
  const load = vi.fn(() => new Promise<MonitorCore>((_yes, no) => { reject = no; })); const post = vi.fn(); const receive = monitorWorkerHost(load, post);
  const first = receive(request(1)); await receive(request(2));
  expect(post).toHaveBeenLastCalledWith({ id: 2, error: "Monitor is busy or unavailable." });
  reject(new Error("unsafe source detail")); await first; await receive(request(3));
  expect(load).toHaveBeenCalledTimes(1); expect(JSON.stringify(post.mock.calls)).not.toContain("unsafe source detail");
});
it("rejects malformed debug envelopes and oversized replies without changing runtime", async () => {
  const runtime = core(); const post = vi.fn(); const receive = monitorWorkerHost(async () => runtime, post); await receive(request(1));
  for (const data of [{ id: 2, sessionId: 7, debug: {} }, { id: 3, sessionId: 7, debug: { request: 1 } }, { id: 4, sessionId: 7, debug: { request: null }, sourceMap: {} }]) await receive(data);
  expect(runtime.debugState).not.toHaveBeenCalled(); expect(runtime.debugCommand).not.toHaveBeenCalled();
  runtime.debugState = () => " ".repeat(12 * 1024 * 1024 + 1);
  await receive({ id: 5, sessionId: 7, debug: { request: null } });
  expect(post).toHaveBeenLastCalledWith({ id: 5, error: expect.any(String) });
  runtime.debugState = () => null; await receive({ id: 6, sessionId: 7, debug: { request: null } }); expect(post).toHaveBeenLastCalledWith({ id: 6, encoded: null });
});

it("normal original construction requires fresh source and closes the owner after firmware rejection", async () => {
  const load = vi.fn(async () => { throw "DME1400: original BASIC runtime is not qualified"; });
  const post = vi.fn(); const receive = monitorWorkerHost(load, post, async () => ({ schemaVersion: 1, profile: qualifiedProfile }));
  await receive({ ...request(1), firmware: "original" }); expect(load).not.toHaveBeenCalled();
  const debugSource = { schemaVersion: 1, source: "10 END", sourceSha256: "a".repeat(64), cassetteSha256: "b".repeat(64) };
  await receive({ id: 10, sessionId: 7, originalProfile: qualifiedProfile });
  await receive({ ...request(2), firmware: "original", debugSource });
  expect(load).toHaveBeenCalledWith(7, "original", expect.any(Object), debugSource, undefined);
  expect(post).toHaveBeenLastCalledWith({ id: 2, error: "DME1400: original BASIC runtime is not qualified" });
  await receive(request(3)); expect(load).toHaveBeenCalledTimes(1);
  expect(post).toHaveBeenLastCalledWith({ id: 3, error: "Monitor is busy or unavailable." });
});

const qualifiedProfile = { schemaVersion: 1, backend: "original", target: "dragon32", profileId: "eightbit-dragon32-basic-1", firmwareSha256: "a".repeat(64), coreVersion: "0.5.2" };
const freshSource = { schemaVersion: 1, source: "10 END", sourceSha256: "a".repeat(64), cassetteSha256: "b".repeat(64) };
it.each(["eightbit-dragon32-basic-1", "eightbit-dragon32-basic-extended-1"])("checks %s identity before construction and fixes it to the exact session", async profileId => {
  const profile = { ...qualifiedProfile, profileId };
  const runtime = core(), load = vi.fn(async () => runtime), post = vi.fn();
  const capability = vi.fn(async () => ({ schemaVersion: 1, profile }));
  const receive = monitorWorkerHost(load, post, capability);
  await receive({ ...request(1), firmware: "original", debugSource: freshSource }); expect(load).not.toHaveBeenCalled();
  await receive({ id: 2, sessionId: 7, originalProfile: profile });
  expect(post).toHaveBeenLastCalledWith({ id: 2, originalProfile: profile }); expect(load).not.toHaveBeenCalled();
  await receive({ ...request(3, 8), firmware: "original", debugSource: freshSource }); expect(load).not.toHaveBeenCalled();
  await receive({ ...request(4), firmware: "original", debugSource: freshSource }); expect(load).toHaveBeenCalledTimes(1);
  await receive({ id: 5, sessionId: 7, originalProfile: profile });
  expect(post).toHaveBeenLastCalledWith({ id: 5, error: expect.any(String) }); expect(capability).toHaveBeenCalledTimes(1);
});
it.each([null, { ...qualifiedProfile, profileId: "eightbit-dragon32-basic-extended-1" }, { ...qualifiedProfile, firmwareSha256: "b".repeat(64) }, { ...qualifiedProfile, coreVersion: "0.5.3" }])("mismatched worker capability closes this owner without constructing a guest", async profile => {
  const load = vi.fn(async () => core()), post = vi.fn();
  const receive = monitorWorkerHost(load, post, async () => ({ schemaVersion: 1, profile }));
  await receive({ id: 1, sessionId: 7, originalProfile: qualifiedProfile });
  expect(post).toHaveBeenLastCalledWith({ id: 1, error: expect.stringContaining("assets changed") });
  await receive({ ...request(2), firmware: "original", debugSource: freshSource }); await receive(request(3));
  expect(load).not.toHaveBeenCalled();
});
it.each([null, { ...qualifiedProfile, extra: true }])("rejects malformed identity before reading capability and keeps legacy development payloads unchanged", async malformed => {
  const load = vi.fn(async () => core()), capability = vi.fn(async () => ({ schemaVersion: 1, profile: qualifiedProfile })), post = vi.fn();
  const receive = monitorWorkerHost(load, post, capability);
  await receive({ id: 1, sessionId: 7, originalProfile: malformed }); expect(capability).not.toHaveBeenCalled(); expect(load).not.toHaveBeenCalled();
  const fresh = monitorWorkerHost(load, post, capability); await fresh(request(2)); expect(load).toHaveBeenCalledTimes(1); expect(capability).not.toHaveBeenCalled();
});
it("rejects overlapping launch during identity loading and cannot recover a failed loader in place", async () => {
  let reject: (error: Error) => void = () => {};
  const load = vi.fn(async () => core()), post = vi.fn();
  const receive = monitorWorkerHost(load, post, () => new Promise((_resolve, no) => { reject = no; }));
  const pending = receive({ id: 1, sessionId: 7, originalProfile: qualifiedProfile });
  await receive({ ...request(2), firmware: "original", debugSource: freshSource }); expect(load).not.toHaveBeenCalled();
  reject(new Error("private module details")); await pending;
  await receive({ id: 3, sessionId: 7, originalProfile: qualifiedProfile });
  expect(post).toHaveBeenLastCalledWith({ id: 3, error: "Monitor is busy or unavailable." }); expect(JSON.stringify(post.mock.calls)).not.toContain("private module details");
});

it("bounds media separately from metadata and fixes the original-media owner", async () => {
  const runtime = core(), load = vi.fn(async () => runtime), post = vi.fn();
  const receive = monitorWorkerHost(load, post, async () => ({ schemaVersion: 1, profile: qualifiedProfile }));
  await receive({ id: 1, sessionId: 7, originalProfile: qualifiedProfile });
  const originalMedia = { kind: "machine-code", bytes: Uint8Array.of(0x39), identity: { schemaVersion: 1, input: { format: "raw", loadAddress: 28672, executionAddress: 28672 }, inputSha256: "a".repeat(64), cassetteSha256: "b".repeat(64) } };
  await receive({ ...request(2), firmware: "original", originalMedia: { ...originalMedia, bytes: new Uint8Array(8 * 1024 * 1024 + 1) } });
  expect(load).not.toHaveBeenCalled();
  await receive({ ...request(3), firmware: "original", originalMedia, debugSource: freshSource });
  expect(load).not.toHaveBeenCalled();
  await receive({ ...request(4), firmware: "original", originalMedia });
  expect(load).toHaveBeenCalledWith(7, "original", expect.any(Object), undefined, originalMedia);
  await receive({ ...request(5), firmware: "original", originalMedia });
  expect(load).toHaveBeenCalledOnce(); expect(post).toHaveBeenLastCalledWith({ id: 5, error: expect.any(String) });
});

it("checks the D64 mode before constructing a worker guest", async () => {
  const profile = { ...qualifiedProfile, target: "dragon64-native", profileId: "eightbit-dragon64-basic-64k-dev-1" };
  const load = vi.fn(async () => core()), post = vi.fn(), capability = vi.fn(async () => ({ schemaVersion: 1, profile }));
  const receive = monitorWorkerHost(load, post, capability);
  await receive({ id: 1, sessionId: 7, originalProfile: profile });
  expect(capability).toHaveBeenCalledWith("dragon64-native");
  await receive({ ...request(2), firmware: "original", debugSource: freshSource });
  expect(load).not.toHaveBeenCalled();
  await receive({ ...request(3), firmware: "original", debugSource: { ...freshSource, target: "dragon64-native" } });
  expect(load).toHaveBeenCalledOnce();
});
