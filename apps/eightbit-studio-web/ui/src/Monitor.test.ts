import { MonitorWorker } from "./Monitor";

class FakeWorker {
  static latest: FakeWorker;
  onmessage: ((event: MessageEvent<unknown>)=>void)|null=null;
  onerror: (()=>void)|null=null;
  messages: unknown[]=[];
  terminated=false;
  constructor(){FakeWorker.latest=this;}
  postMessage(value:unknown){this.messages.push(value);}
  terminate(){this.terminated=true;}
}
beforeEach(()=>{vi.useFakeTimers();vi.stubGlobal("Worker",FakeWorker);});
afterEach(()=>{vi.useRealTimers();vi.unstubAllGlobals();});
it("constructs an explicit Spectrum worker with an immutable fresh-source launch",async()=>{
 const source={schemaVersion:1 as const,target:"spectrum48" as const,kind:"basic" as const,source:"10 STOP",sourcePath:"src/main.bas",revision:"a".repeat(64),sourceSha256:"b".repeat(64),cassetteSha256:"c".repeat(64)};
 const runtime=new MonitorWorker("spectrum-development",undefined,undefined,undefined,undefined,undefined,source);
 source.source="changed";
 const result=runtime.request({operation:"state"});
 expect(FakeWorker.latest.messages[0]).toMatchObject({firmware:"spectrum-development",launch:{source:"10 STOP",target:"spectrum48"},request:{sessionId:runtime.sessionId}});
 FakeWorker.latest.onmessage?.({data:{id:1,frame:{firmwareId:"eightbit-spectrum48-basic-0.1-dev"}}} as MessageEvent<unknown>);
 await result;runtime.dispose();expect(FakeWorker.latest.terminated).toBe(true);
});
it("owns a worker only after explicit execution and ignores stale replies",async()=>{const runtime=new MonitorWorker();const result=runtime.request({operation:"state"});const worker=FakeWorker.latest;worker.onmessage?.({data:{id:99,frame:"stale"}} as MessageEvent<unknown>);worker.onmessage?.({data:{id:1,frame:{schemaVersion:1}}} as MessageEvent<unknown>);expect(await result).toEqual({schemaVersion:1});runtime.dispose();expect(worker.terminated).toBe(true);await expect(runtime.request({operation:"state"})).rejects.toThrow("stopped");});
it("bounds requests and terminates a hung runtime",async()=>{const runtime=new MonitorWorker();const result=runtime.request({operation:"state"});const rejected=expect(result).rejects.toThrow();await expect(runtime.request({operation:"state"})).rejects.toThrow("busy");await vi.advanceTimersByTimeAsync(5001);await rejected;expect(FakeWorker.latest.terminated).toBe(true);});
it("fixes an explicit development profile for the owned worker lifetime",async()=>{const runtime=new MonitorWorker("basic-development");const result=runtime.request({operation:"state"});expect(FakeWorker.latest.messages[0]).toEqual({id:1,firmware:"basic-development",launch:{schemaVersion:1,backend:"original-development",target:"dragon32",cassette:null},request:{schemaVersion:1,sessionId:runtime.sessionId,command:{operation:"state"}}});FakeWorker.latest.onmessage?.({data:{id:1,frame:{firmwareId:"eightbit-dragon32-basic-0.1-dev"}}} as MessageEvent<unknown>);await result;runtime.dispose();expect(FakeWorker.latest.terminated).toBe(true);});

it("retains a worker across effect replay and disposes the previous firmware owner",async()=>{
  const original=new MonitorWorker();
  const firstRelease=original.retain();
  const firstResult=original.request({operation:"state"});
  const firstRejected=expect(firstResult).rejects.toThrow("stopped");
  const firstWorker=FakeWorker.latest;
  firstRelease();
  const secondRelease=original.retain();
  await vi.advanceTimersByTimeAsync(0);
  expect(firstWorker.terminated).toBe(false);
  const replacement=new MonitorWorker("basic-development");
  const replacementRelease=replacement.retain();
  const nextResult=replacement.request({operation:"state"});
  const nextRejected=expect(nextResult).rejects.toThrow("stopped");
  const nextWorker=FakeWorker.latest;
  secondRelease();
  await vi.advanceTimersByTimeAsync(0);
  await firstRejected;
  expect(firstWorker.terminated).toBe(true);
  expect(nextWorker.terminated).toBe(false);
  replacementRelease();
  await vi.advanceTimersByTimeAsync(0);
  await nextRejected;
  expect(nextWorker.terminated).toBe(true);
});

it("does not let an old failure callback terminate a newer request", async () => {
  const runtime = new MonitorWorker();
  const first = runtime.request({ operation: "state" });
  const worker = FakeWorker.latest;
  const lateFailure = worker.onerror;
  worker.onmessage?.({ data: { id: 1, frame: "first" } } as MessageEvent<unknown>);
  expect(await first).toBe("first");
  const next = runtime.request({ operation: "reset" });
  lateFailure?.();
  expect(worker.terminated).toBe(false);
  worker.onmessage?.({ data: { id: 2, frame: "next" } } as MessageEvent<unknown>);
  expect(await next).toBe("next");
  runtime.dispose();
});

it("mounts an immutable explicit build before the first execution slice, without typing commands", async () => {
  const cassette = Uint8Array.of(0x55, 0x3c);
  const runtime = new MonitorWorker("basic-development", cassette);
  cassette.fill(0);
  const result = runtime.request({ operation: "advance", cycles: 17920 });
  const worker = FakeWorker.latest;
  expect(worker.messages).toEqual([{ id: 1, firmware: "basic-development", launch: { schemaVersion: 1, backend: "original-development", target: "dragon32", cassette: null }, request: { schemaVersion: 1, sessionId: runtime.sessionId, command: { operation: "mount-cassette", bytes: [0x55, 0x3c] } } }]);
  worker.onmessage?.({ data: { id: 1, frame: { cassette: { filename: "ORIGINAL" } } } } as MessageEvent<unknown>);
  await Promise.resolve();
  expect(worker.messages[1]).toEqual({ id: 2, firmware: "basic-development", request: { schemaVersion: 1, sessionId: runtime.sessionId, command: { operation: "advance", cycles: 17920 } } });
  worker.onmessage?.({ data: { id: 2, frame: "running" } } as MessageEvent<unknown>);
  expect(await result).toBe("running");
  runtime.dispose();
});
it("cannot execute after an initial build cassette was rejected", async () => {
  const runtime = new MonitorWorker("basic-development", Uint8Array.of(0));
  const result = runtime.request({ operation: "advance", cycles: 17920 });
  const rejected = expect(result).rejects.toThrow("Invalid cassette");
  const worker = FakeWorker.latest;
  worker.onmessage?.({ data: { id: 1, error: "Invalid cassette" } } as MessageEvent<unknown>);
  await rejected;
  expect(worker.terminated).toBe(true);
  expect(worker.messages).toHaveLength(1);
  await expect(runtime.request({ operation: "advance", cycles: 17920 })).rejects.toThrow("stopped");
});

it("sends a digest-bound build in the typed launch only once", async () => {
  const cassette = Uint8Array.of(85, 60);
  const runtime = new MonitorWorker("basic-development", cassette, "a".repeat(64));
  cassette.fill(0);
  const first = runtime.request({ operation: "state" }); const worker = FakeWorker.latest;
  expect(worker.messages[0]).toEqual({ id: 1, firmware: "basic-development", launch: { schemaVersion: 1, backend: "original-development", target: "dragon32", cassette: { bytes: [85, 60], sha256: "a".repeat(64), buildId: null } }, request: { schemaVersion: 1, sessionId: runtime.sessionId, command: { operation: "state" } } });
  worker.onmessage?.({ data: { id: 1, frame: "ready" } } as MessageEvent<unknown>); await first;
  const next = runtime.request({ operation: "advance", cycles: 100 });
  expect(worker.messages[1]).not.toHaveProperty("launch");
  worker.onmessage?.({ data: { id: 2, frame: "running" } } as MessageEvent<unknown>); await next;
  runtime.dispose();
});

it("rejects oversized initial media before worker allocation or array expansion", async () => {
  const create = vi.fn(); vi.stubGlobal("Worker", create);
  const runtime = new MonitorWorker("basic-development", new Uint8Array(1048577), "a".repeat(64));
  await expect(runtime.request({ operation: "state" })).rejects.toThrow("1–1048576");
  expect(create).not.toHaveBeenCalled(); runtime.dispose();
});

it("owns fresh debug source once and aborts an in-flight debug call without accepting late replies", async () => {
  const source = { schemaVersion: 1 as const, source: "10 END", sourceSha256: "a".repeat(64), cassetteSha256: "b".repeat(64), breakpointLine: 10 };
  const runtime = new MonitorWorker("basic-development", undefined, undefined, source); source.source = "CHANGED";
  const initial = runtime.request({ operation: "state" }); const worker = FakeWorker.latest;
  expect(worker.messages[0]).toMatchObject({ debugSource: { source: "10 END" } }); expect(worker.messages[0]).not.toHaveProperty("launch");
  worker.onmessage?.({ data: { id: 1, frame: "ready" } } as MessageEvent<unknown>); await initial;
  const controller = new AbortController(); const pending = runtime.debug(null, controller.signal); const rejected = expect(pending).rejects.toThrow("stopped");
  expect(worker.messages[1]).toEqual({ id: 2, sessionId: runtime.sessionId, debug: { request: null } });
  await expect(runtime.request({ operation: "advance", cycles: 32 })).rejects.toThrow("busy");
  controller.abort(); await rejected; expect(worker.terminated).toBe(true);
  worker.onmessage?.({ data: { id: 2, encoded: "late" } } as MessageEvent<unknown>);
  await expect(runtime.debug(null)).rejects.toThrow();
});

it("normal original worker retains fresh source ownership and surfaces admission rejection without default fallback", async () => {
  const source = { schemaVersion: 1 as const, source: "10 END", sourceSha256: "a".repeat(64), cassetteSha256: "b".repeat(64) };
  const runtime = new MonitorWorker("original", undefined, undefined, source, qualifiedProfile); source.source = "CHANGED";
  const result = runtime.request({ operation: "state" });
  const worker = FakeWorker.latest;
  expect(worker.messages[0]).toEqual({ id: 1, sessionId: runtime.sessionId, originalProfile: qualifiedProfile });
  worker.onmessage?.({ data: { id: 1, originalProfile: qualifiedProfile } } as MessageEvent<unknown>);
  await Promise.resolve();
  expect(worker.messages[1]).toMatchObject({ firmware: "original", debugSource: { source: "10 END" } });
  expect(worker.messages[1]).not.toHaveProperty("launch");
  worker.onmessage?.({ data: { id: 2, error: "DME1400: original BASIC runtime is not qualified" } } as MessageEvent<unknown>);
  await expect(result).rejects.toThrow("not qualified");
  runtime.dispose(); expect(worker.terminated).toBe(true);
});

const qualifiedProfile = { schemaVersion: 1 as const, backend: "original" as const, target: "dragon32" as const, profileId: "eightbit-dragon32-basic-extended-1" as const, firmwareSha256: "a".repeat(64), coreVersion: "0.5.2" };
const freshSource = { schemaVersion: 1 as const, source: "10 END", sourceSha256: "a".repeat(64), cassetteSha256: "b".repeat(64) };
it("requires the selected identity before allocating a normal worker", async () => {
  const create = vi.fn(); vi.stubGlobal("Worker", create);
  const runtime = new MonitorWorker("original", undefined, undefined, freshSource);
  await expect(runtime.request({ operation: "state" })).rejects.toThrow("selected firmware identity");
  expect(create).not.toHaveBeenCalled();
});
it.each([undefined, null, { ...qualifiedProfile, profileId: "eightbit-dragon32-basic-1" }, { ...qualifiedProfile, firmwareSha256: "b".repeat(64) }, { ...qualifiedProfile, coreVersion: "0.5.3" }])("rejects stale or legacy identity acknowledgement before sending source", async profile => {
  const runtime = new MonitorWorker("original", undefined, undefined, freshSource, qualifiedProfile);
  const result = runtime.request({ operation: "state" }); const rejected = expect(result).rejects.toThrow("assets changed"); const worker = FakeWorker.latest;
  worker.onmessage?.({ data: { id: 1, originalProfile: profile, frame: "legacy" } } as MessageEvent<unknown>);
  await rejected; expect(worker.terminated).toBe(true); expect(worker.messages).toHaveLength(1);
  await expect(runtime.request({ operation: "state" })).rejects.toThrow("stopped");
});
it("bounds pending identity, ignores its late acknowledgement and permits a fresh owner", async () => {
  const runtime = new MonitorWorker("original", undefined, undefined, freshSource, qualifiedProfile);
  const pending = runtime.request({ operation: "state" }); const rejected = expect(pending).rejects.toThrow(); const worker = FakeWorker.latest;
  await vi.advanceTimersByTimeAsync(5001); await rejected; expect(worker.terminated).toBe(true);
  worker.onmessage?.({ data: { id: 1, originalProfile: qualifiedProfile } } as MessageEvent<unknown>);
  expect(worker.messages).toHaveLength(1);
  const selected = { ...qualifiedProfile }; const fresh = new MonitorWorker("original", undefined, undefined, freshSource, selected);
  selected.firmwareSha256 = "c".repeat(64);
  const ready = fresh.request({ operation: "state" }); const replacement = FakeWorker.latest;
  expect(replacement.messages[0]).toMatchObject({ originalProfile: qualifiedProfile });
  replacement.onmessage?.({ data: { id: 99, originalProfile: qualifiedProfile } } as MessageEvent<unknown>);
  expect(replacement.messages).toHaveLength(1);
  replacement.onmessage?.({ data: { id: 1, originalProfile: qualifiedProfile } } as MessageEvent<unknown>);
  await Promise.resolve(); expect(replacement.messages[1]).toMatchObject({ debugSource: freshSource });
  replacement.onmessage?.({ data: { id: 2, frame: "ready" } } as MessageEvent<unknown>); expect(await ready).toBe("ready"); fresh.dispose();
});
it.each([false, true])("cancels identity before construction even across the acknowledgement microtask", async acknowledged => {
  const runtime = new MonitorWorker("original", undefined, undefined, freshSource, qualifiedProfile);
  const pending = runtime.request({ operation: "state" }); const rejected = expect(pending).rejects.toThrow("stopped"); const worker = FakeWorker.latest;
  if (acknowledged) worker.onmessage?.({ data: { id: 1, originalProfile: qualifiedProfile } } as MessageEvent<unknown>);
  runtime.dispose(); await rejected;
  worker.onmessage?.({ data: { id: 1, originalProfile: qualifiedProfile } } as MessageEvent<unknown>);
  expect(worker.messages).toHaveLength(1); expect(worker.terminated).toBe(true);
});

it("retains exact imported bytes and sends them only after profile acknowledgement", async () => {
  const bytes = Uint8Array.of(0x86, 65, 0x39);
  const identity = { schemaVersion: 1 as const, input: { format: "raw" as const, loadAddress: 28672, executionAddress: 28672 }, inputSha256: "c".repeat(64), cassetteSha256: "d".repeat(64) };
  const runtime = new MonitorWorker("original", undefined, undefined, undefined, qualifiedProfile, { kind: "machine-code", identity, bytes });
  bytes.fill(0); identity.input.loadAddress = 1;
  const result = runtime.request({ operation: "state" }); const worker = FakeWorker.latest;
  expect(worker.messages).toHaveLength(1);
  worker.onmessage?.({ data: { id: 1, originalProfile: qualifiedProfile } } as MessageEvent<unknown>);
  await Promise.resolve();
  expect(worker.messages[1]).toMatchObject({ originalMedia: { kind: "machine-code", bytes: Uint8Array.of(0x86, 65, 0x39), identity: { input: { loadAddress: 28672 } } } });
  worker.onmessage?.({ data: { id: 2, frame: "ready" } } as MessageEvent<unknown>); await result;
  const next = runtime.request({ operation: "state" });
  expect(worker.messages[2]).not.toHaveProperty("originalMedia");
  worker.onmessage?.({ data: { id: 3, frame: "ready" } } as MessageEvent<unknown>); await next;
  runtime.dispose();
});

it("captures assembly source and breakpoint settings before profile negotiation", async () => {
  const source = { kind: "assembly-source" as const, schemaVersion: 1 as const, project: { schemaVersion: 1 as const, name: "Assembly", kind: "assembly" as const, source: "START\n rts\n", manifest: null, supportFiles: {} }, buildId: "c".repeat(64), cassetteSha256: "d".repeat(64), debug: { kind: "source" as const, sourceLine: 2 } };
  const runtime = new MonitorWorker("original", undefined, undefined, source, qualifiedProfile);
  source.project.source = "changed"; source.debug.sourceLine = 999;
  const result = runtime.request({ operation: "state" }); const worker = FakeWorker.latest;
  expect(worker.messages).toHaveLength(1);
  worker.onmessage?.({ data: { id: 1, originalProfile: qualifiedProfile } } as MessageEvent<unknown>);
  await Promise.resolve();
  expect(worker.messages[1]).toMatchObject({ debugSource: { project: { source: "START\n rts\n" }, debug: { kind: "source", sourceLine: 2 } } });
  worker.onmessage?.({ data: { id: 2, frame: "ready" } } as MessageEvent<unknown>); await result;
  runtime.dispose();
});

it.each(["dragon64", "dragon64-native"] as const)("binds %s source to the selected worker firmware mode", async target => {
  const profile = { ...qualifiedProfile, target, profileId: target === "dragon64" ? "eightbit-dragon64-basic-32k-dev-1" as const : "eightbit-dragon64-basic-64k-dev-1" as const };
  const runtime = new MonitorWorker("original", undefined, undefined, freshSource, profile);
  const pending = runtime.request({ operation: "state" }); const worker = FakeWorker.latest;
  worker.onmessage?.({ data: { id: 1, originalProfile: profile } } as MessageEvent<unknown>);
  await Promise.resolve(); await Promise.resolve();
  expect(worker.messages[1]).toMatchObject({ debugSource: { ...freshSource, target } });
  worker.onmessage?.({ data: { id: 2, frame: { state: "running" } } } as MessageEvent<unknown>);
  await pending; runtime.dispose();
});
