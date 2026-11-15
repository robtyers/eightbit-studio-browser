import { Activity, useEffect, useState, type ComponentType, type ReactNode } from "react";
import { OriginalHostConnection } from "@eightbitstudio/workbench/services/originalMachineDebug";
import { DevelopmentHostConnection } from "@eightbitstudio/workbench/services/developmentHost";
import type { DevelopmentDebugger } from "./BrowserWorkbench";
vi.mock("@eightbitstudio/workbench/components/MonitorConsole", () => ({ MonitorConsole: ({ transport, onStop }: { transport: (command: { operation: "type"; text: string }) => Promise<unknown>; onStop: () => void }) => <><button onClick={() => { void transport({ operation: "type", text: "CLOAD" }); }}>Load test cassette</button><button onClick={onStop}>Stop test console</button></> }));
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { BrowserWorkbench, type EmulatorProps } from "./BrowserWorkbench";
import type { BrowserEngine, BrowserRequest, BrowserMachine } from "./contracts";
import { recentProjects } from "./projects";
import SAMPLE_SOURCE from "./guess-the-number.bas?raw";
import cpuMark from "../../workbench/branding/cpu.svg";

const BUILD = { schemaVersion: 1, ok: true, result: { summary: { tokenBytes: 22, cassetteSha256: "b".repeat(64) }, cassette: [1, 2, 3], roundTripVerified: true } };
function Emulator({ onState }: EmulatorProps) { return <button onClick={() => onState("ready")}>Fake emulator ready</button>; }
const DRAGON: BrowserMachine = { id: "dragon32", label: "Dragon 32", cpu: "6809", extension: "cas", manifest: "dragon.toml", runtime: "xroar", romSize: 16384, romNames: ["d32.rom"], sample: SAMPLE_SOURCE };
const C64: BrowserMachine = { id: "c64-pal", label: "Commodore 64 (PAL)", cpu: "6502", extension: "prg", manifest: "c64.toml", runtime: "chips-c64", romSize: 20480, romNames: ["basic-901226-01.bin", "kernal-901227-03.bin", "chargen-901225-01.bin"], sample: '10 PRINT "C64"\n20 END\n' };
const SPECTRUM: BrowserMachine = { id: "spectrum48", label: "ZX Spectrum 48K (UK)", cpu: "Z80", extension: "tap", manifest: "spectrum.toml", runtime: "mame-z80", romSize: 16384, romNames: ["spectrum.rom"], sample: '10 PRINT "SPECTRUM"\n20 STOP\n' };
const ZX81: BrowserMachine = { id: "zx81-1k", label: "ZX81 (1K)", cpu: "Z80", extension: "p", manifest: "zx81.toml", runtime: "mame-z80", romSize: 8192, romNames: ["zx81.rom"], sample: '10 PRINT "ZX81"\n20 STOP\n' };
const BBC: BrowserMachine = { id: "bbc-model-b", label: "BBC Micro Model B (32 KB)", cpu: "6502", extension: "ssd", manifest: "bbc.toml", runtime: "mame-bbc", romSize: 66496, romNames: ["os12.rom", "basic2.rom", "dnfs120.rom", "cm62024.bin", "saa5050"], sample: '10 MODE 7\n20 PRINT "BBC"\n30 END\n' };
function mount(handler: BrowserEngine["request"], machines: BrowserMachine[] = [DRAGON], Monitor?: Parameters<typeof BrowserWorkbench>[0]["Monitor"], DevelopmentDebugger?: DevelopmentDebugger, original: Pick<Parameters<typeof BrowserWorkbench>[0], "OriginalRuntime" | "originalProfile" | "originalProfiles" | "SpectrumMonitor" | "C64Monitor" | "VectrexMonitor"> = {}, wrapper?: ComponentType<{ children: ReactNode }>) {
  return render(<BrowserWorkbench machines={machines} engine={{ request: (request, bytes, signal) => request.operation === "presentation" ? Promise.resolve({ schemaVersion: 1, ok: true, result: { highlights: [] } }) : request.operation === "create-project" ? Promise.resolve({ schemaVersion: 1, ok: true, result: { entryPath: request.kind === "basic" ? "src/main.bas" : "src/recovered.asm", project: { schemaVersion: request.target === "dragon32" ? 1 : 2, ...(request.target !== "dragon32" ? { target: request.target, supportFiles: {} } : {}), name: request.name, kind: request.kind, source: request.source, manifest: request.kind === "basic" ? "test-manifest" : null } } }) : request.operation === "import-project" ? Promise.resolve({ schemaVersion: 1, ok: true, result: { entryPath: "src/main.bas", project: JSON.parse(new TextDecoder().decode(bytes)) } }) : handler(request, bytes, signal) }} Emulator={Emulator} Monitor={Monitor} DevelopmentDebugger={DevelopmentDebugger} {...original} assetBase="/studio/" />, { wrapper });
}
async function begin(blank = false) { fireEvent.click(screen.getByRole("button", { name: "Create new project" })); if (blank) fireEvent.change(screen.getByLabelText("Start with"), { target: { value: "blank" } }); fireEvent.click(screen.getByRole("button", { name: "Create project" })); await screen.findByRole("textbox", { name: "Dragon 32 BASIC source" }); }
function saveMenu() { fireEvent.click(screen.getByRole("button", { name: "Save work" })); return screen.getByRole("dialog", { name: "Save your work" }); }
function closeMenu() { fireEvent.keyDown(document, { key: "Escape" }); }
function file(size: number, name: string, text?: string) {
  const bytes = text === undefined ? new Uint8Array(size) : new TextEncoder().encode(text);
  const value = new File([bytes], name);
  Object.defineProperty(value, "arrayBuffer", { value: async () => bytes.buffer });
  return value;
}
async function check() { fireEvent.click(screen.getByRole("button", { name: "Check & build" })); await screen.findByText(/^Game file ready\. Code/); }
beforeEach(() => localStorage.clear());
afterEach(() => vi.restoreAllMocks());
it.each((["spectrum48","spectrum128","spectrum-plus2"] as const).flatMap(id=>(["basic","assembly"] as const).map(kind=>({id,kind}))))("uses normal Play for a fresh built-in $id $kind build without ROM setup", async ({id,kind}) => {
  const assemblySample = "ORG 24576\nLD A,42\nLD ($C000),A\nRET\n";
  const machine: BrowserMachine = {...SPECTRUM, id, label:id, defaultRuntime:"original", assemblySample};
  const started = vi.fn(); const dispose = vi.fn();
  const SpectrumMonitor: NonNullable<Parameters<typeof BrowserWorkbench>[0]["SpectrumMonitor"]> = ({source,onStop,onRuntime}) => {
    useEffect(()=>{started(source);onRuntime?.(dispose);return()=>onRuntime?.();},[source,onRuntime]);
    return <button onClick={onStop}>Stop original Spectrum</button>;
  };
  const handler = vi.fn(async()=>({...BUILD,result:{...BUILD.result,summary:{...BUILD.result.summary,sourceSha256:"a".repeat(64)},...(kind==="assembly"?{assembly:{buildId:"c".repeat(64),loadAddress:24576,entryAddress:24576}}:{})}}));
  mount(handler,[DRAGON,machine],undefined,undefined,{SpectrumMonitor});
  fireEvent.click(screen.getByRole("button",{name:"Create new project"}));
  fireEvent.change(screen.getByLabelText("Computer"),{target:{value:id}});
  if(kind==="assembly")fireEvent.change(screen.getByLabelText("Start with"),{target:{value:"assembly"}});
  fireEvent.click(screen.getByRole("button",{name:"Create project"}));
  const editor=await screen.findByRole("textbox",{name:kind==="assembly"?"Z80 assembly source":`${machine.label} BASIC source`});
  await check();fireEvent.click(screen.getByRole("button",{name:"Play"}));
  await screen.findByRole("button",{name:"Stop original Spectrum"});
  expect(started).toHaveBeenLastCalledWith(expect.objectContaining({target:id,kind,sourceSha256:"a".repeat(64),cassetteSha256:"b".repeat(64)}));
  expect(screen.queryByRole("dialog",{name:/Set up your virtual/})).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button",{name:"Stop original Spectrum"}));
  expect(dispose).toHaveBeenCalledOnce();
  fireEvent.click(screen.getByRole("button",{name:"Computer setup"}));
  const setup=screen.getByRole("region",{name:"System ROM"});
  expect(within(setup).getByLabelText("Project emulator")).toHaveValue("original");
  if(kind==="basic") {
    fireEvent.change(within(setup).getByLabelText("Project emulator"),{target:{value:"mame-z80"}});
    fireEvent.click(screen.getByRole("button",{name:"Play"}));
    expect(screen.getByRole("dialog",{name:`Set up your virtual ${machine.label}`})).toBeVisible();
    closeMenu();
  } else {
    expect(within(setup).getByRole("option",{name:"MAME · supplied ROM"})).toBeDisabled();
  }
  fireEvent.change(editor,{target:{value:kind==="assembly"?"ORG 24576\nRET\n":"10 STOP\n"}});
  expect(screen.getByRole("button",{name:"Play"})).toBeDisabled();
});
it("uses the shared Spectrum debugger with source-bound steps and disposes it after an edit",async()=>{
 const sample="ORG 24576\nLD A,42\nLD ($C000),A\nRET\n";
 const machine={...SPECTRUM,assemblySample:sample};const dispose=vi.fn();const started=vi.fn();
 const controls=vi.fn();
 const SpectrumMonitor:NonNullable<Parameters<typeof BrowserWorkbench>[0]["SpectrumMonitor"]>=({source,onRuntime,onDebugger,onInspection})=>{
  useEffect(()=>{
   started(source);onRuntime?.(dispose);
   if(!source)return;
   let stopId=1;
   const emit=(state:"running"|"stopped",line:number|null)=>onInspection?.({schemaVersion:1,sessionId:7,firmwareId:"eightbit-spectrum48-basic-0.1-dev",state:"running",debug:{state,revision:source.revision,stopId,valuesValid:state==="stopped",breakpointVerified:state==="stopped",sourceLine:line,breakpointAddress:24576,registers:state==="running"?null:{af:42*256,bc:0,de:0,hl:0,"af'":0,"bc'":0,"de'":0,"hl'":0,ix:0,iy:23610,sp:65534,pc:24576},memory:state==="running"?null:{startAddress:24576,bytes:Array<number>(64).fill(0)}}},7,source);
   onDebugger?.({command:async(operation,id)=>{controls(operation,id);stopId++;emit(operation==="continue"?"running":"stopped",operation==="continue"?null:3);}});
   emit("stopped",2);return()=>{onDebugger?.();onRuntime?.();};
  },[source,onRuntime,onDebugger,onInspection]);
  return <p>Original Spectrum game window</p>;
 };
 const handler=vi.fn(async()=>({...BUILD,result:{...BUILD.result,summary:{...BUILD.result.summary,sourceSha256:"a".repeat(64)},assembly:{buildId:"c".repeat(64),loadAddress:24576,entryAddress:24576}}}));
 mount(handler,[DRAGON,machine],undefined,undefined,{SpectrumMonitor});
 fireEvent.click(screen.getByRole("button",{name:"Create new project"}));fireEvent.change(screen.getByLabelText("Computer"),{target:{value:"spectrum48"}});fireEvent.change(screen.getByLabelText("Start with"),{target:{value:"assembly"}});fireEvent.click(screen.getByRole("button",{name:"Create project"}));await screen.findByRole("textbox",{name:"Z80 assembly source"});await waitFor(()=>expect(screen.getByRole("button",{name:"Check & build"})).toBeEnabled());await check();
 fireEvent.click(screen.getByRole("tab",{name:"Debug"}));fireEvent.change(screen.getByLabelText("Spectrum source breakpoint line"),{target:{value:"2"}});fireEvent.click(screen.getAllByRole("button",{name:"Debug"})[0]!);
 await screen.findByText("● Verified breakpoint at $6000");expect(started).toHaveBeenCalledWith(expect.objectContaining({breakpointLine:2}));
 fireEvent.click(screen.getByRole("button",{name:"Step"}));await waitFor(()=>expect(controls).toHaveBeenCalledWith("step",1));
 fireEvent.click(screen.getByRole("button",{name:"Continue"}));await waitFor(()=>expect(screen.getByRole("button",{name:"Pause"})).toBeEnabled());expect(screen.getByText("Registers are available only at a verified stop.")).toBeVisible();
 fireEvent.click(screen.getByRole("button",{name:"Pause"}));await waitFor(()=>expect(screen.getByRole("button",{name:"Step"})).toBeEnabled());
 fireEvent.change(screen.getByRole("textbox",{name:"Z80 assembly source"}),{target:{value:"ORG 24576\nRET\n"}});expect(dispose).toHaveBeenCalledOnce();expect(screen.queryByText("● Verified breakpoint at $6000")).not.toBeInTheDocument();
});
it("builds Spectrum assembly from the family template and explicitly runs a fresh development source without a ROM",async()=>{
 const assemblySample="ORG 24576\nLD A,2\nOUT (254),A\nRET\n";
 const machine={...SPECTRUM,assemblySample};
 const dispose=vi.fn(); const started=vi.fn();
 const SpectrumMonitor:NonNullable<Parameters<typeof BrowserWorkbench>[0]["SpectrumMonitor"]>=({source,onStop,onRuntime})=>{
  useEffect(()=>{started(source);onRuntime?.(dispose);return()=>{onRuntime?.();};},[source,onRuntime]);
  return <button onClick={onStop}>Stop original Spectrum</button>;
 };
 const handler=vi.fn(async()=>({...BUILD,result:{...BUILD.result,summary:{...BUILD.result.summary,sourceSha256:"a".repeat(64)},assembly:{buildId:"c".repeat(64),loadAddress:24576,entryAddress:24576}}}));
 mount(handler,[DRAGON,machine],undefined,undefined,{SpectrumMonitor});
 fireEvent.click(screen.getByRole("button",{name:"Create new project"}));
 fireEvent.change(screen.getByLabelText("Computer"),{target:{value:"spectrum48"}});
 fireEvent.change(screen.getByLabelText("Start with"),{target:{value:"assembly"}});
 fireEvent.click(screen.getByRole("button",{name:"Create project"}));
 const editor=await screen.findByRole("textbox",{name:"Z80 assembly source"});
 expect(editor).toHaveValue(assemblySample);await check();
 fireEvent.click(screen.getByRole("button",{name:"Computer setup"}));
 fireEvent.click(within(screen.getByRole("region",{name:"System ROM"})).getByRole("button",{name:"Run current build in original Spectrum (development)"}));
 await screen.findByRole("button",{name:"Stop original Spectrum"});
 expect(started).toHaveBeenLastCalledWith(expect.objectContaining({target:"spectrum48",kind:"assembly",source:assemblySample,sourceSha256:"a".repeat(64),revision:"c".repeat(64),cassetteSha256:"b".repeat(64)}));
 expect(screen.queryByRole("dialog",{name:/Set up your virtual/})).not.toBeInTheDocument();
 fireEvent.change(editor,{target:{value:"ORG 24576\nRET\n"}});
 expect(dispose).toHaveBeenCalledOnce();
 expect(screen.queryByRole("button",{name:"Stop original Spectrum"})).not.toBeInTheDocument();
 expect(screen.queryByRole("button",{name:"Run current build in original Spectrum (development)"})).not.toBeInTheDocument();
});

it("boots the optional original monitor explicitly without a project or ROM", () => {
  const request = vi.fn();
  function Monitor({ onStop }: { onStop: () => void }) { return <button onClick={onStop}>Stop test monitor</button>; }
  render(<BrowserWorkbench machines={[DRAGON]} engine={{ request }} Emulator={Emulator} Monitor={Monitor} assetBase="/studio/" />);
  expect(screen.queryByRole("button", { name: "Stop test monitor" })).not.toBeInTheDocument();
  fireEvent.click(within(screen.getByRole("navigation", { name: "Workbench views" })).getByRole("button", { name: "Computer setup" }));
  fireEvent.click(within(screen.getByRole("region", { name: "System ROM" })).getByRole("button", { name: "Start original monitor" }));
  expect(screen.getByRole("button", { name: "Stop test monitor" })).toBeVisible();
  expect(request).not.toHaveBeenCalled();
  expect(screen.getByRole("button", { name: "Play" })).toBeDisabled();
  fireEvent.click(screen.getByRole("button", { name: "Stop test monitor" }));
  expect(screen.queryByRole("button", { name: "Stop test monitor" })).not.toBeInTheDocument();
  expect(within(screen.getByRole("region", { name: "System ROM" })).getByLabelText("Choose ROM file… for Dragon 32")).toBeEnabled();
});

it("starts with a project welcome, inert shell and hides technical imports", async () => {
  const request = vi.fn(); mount(request);
  expect(screen.getByRole("navigation", { name: "Workbench views" })).toBeInTheDocument();
  expect(screen.getByRole("heading", { name: "Get Started" })).toBeInTheDocument();
  expect(screen.getByRole("img", { name: "Eightbit Studio" }).querySelector("img")?.getAttribute("src")).toBe(cpuMark);
  expect(screen.getByLabelText("Raw load address")).not.toBeVisible();
  expect(screen.queryByRole("button", { name: "Debug" })).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Play" })).toBeDisabled();
  expect(screen.getByRole("button", { name: "Check & build" })).toBeDisabled();
  expect(request).not.toHaveBeenCalled();
  await begin(true);
  expect(screen.getByRole("textbox", { name: "Dragon 32 BASIC source" })).toHaveValue("");
});

it.each([DRAGON, C64, BBC, SPECTRUM, ZX81])("explains ROM setup and keeps source and licence links available for $label", (machine) => {
  const request = vi.fn(); mount(request, [machine]);
  fireEvent.click(within(screen.getByRole("navigation", { name: "Workbench views" })).getByRole("button", { name: "Computer setup" }));
  const setup = screen.getByRole("complementary", { name: "Computer setup" });
  const rom = within(setup).getByRole("region", { name: "System ROM" });
  expect(rom).toHaveTextContent("built-in software");
  expect(rom).toHaveTextContent("editing and saving work without it");
  const input = within(rom).getByLabelText(`Choose ROM ${machine.romNames.length > 1 ? "files" : "file"}… for ${machine.label}`);
  expect(input.closest("label")).toHaveTextContent(machine.romNames.length > 1 ? "Choose ROM files…" : "Choose ROM file…");
  expect(within(rom).getByRole("button", { name: "Remove ROM" })).toBeDisabled();
  const source = within(setup).getByRole("region", { name: "Open-source software" });
  expect(within(source).getAllByRole("link")).toHaveLength(1);
  expect(within(source).getByRole("link", { name: "Source code on GitHub" })).toHaveAttribute("href", "https://github.com/robtyers/eightbit-studio-browser/releases/tag/browser-0.5.3-20261115.1");
  expect(request).not.toHaveBeenCalled();
});

it("allows the extensionless BBC character ROM and checks all five files before playing", async () => {
  const inspected: Uint8Array[] = [];
  mount(async (request, bytes) => {
    if (request.operation === "firmware") { inspected.push(bytes.slice()); return { schemaVersion: 1, ok: true, result: { matchesVerifiedFirmware: true } }; }
    return BUILD;
  }, [BBC]);
  fireEvent.click(screen.getByRole("button", { name: "Create new project" }));
  fireEvent.change(screen.getByLabelText("Computer"), { target: { value: BBC.id } });
  fireEvent.click(screen.getByRole("button", { name: "Create project" }));
  await screen.findByRole("textbox", { name: `${BBC.label} BASIC source` });
  await check(); fireEvent.click(screen.getByRole("button", { name: "Play" }));
  const picker = screen.getByLabelText(`Choose ROM files… for ${BBC.label}`);
  expect(picker).not.toHaveAttribute("accept");
  fireEvent.change(picker, { target: { files: BBC.romNames.map((name, index) => file(index === 4 ? 960 : 16384, name)).reverse() } });
  await screen.findAllByText("System ROM checked. Choose Play to start your game.");
  expect(inspected[0]).toHaveLength(66496);
  expect(localStorage.getItem("roms")).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "Play" }));
  await screen.findByRole("button", { name: "Fake emulator ready" });
});

it("shows ROM readiness and requires setup again after Remove ROM", async () => {
  mount(async (request) => request.operation === "firmware" ? { schemaVersion: 1, ok: true, result: { matchesVerifiedDragon32: true } } : BUILD);
  await begin(); await check();
  fireEvent.click(screen.getByRole("button", { name: "Computer setup" }));
  const rom = screen.getByRole("region", { name: "System ROM" });
  fireEvent.change(within(rom).getByLabelText("Choose ROM file… for Dragon 32"), { target: { files: [file(16384, "generated-test.rom")] } });
  await within(rom).findByText("ROM ready · kept only until this page closes.");
  expect(screen.queryByRole("button", { name: "Fake emulator ready" })).not.toBeInTheDocument();
  fireEvent.click(within(rom).getByRole("button", { name: "Remove ROM" }));
  expect(within(rom).getByRole("button", { name: "Remove ROM" })).toBeDisabled();
  expect(within(rom).queryByText(/ROM ready/)).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Play" }));
  expect(screen.getByRole("dialog", { name: "Set up your virtual Dragon 32" })).toBeInTheDocument();
});

it("collapses Explorer when entering a compact viewport and permits manual reopening", () => {
  const width = window.innerWidth;
  const mounted = mount(vi.fn());
  expect(screen.getByRole("button", { name: "Explorer" })).toHaveAttribute("aria-expanded", "true");
  try {
    Object.defineProperty(window, "innerWidth", { configurable: true, writable: true, value: 390 });
    fireEvent(window, new Event("resize"));
    expect(screen.getByRole("button", { name: "Explorer" })).toHaveAttribute("aria-expanded", "false");
    fireEvent.click(screen.getByRole("button", { name: "Explorer" }));
    fireEvent(window, new Event("resize"));
    expect(screen.getByRole("button", { name: "Explorer" })).toHaveAttribute("aria-expanded", "true");
    const remove = vi.spyOn(window, "removeEventListener");
    mounted.unmount(); expect(remove).toHaveBeenCalledWith("resize", expect.any(Function));
  } finally { window.innerWidth = width; mounted.unmount(); }
});

it("checks the original sample through Rust, exposes save choices and invalidates edited builds", async () => {
  const request = vi.fn(async () => BUILD); mount(request); await begin();
  expect(screen.getByRole("textbox", { name: "Dragon 32 BASIC source" })).toHaveValue(SAMPLE_SOURCE);
  await check();
  expect(request).toHaveBeenCalledWith(expect.objectContaining({ schemaVersion: 1, operation: "test-project", project: expect.objectContaining({ source: SAMPLE_SOURCE }) }), new Uint8Array(), expect.any(AbortSignal));
  expect(screen.getByText(/Cassette SHA-256:/)).not.toBeVisible();
  expect(within(saveMenu()).getByRole("button", { name: /Save playable game file/ })).toBeEnabled(); closeMenu();
  fireEvent.change(screen.getByRole("textbox", { name: "Dragon 32 BASIC source" }), { target: { value: "10 END\n" } });
  expect(screen.getByRole("button", { name: "Play" })).toBeDisabled();
  expect(within(saveMenu()).getByRole("button", { name: /Save playable game file/ })).toBeDisabled();
});

it("downloads loading audio only from a current build and recovers from failed audio export", async () => {
  const createURL = vi.fn((_blob: Blob) => "blob:loading-audio");
  Object.defineProperty(URL, "createObjectURL", { configurable: true, writable: true, value: createURL });
  const clicked = vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {});
  let fail = true;
  const request = vi.fn(async (input: BrowserRequest) => input.operation === "cassette-audio"
    ? fail ? { schemaVersion: 1, ok: false, message: "DSW1025: cannot render this cassette" } : { schemaVersion: 1, ok: true, result: { audio: new Uint8Array(44) } }
    : BUILD);
  mount(request); await begin();
  expect(within(saveMenu()).getByRole("button", { name: /Save cassette loading audio/ })).toBeDisabled(); closeMenu();
  await check();
  fireEvent.click(within(saveMenu()).getByRole("button", { name: /Save cassette loading audio/ }));
  await screen.findAllByText("DSW1025: cannot render this cassette");
  expect(createURL).not.toHaveBeenCalled();
  fail = false;
  fireEvent.click(within(saveMenu()).getByRole("button", { name: /Save cassette loading audio/ }));
  await waitFor(() => expect(clicked).toHaveBeenCalledOnce());
  expect(clicked.mock.instances[0]).toHaveAttribute("download", "game.wav");
  expect(request).toHaveBeenLastCalledWith({ schemaVersion: 1, operation: "cassette-audio", target: "dragon32", cassetteSha256: BUILD.result.summary.cassetteSha256 }, Uint8Array.from(BUILD.result.cassette), expect.any(AbortSignal));
  expect(createURL.mock.calls[0]?.[0]).toMatchObject({ type: "audio/wav", size: 44 });
  fireEvent.change(screen.getByRole("textbox", { name: "Dragon 32 BASIC source" }), { target: { value: "10 END\n" } });
  expect(within(saveMenu()).getByRole("button", { name: /Save cassette loading audio/ })).toBeDisabled();
});

it.each(["edit", "cancel"])("discards late audio after %s and permits a fresh request", async (action) => {
  const createURL = vi.fn((_blob: Blob) => "blob:loading-audio");
  Object.defineProperty(URL, "createObjectURL", { configurable: true, writable: true, value: createURL });
  vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {});
  let complete: ((value: unknown) => void) | undefined;
  let signal: AbortSignal | undefined;
  const request = vi.fn((input: BrowserRequest, _bytes: Uint8Array, abort: AbortSignal) => {
    if (input.operation !== "cassette-audio") return Promise.resolve(BUILD);
    signal = abort;
    return new Promise<unknown>(resolve => { complete = resolve; });
  });
  mount(request); await begin(); await check();
  fireEvent.click(within(saveMenu()).getByRole("button", { name: /Save cassette loading audio/ }));
  if (action === "edit") fireEvent.change(screen.getByRole("textbox", { name: "Dragon 32 BASIC source" }), { target: { value: "10 END\n" } });
  else fireEvent.click(screen.getByRole("button", { name: "Cancel operation" }));
  expect(signal?.aborted).toBe(true);
  await act(async () => complete?.({ schemaVersion: 1, ok: true, result: { audio: new Uint8Array(44) } }));
  expect(createURL).not.toHaveBeenCalled();
  await check();
  fireEvent.click(within(saveMenu()).getByRole("button", { name: /Save cassette loading audio/ }));
  await act(async () => complete?.({ schemaVersion: 1, ok: true, result: { audio: new Uint8Array(44) } }));
  expect(createURL).toHaveBeenCalledOnce();
});

it("keeps Dragon loading audio out of other machine save options", async () => {
  mount(async () => BUILD, [C64]);
  fireEvent.click(screen.getByRole("button", { name: "Create new project" }));
  fireEvent.change(screen.getByLabelText("Computer"), { target: { value: C64.id } });
  fireEvent.click(screen.getByRole("button", { name: "Create project" }));
  await screen.findByRole("textbox", { name: "Commodore 64 (PAL) BASIC source" });
  expect(within(saveMenu()).queryByRole("button", { name: /Save cassette loading audio/ })).not.toBeInTheDocument();
});

it("cancels stale work after an edit, discards failed builds and permits recovery", async () => {
  let resolve: ((value: unknown) => void) | undefined; let signal: AbortSignal | undefined; let fail = false;
  const request = vi.fn((_: BrowserRequest, __: Uint8Array, abort: AbortSignal) => {
    signal = abort;
    return request.mock.calls.length === 1 ? new Promise<unknown>((done) => { resolve = done; }) : Promise.resolve(fail ? { schemaVersion: 1, ok: false, message: "Broken BASIC", diagnostics: [{ row: 1, column: 4, code: "DSB1001", message: "Missing quote" }] } : BUILD);
  });
  mount(request); await begin();
  fireEvent.click(screen.getByRole("button", { name: "Check & build" }));
  fireEvent.change(screen.getByRole("textbox", { name: "Dragon 32 BASIC source" }), { target: { value: "10 PRINT 42\n" } });
  expect(signal?.aborted).toBe(true); await act(async () => { resolve?.(BUILD); });
  expect(screen.getByRole("button", { name: "Play" })).toBeDisabled();
  fail = true; fireEvent.click(screen.getByRole("button", { name: "Check & build" }));
  await screen.findByRole("alert"); expect(screen.getByRole("alert")).toHaveTextContent("Broken BASIC");
  fireEvent.click(screen.getByRole("button", { name: "Line 1, column 4: Missing quote" }));
  const input = screen.getByRole("textbox", { name: "Dragon 32 BASIC source" }); expect(input).toHaveFocus(); expect((input as HTMLTextAreaElement).selectionStart).toBe(3);
  expect(screen.getByText("DSB1001")).not.toBeVisible();
  expect(screen.getByRole("button", { name: "Play" })).toBeDisabled();
  fail = false; await check(); expect(screen.getByRole("button", { name: "Play" })).toBeEnabled();
});

it("guides ROM selection after Play, never starts automatically, and tears down on Stop", async () => {
  mount(async (value) => value.operation === "firmware" ? { schemaVersion: 1, ok: true, result: { matchesVerifiedDragon32: true } } : BUILD);
  await begin(); await check(); fireEvent.click(screen.getByRole("button", { name: "Play" }));
  const setup = screen.getByRole("dialog", { name: "Set up your virtual Dragon 32" });
  expect(setup).toHaveTextContent("original computer’s built-in software");
  expect(setup).toHaveTextContent("Choose your Dragon 32 system ROM to play. Your game file is ready.");
  fireEvent.change(within(setup).getByLabelText("Choose ROM file… for Dragon 32"), { target: { files: [file(16384, "generated-test.rom")] } });
  await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  expect(screen.queryByRole("button", { name: "Fake emulator ready" })).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Play" }));
  fireEvent.click(screen.getByRole("button", { name: "Fake emulator ready" }));
  expect(screen.getByRole("button", { name: "Stop" })).toBeEnabled();
  fireEvent.click(screen.getByRole("button", { name: "Stop" }));
  expect(screen.queryByRole("button", { name: "Fake emulator ready" })).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Play" })); fireEvent(window, new Event("pagehide"));
  expect(screen.queryByRole("button", { name: "Fake emulator ready" })).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Play" })); expect(screen.getByRole("dialog")).toBeInTheDocument();
});

it("rejects a wrong ROM and cancels pending setup when the dialogue closes", async () => {
  let pending: ((value: unknown) => void) | undefined; let signal: AbortSignal | undefined; let deferred = false;
  mount((request, _bytes, abort) => request.operation === "firmware" ? deferred ? new Promise((resolve) => { pending = resolve; signal = abort; }) : Promise.resolve({ schemaVersion: 1, ok: true, result: { matchesVerifiedDragon32: false } }) : Promise.resolve(BUILD));
  await begin(); await check(); fireEvent.click(screen.getByRole("button", { name: "Play" }));
  fireEvent.change(screen.getByLabelText("Choose ROM file… for Dragon 32"), { target: { files: [file(16384, "generated-test.rom")] } });
  await screen.findAllByText(/does not match the supported/);
  expect(screen.queryByRole("button", { name: "Fake emulator ready" })).not.toBeInTheDocument();
  deferred = true;
  fireEvent.change(screen.getByLabelText("Choose ROM file… for Dragon 32"), { target: { files: [file(16384, "generated-test.rom")] } });
  await waitFor(() => expect(signal).toBeDefined()); closeMenu(); expect(signal?.aborted).toBe(true);
  await act(async () => { pending?.({ schemaVersion: 1, ok: true, result: { matchesVerifiedDragon32: true } }); });
  fireEvent.click(screen.getByRole("button", { name: "Play" })); expect(screen.getByRole("dialog")).toBeInTheDocument();
});

it("recovers projects automatically between sessions and reports quota failures", async () => {
  const mounted = mount(async () => BUILD); await begin();
  fireEvent.change(screen.getByRole("textbox", { name: "Dragon 32 BASIC source" }), { target: { value: "10 END\n" } });
  await waitFor(() => expect(recentProjects()[0]?.project.source).toBe("10 END\n"));
  expect(screen.getByText("Saved on this device")).toBeInTheDocument();
  expect(JSON.stringify(recentProjects())).not.toContain("firmware");
  mounted.unmount(); mount(async () => BUILD);
  fireEvent.click(screen.getByRole("button", { name: /My first game.*Dragon 32.*On this device/ }));
  await screen.findByRole("textbox", { name: "Dragon 32 BASIC source" });
  expect(screen.getByRole("textbox", { name: "Dragon 32 BASIC source" })).toHaveValue("10 END\n");
  const storage = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => { throw new Error("Quota exceeded"); });
  fireEvent.change(screen.getByRole("textbox", { name: "Dragon 32 BASIC source" }), { target: { value: "10 PRINT 7\n" } });
  await screen.findByText("Local recovery failed — download a copy");
  expect(recentProjects()[0]?.project.source).toBe("10 END\n"); storage.mockRestore();
});

it("resumes local recovery after effect replay and revokes pending saves on unmount", async () => {
  let show: (visible: boolean) => void = () => {};
  function Lifecycle({ children }: { children: ReactNode }) {
    const [visible, setVisible] = useState(true); show = setVisible;
    return <Activity mode={visible ? "visible" : "hidden"}>{children}</Activity>;
  }
  const mounted = mount(async () => BUILD, [DRAGON], undefined, undefined, {}, Lifecycle);
  await begin(); await screen.findByText("Saved on this device");
  const id = recentProjects()[0]!.id;
  // React refresh also cleans up/restarts effects while retaining the open project.
  await act(async () => { show(false); });
  await act(async () => { show(true); });
  fireEvent.change(screen.getByRole("textbox", { name: "Dragon 32 BASIC source" }), { target: { value: "10 PRINT 42\n" } });
  await waitFor(() => expect(recentProjects().find(entry => entry.id === id)?.project.source).toBe("10 PRINT 42\n"));
  expect(screen.getByText("Saved on this device")).toBeVisible();
  expect(within(saveMenu()).getByText(/^Saved on this device\./)).toBeVisible(); closeMenu();
  vi.useFakeTimers();
  try {
    fireEvent.change(screen.getByRole("textbox", { name: "Dragon 32 BASIC source" }), { target: { value: "10 PRINT 99\n" } });
    mounted.unmount();
    await act(async () => { await vi.advanceTimersByTimeAsync(400); });
    expect(recentProjects().find(entry => entry.id === id)?.project.source).toBe("10 PRINT 42\n");
  } finally { vi.useRealTimers(); }
});

it("keeps recovery inert and ignores late results after cancellation", async () => {
  let resolve: ((value: unknown) => void) | undefined;
  mount(() => new Promise((done) => { resolve = done; })); await begin();
  fireEvent.click(screen.getByRole("button", { name: "Recover source…" }));
  fireEvent.change(screen.getByLabelText("Choose game or recording…"), { target: { files: [file(8, "original.cas")] } });
  await waitFor(() => expect(screen.getByRole("button", { name: "Cancel operation" })).toBeInTheDocument());
  fireEvent.click(screen.getByRole("button", { name: "Cancel operation" }));
  await act(async () => { resolve?.({ schemaVersion: 1, ok: true, result: { kind: "basic", source: "10 END\n", warnings: [] } }); });
  expect(screen.getByRole("textbox", { name: "Dragon 32 BASIC source" })).toHaveValue(SAMPLE_SOURCE);
  expect(screen.queryByRole("button", { name: "Edit this copy" })).not.toBeInTheDocument();
});

it("previews source and drafts, requires confirmation before replacement, and requires a fresh build before running assembly", async () => {
  mount(async () => BUILD); await begin();
  fireEvent.click(screen.getByRole("button", { name: "Load existing project…" }));
  fireEvent.change(screen.getByLabelText("Choose source file…"), { target: { files: [file(0, "saved.bas", "10 END\n")] } });
  await screen.findByRole("textbox", { name: "Imported source preview" });
  expect(screen.getByRole("textbox", { name: "Dragon 32 BASIC source" })).toHaveValue(SAMPLE_SOURCE);
  fireEvent.change(screen.getByRole("textbox", { name: "Dragon 32 BASIC source" }), { target: { value: SAMPLE_SOURCE + "\n" } });
  const confirm = vi.spyOn(window, "confirm").mockReturnValue(false);
  fireEvent.click(screen.getByRole("button", { name: "Edit this copy" })); expect(screen.getByRole("textbox", { name: "Dragon 32 BASIC source" })).toHaveValue(SAMPLE_SOURCE + "\n");
  confirm.mockReturnValue(true); fireEvent.click(screen.getByRole("button", { name: "Edit this copy" }));
  await waitFor(() => expect(screen.getByRole("textbox", { name: "Dragon 32 BASIC source" })).toHaveValue("10 END\n"));
  localStorage.setItem("eightbit-studio.browser.dragon32.draft.v1", JSON.stringify({ schemaVersion: 1, kind: "assembly", source: "fcb $00" }));
  fireEvent.click(screen.getByRole("button", { name: "Load existing project…" })); fireEvent.click(screen.getByRole("button", { name: "Restore browser draft" }));
  fireEvent.click(screen.getByRole("button", { name: "Edit this copy" }));
  await screen.findByRole("textbox", { name: "6809 assembly source" });
  expect(screen.getByRole("textbox", { name: "6809 assembly source" })).toHaveValue("fcb $00");
  expect(screen.getByRole("button", { name: "Check & build" })).toBeEnabled();
  expect(screen.getByRole("button", { name: "Play" })).toBeDisabled();
});

it("contains dialogue focus, restores the opener and supports keyboard save", async () => {
  mount(async () => BUILD); await begin();
  const opener = screen.getByRole("button", { name: "Save work" }); opener.focus();
  fireEvent.keyDown(window, { key: "s", ctrlKey: true });
  const dialog = screen.getByRole("dialog"); const close = within(dialog).getByRole("button", { name: "Close Save your work" });
  expect(close).toHaveFocus(); fireEvent.keyDown(document, { key: "Tab", shiftKey: true });
  expect(within(dialog).getByRole("button", { name: /Keep a separate local copy/ })).toHaveFocus();
  fireEvent.keyDown(document, { key: "Tab" }); expect(close).toHaveFocus();
  closeMenu(); expect(opener).toHaveFocus(); expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
});

it("offers processor groups and sends the chosen computer and sample through the shared build boundary", async () => {
  const request = vi.fn(async () => BUILD); mount(request, [DRAGON, C64, SPECTRUM]);
  fireEvent.click(screen.getByRole("button", { name: "Create new project" }));
  const select = screen.getByLabelText("Computer");
  expect(within(select).getByRole("group", { name: "6502" })).toBeInTheDocument();
  expect(within(select).getByRole("group", { name: "Z80" })).toBeInTheDocument();
  fireEvent.change(select, { target: { value: C64.id } });
  fireEvent.click(screen.getByRole("button", { name: "Create project" }));
  expect(await screen.findByRole("textbox", { name: `${C64.label} BASIC source` })).toHaveValue(C64.sample);
  expect(screen.getByRole("button", { name: "c64.toml" })).toBeInTheDocument();
  await check();
  expect(request).toHaveBeenCalledWith(expect.objectContaining({ operation: "test-project", project: expect.objectContaining({ schemaVersion: 2, target: C64.id, source: C64.sample }) }), new Uint8Array(), expect.any(AbortSignal));
  fireEvent.click(screen.getByRole("button", { name: "Play" }));
  const setup = screen.getByRole("dialog", { name: `Set up your virtual ${C64.label}` });
  expect(within(setup).getByLabelText(`Choose ROM files… for ${C64.label}`)).toHaveAttribute("multiple");
});

it("packs named C64 ROM files in profile order and forgets them when switching computer through Home", async () => {
  const request = vi.fn(async (value: BrowserRequest) => value.operation === "firmware" ? { schemaVersion: 1, ok: true, result: { matchesVerifiedFirmware: true } } : BUILD);
  mount(request, [DRAGON, C64, SPECTRUM]);
  fireEvent.click(screen.getByRole("button", { name: "Create new project" }));
  fireEvent.change(screen.getByLabelText("Computer"), { target: { value: C64.id } });
  fireEvent.click(screen.getByRole("button", { name: "Create project" }));
  await screen.findByRole("textbox", { name: `${C64.label} BASIC source` }); await check();
  fireEvent.click(screen.getByRole("button", { name: "Play" }));
  fireEvent.change(screen.getByLabelText(`Choose ROM files… for ${C64.label}`), { target: { files: [file(4096, "chargen-901225-01.bin"), file(8192, "kernal-901227-03.bin"), file(8192, "basic-901226-01.bin")] } });
  await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  expect(request.mock.calls.find(([value]) => value.operation === "firmware")?.[0]).toEqual({ schemaVersion: 1, operation: "firmware", target: C64.id });
  fireEvent.click(screen.getByRole("button", { name: "Play" }));
  expect(screen.getByRole("button", { name: "Fake emulator ready" })).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Stop" }));
  await screen.findByText(/Saved on this device/);
  fireEvent.click(screen.getByRole("button", { name: "Project home" }));
  fireEvent.click(screen.getByRole("button", { name: "Create new project" }));
  fireEvent.change(screen.getByLabelText("Computer"), { target: { value: SPECTRUM.id } });
  fireEvent.click(screen.getByRole("button", { name: "Create project" }));
  await screen.findByRole("textbox", { name: `${SPECTRUM.label} BASIC source` }); await check();
  fireEvent.click(screen.getByRole("button", { name: "Play" }));
  expect(screen.getByRole("dialog", { name: `Set up your virtual ${SPECTRUM.label}` })).toBeInTheDocument();
  expect(screen.queryByRole("button", { name: "Fake emulator ready" })).not.toBeInTheDocument();
});

it("requires a computer choice for a plain source listing and retains that choice through preview", async () => {
  mount(async () => BUILD, [DRAGON, C64, SPECTRUM]);
  fireEvent.click(screen.getByRole("button", { name: /^Load existing project$/ }));
  fireEvent.change(screen.getByLabelText("Computer for source files"), { target: { value: C64.id } });
  fireEvent.change(screen.getByLabelText("Choose source file…"), { target: { files: [file(0, "game.bas", C64.sample)] } });
  await screen.findByRole("textbox", { name: "Imported source preview" });
  fireEvent.click(screen.getByRole("button", { name: "Edit this copy" }));
  expect(await screen.findByRole("textbox", { name: `${C64.label} BASIC source` })).toHaveValue(C64.sample);
});
it("launches the optional development BASIC profile explicitly and keeps ROM setup available",()=>{const request=vi.fn(async()=>({}));function Monitor({onStop,firmware}:{onStop:()=>void;firmware?:"monitor"|"basic-development"}){return <button onClick={onStop}>Stop {firmware}</button>;}render(<BrowserWorkbench machines={[DRAGON]} engine={{request}} Emulator={Emulator} Monitor={Monitor} assetBase="/studio/"/>);expect(screen.queryByRole("button",{name:"Stop basic-development"})).not.toBeInTheDocument();fireEvent.click(within(screen.getByRole("navigation",{name:"Workbench views"})).getByRole("button",{name:"Computer setup"}));fireEvent.click(within(screen.getByRole("region",{name:"System ROM"})).getByRole("button",{name:"Start original BASIC (development)"}));expect(screen.getByRole("button",{name:"Stop basic-development"})).toBeVisible();fireEvent.click(screen.getByRole("button",{name:"Stop basic-development"}));expect(screen.getByRole("region",{name:"System ROM"})).toBeVisible();});

it("mounts only a fresh explicit development build and keeps normal Play on the selected-ROM route", async () => {
  const mounts: number[][] = [];
  const hashes: string[] = [];
  function Monitor({ onStop, firmware, cassette, cassetteSha256 }: { onStop: () => void; firmware?: "monitor" | "basic-development"; cassette?: Uint8Array; cassetteSha256?: string }) {
    if (cassette) mounts.push(Array.from(cassette));
    if (cassetteSha256) hashes.push(cassetteSha256);
    return <button onClick={onStop}>Stop {firmware}</button>;
  }
  mount(async () => BUILD, [DRAGON], Monitor);
  await begin();
  expect(screen.queryByRole("button", { name: "Mount built cassette in original BASIC (development)" })).not.toBeInTheDocument();
  await check();
  fireEvent.click(screen.getByRole("button", { name: "Computer setup" }));
  fireEvent.click(within(screen.getByRole("region", { name: "System ROM" })).getByRole("button", { name: "Mount built cassette in original BASIC (development)" }));
  expect(screen.getByRole("button", { name: "Stop basic-development" })).toBeVisible();
  expect(mounts[0]).toEqual([1, 2, 3]);
  expect(hashes[0]).toBe(BUILD.result.summary.cassetteSha256);
  expect(screen.queryByRole("button", { name: "Fake emulator ready" })).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Stop basic-development" }));
  fireEvent.click(screen.getByRole("button", { name: "Play" }));
  expect(screen.getByRole("dialog", { name: "Set up your virtual Dragon 32" })).toBeInTheDocument();
  fireEvent.keyDown(document, { key: "Escape" });
  fireEvent.change(screen.getByRole("textbox", { name: "Dragon 32 BASIC source" }), { target: { value: "10 PRINT 99\n" } });
  expect(screen.queryByRole("button", { name: "Mount built cassette in original BASIC (development)" })).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Play" })).toBeDisabled();
});

function debugFactory(normal = false) {
  let loaded = normal; let revision = 0; let state = normal ? "stopped" : "ready"; const release = vi.fn(); const order: string[] = [];
  const factory: DevelopmentDebugger = (source, publish) => {
    expect(source.sourceSha256).toBe("c".repeat(64)); expect(source.breakpointLine).toBe(20);
    const owner = { sessionId: 17, buildId: source.sourceSha256, ...(normal ? { backend: "original" as const } : {}) };
    const frame = () => ({ schemaVersion: 1, sessionId: 17, firmwareId: normal ? "eightbit-dragon32-basic-1" : "eightbit-dragon32-basic-0.1-dev", state: "running", message: "Development", text: " ".repeat(528), width: 256, height: 192, pixels: Array<number>(196608).fill(0), audio: [] });
    const encoded = () => JSON.stringify({ schemaVersion: 1, backend: normal ? "original" : "original-development", revision: { sessionId: owner.sessionId, buildId: owner.buildId, revision: String(revision) }, session: frame(), status: { state, message: "Development", valuesValid: state === "stopped", breakpointVerified: state === "stopped", variables: state === "stopped" ? [{ name: "COUNT", runtimeName: "CO", kind: "number", displayValue: "41", numericValue: 41 }] : [], ...(state === "stopped" ? { stopId: String(revision), registers: { a: 0, b: 0, cc: 0, dp: 0, x: 0, y: 0, u: 0, s: 32700, pc: 34006 }, location: { basicLine: 20, ordinal: 0, sourceStart: 12, sourceEnd: 30, runtimeAddress: 770, tokenLength: 12 } } : {}) } });
    return { sessionId: 17, connection: new DevelopmentHostConnection(owner, { monitor: async command => { if (command.operation === "type") { order.push(command.text); if (command.text === "CLOAD") loaded = true; } revision++; return frame(); }, debug: async request => { if (request) { if (!loaded) throw new Error("Guest not loaded"); order.push(JSON.parse(request).command.operation); state = "stopped"; revision++; } return encoded(); }, release }, publish) };
  };
  return { factory, release, order };
}
it("uses the existing Debug panel only after explicit development selection, then guest load, Debug and Continue", async () => {
  const debug = debugFactory(); const releasePlain = vi.fn();
  function Plain({ onRuntime }: { onRuntime?: (release?: () => void) => void }) { useEffect(() => { onRuntime?.(releasePlain); return () => onRuntime?.(); }, [onRuntime]); return <p>Plain development monitor mounted</p>; }
  const factory = vi.fn((...args: Parameters<DevelopmentDebugger>) => { expect(releasePlain).toHaveBeenCalledTimes(1); return debug.factory(...args); });
  const view = mount(async () => ({ ...BUILD, result: { ...BUILD.result, summary: { ...BUILD.result.summary, sourceSha256: "c".repeat(64) } } }), [DRAGON], Plain, factory);
  await begin(); await check(); expect(screen.queryByRole("tab", { name: "Debug" })).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Computer setup" }));
  fireEvent.click(screen.getAllByRole("button", { name: "Mount built cassette in original BASIC (development)" })[0]!);
  expect(screen.getByRole("tab", { name: "Debug" })).toBeVisible(); expect(factory).not.toHaveBeenCalled(); expect(screen.getByText("Plain development monitor mounted")).toBeVisible();
  fireEvent.click(screen.getByRole("tab", { name: "Debug" }));
  fireEvent.change(screen.getByLabelText("BASIC breakpoint line"), { target: { value: "20" } });
  expect(fireEvent.keyDown(window, { key: "F5", repeat: true })).toBe(false); expect(factory).not.toHaveBeenCalled();
  expect(fireEvent.keyDown(window, { key: "F5" })).toBe(false);
  await screen.findByRole("button", { name: "Load test cassette" }); expect(factory).toHaveBeenCalledTimes(1);
  fireEvent.click(screen.getByRole("button", { name: "Load test cassette" }));
  await waitFor(() => expect(debug.order).toEqual(["CLOAD"]));
  fireEvent.click(screen.getByRole("button", { name: "Explorer" }));
  fireEvent.click(screen.getByRole("button", { name: "dragon.toml" }));
  expect(screen.getByRole("textbox", { name: "Project file dragon.toml" })).toBeVisible();
  fireEvent.click(screen.getByRole("tab", { name: "Debug" }));
  expect(fireEvent.keyDown(window, { key: "F5" })).toBe(false);
  await screen.findByText("COUNT");
  const sourceEditor = screen.getByRole<HTMLTextAreaElement>("textbox", { name: "Dragon 32 BASIC source" });
  expect(sourceEditor).toHaveFocus(); expect([sourceEditor.selectionStart, sourceEditor.selectionEnd]).toEqual([12, 30]); expect(screen.getByText("41")).toBeVisible(); expect(debug.order).toEqual(["CLOAD", "RUN", "continue"]);
  fireEvent.click(screen.getByRole("button", { name: "Continue" })); await waitFor(() => expect(debug.order).toEqual(["CLOAD", "RUN", "continue", "continue"]));
  fireEvent.change(screen.getByRole("textbox", { name: "Dragon 32 BASIC source" }), { target: { value: "10 END" } });
  expect(debug.release).toHaveBeenCalledTimes(1); expect(screen.queryByRole("tab", { name: "Debug" })).not.toBeInTheDocument();
  expect(fireEvent.keyDown(window, { key: "F5" })).toBe(true); expect(factory).toHaveBeenCalledTimes(1);
  view.unmount(); expect(debug.release).toHaveBeenCalledTimes(1);
});
it("keeps ordinary Play on XRoar and clears a development selection when source changes", async () => {
  const factory = vi.fn(debugFactory().factory);
  mount(async () => ({ ...BUILD, result: { ...BUILD.result, summary: { ...BUILD.result.summary, sourceSha256: "c".repeat(64) } } }), [DRAGON], () => null, factory);
  await begin(); await check(); fireEvent.click(screen.getByRole("button", { name: "Play" }));
  expect(screen.getByRole("dialog", { name: "Set up your virtual Dragon 32" })).toBeVisible(); expect(factory).not.toHaveBeenCalled(); closeMenu();
  fireEvent.click(screen.getByRole("button", { name: "Computer setup" })); fireEvent.click(screen.getAllByRole("button", { name: "Mount built cassette in original BASIC (development)" })[0]!);
  fireEvent.change(screen.getByRole("textbox", { name: "Dragon 32 BASIC source" }), { target: { value: "10 END" } });
  expect(screen.queryByRole("tab", { name: "Debug" })).not.toBeInTheDocument(); expect(factory).not.toHaveBeenCalled();
});

it("shows a development setup failure and permits a fresh retry without promoting ordinary Play", async () => {
  const debug = debugFactory(); const factory = vi.fn<DevelopmentDebugger>().mockImplementationOnce(() => { throw new Error("Development worker unavailable"); }).mockImplementation(debug.factory);
  const view = mount(async () => ({ ...BUILD, result: { ...BUILD.result, summary: { ...BUILD.result.summary, sourceSha256: "c".repeat(64) } } }), [DRAGON], () => null, factory);
  await begin(); await check(); fireEvent.click(screen.getByRole("button", { name: "Computer setup" }));
  fireEvent.click(screen.getAllByRole("button", { name: "Mount built cassette in original BASIC (development)" })[0]!); fireEvent.click(screen.getByRole("tab", { name: "Debug" }));
  fireEvent.change(screen.getByLabelText("BASIC breakpoint line"), { target: { value: "20" } });
  fireEvent.click(screen.getAllByRole("button", { name: "Debug" })[0]!);
  await screen.findByText(/Development worker unavailable/);
  expect(screen.getAllByRole("button", { name: "Debug" })[0]).toBeEnabled();
  fireEvent.click(screen.getAllByRole("button", { name: "Debug" })[0]!); await screen.findByRole("button", { name: "Load test cassette" });
  view.unmount(); expect(debug.release).toHaveBeenCalledTimes(1);
});

it("rejects every original-firmware launch control while a development debug route owns the build", async () => {
  const factory = vi.fn<DevelopmentDebugger>().mockImplementation(() => { throw new Error("Development worker unavailable"); });
  mount(async () => ({ ...BUILD, result: { ...BUILD.result, summary: { ...BUILD.result.summary, sourceSha256: "c".repeat(64) } } }), [DRAGON], () => null, factory);
  await begin(); await check(); fireEvent.click(screen.getByRole("button", { name: "Computer setup" }));
  fireEvent.click(screen.getAllByRole("button", { name: "Mount built cassette in original BASIC (development)" })[0]!); fireEvent.click(screen.getByRole("tab", { name: "Debug" }));
  fireEvent.change(screen.getByLabelText("BASIC breakpoint line"), { target: { value: "20" } }); fireEvent.click(screen.getAllByRole("button", { name: "Debug" })[0]!);
  await screen.findByText(/Development worker unavailable/);
  fireEvent.click(screen.getByRole("button", { name: "Play" }));
  const dialog = screen.getByRole("dialog", { name: "Set up your virtual Dragon 32" });
  for (const label of ["Start original monitor", "Start original BASIC (development)"]) {
    const control = within(dialog).getByRole("button", { name: label }); expect(control).toBeDisabled(); fireEvent.click(control);
  }
  expect(dialog).toBeVisible(); expect(factory).toHaveBeenCalledTimes(1);
  closeMenu(); fireEvent.click(screen.getByRole("tab", { name: "Game" }));
  for (const label of ["Start original monitor", "Start original BASIC (development)"]) for (const control of screen.getAllByRole("button", { name: label })) expect(control).toBeDisabled();
});

const qualifiedProfile = { schemaVersion: 1 as const, backend: "original" as const, target: "dragon32" as const, profileId: "eightbit-dragon32-basic-1" as const, firmwareSha256: "a".repeat(64), coreVersion: "0.5.2" };
it.each(["eightbit-dragon32-basic-1", "eightbit-dragon32-basic-extended-1"] as const)("defaults Dragon 32 to %s Run without OEM firmware and retains an explicit XRoar choice", async profileId => {
  const selectedProfile = { ...qualifiedProfile, profileId };
  const dispose = vi.fn(), transport = vi.fn(async () => ({}));
  const factory = vi.fn<NonNullable<Parameters<typeof BrowserWorkbench>[0]["OriginalRuntime"]>>(source => ({ owner: { sessionId: 51, buildId: "kind" in source ? "a".repeat(64) : source.sourceSha256, backend: "original" }, transport, dispose }));
  const view = mount(async () => ({ ...BUILD, result: { ...BUILD.result, summary: { ...BUILD.result.summary, sourceSha256: "c".repeat(64) } } }), [DRAGON], () => <p>Development monitor</p>, undefined, { originalProfile: selectedProfile, OriginalRuntime: factory });
  await begin(); await check(); fireEvent.click(screen.getByRole("button", { name: "Play" }));
  await screen.findByRole("button", { name: "Load test cassette" });
  expect(screen.queryByRole("dialog", { name: "Set up your virtual Dragon 32" })).not.toBeInTheDocument();
  expect(factory).toHaveBeenCalledOnce(); expect(factory.mock.calls[0]![0]).toMatchObject({ source: SAMPLE_SOURCE, sourceSha256: "c".repeat(64), cassetteSha256: "b".repeat(64) });
  expect(factory.mock.calls[0]![0]).not.toHaveProperty("breakpointLine");
  expect(factory.mock.calls[0]![3]).toEqual(selectedProfile);
  expect(screen.getByRole("button", { name: "Check & build" })).toBeDisabled(); expect(screen.getByRole("button", { name: "Play" })).toBeDisabled();
  fireEvent.click(screen.getByRole("button", { name: "Computer setup" }));
  expect(screen.getByRole("button", { name: "Start original monitor" })).toBeDisabled(); expect(screen.getByRole("button", { name: "Start original BASIC (development)" })).toBeDisabled();
  expect(screen.getByText(/uses its bundled firmware/)).toBeVisible();
  expect(screen.getByRole("combobox", { name: "Project emulator" })).toHaveValue("original");
  expect(screen.getByRole("option", { name: "XRoar" })).toBeInTheDocument();
  expect(screen.queryByText(/Add your own to play/)).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Stop test console" })); expect(dispose).toHaveBeenCalledOnce();
  for (const panel of ["Results", "Game", "Debug"]) {
    fireEvent.click(screen.getByRole("tab", { name: panel }));
    expect(screen.getByText(/Built-in Dragon 32 session ended\. Play or Debug starts a fresh session\./)).toBeVisible();
    expect(screen.queryByText(/Original BASIC is using the current build/)).not.toBeInTheDocument();
  }
  fireEvent.click(screen.getByRole("button", { name: "Play" })); await waitFor(() => expect(factory).toHaveBeenCalledTimes(2));
  fireEvent.change(screen.getByRole("textbox", { name: "Dragon 32 BASIC source" }), { target: { value: "10 END" } }); expect(dispose).toHaveBeenCalledTimes(2);
  expect(screen.getByRole("button", { name: "Play" })).toBeDisabled(); view.unmount(); expect(dispose).toHaveBeenCalledTimes(2);
});
it("retries rejected original admission and discards a pending launch after a source change", async () => {
  let finish: (value: unknown) => void = () => {}; const dispose = vi.fn();
  const factory = vi.fn<NonNullable<Parameters<typeof BrowserWorkbench>[0]["OriginalRuntime"]>>().mockImplementationOnce(() => { throw new Error("Original BASIC is not qualified"); }).mockImplementation(source => ({ owner: { sessionId: 51, buildId: "kind" in source ? "a".repeat(64) : source.sourceSha256, backend: "original" }, transport: () => new Promise(resolve => { finish = resolve; }), dispose }));
  const view = mount(async () => ({ ...BUILD, result: { ...BUILD.result, summary: { ...BUILD.result.summary, sourceSha256: "c".repeat(64) } } }), [DRAGON], undefined, undefined, { originalProfile: qualifiedProfile, OriginalRuntime: factory });
  await begin(); await check(); fireEvent.click(screen.getByRole("button", { name: "Computer setup" }));
  fireEvent.change(screen.getByRole("combobox", { name: "Project emulator" }), { target: { value: "original" } }); fireEvent.click(screen.getByRole("button", { name: "Play" }));
  await screen.findByText("Original BASIC is not qualified"); fireEvent.click(screen.getByRole("button", { name: "Play" }));
  await waitFor(() => expect(factory).toHaveBeenCalledTimes(2));
  expect(screen.getByRole("button", { name: "Stop" })).toBeEnabled();
  fireEvent.change(screen.getByRole("textbox", { name: "Dragon 32 BASIC source" }), { target: { value: "10 END" } });
  await act(async () => { finish({}); }); expect(dispose).toHaveBeenCalled(); expect(screen.queryByRole("button", { name: "Load test cassette" })).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Play" })).toBeDisabled(); view.unmount();
});

it("uses the same breakpoint/Continue presentation for an explicitly qualified original Debug owner", async () => {
  const debug = debugFactory(true);
  const factory: NonNullable<Parameters<typeof BrowserWorkbench>[0]["OriginalRuntime"]> = (source, publish, released) => {
    if ("kind" in source) throw new Error("Expected BASIC debug source");
    const owned = debug.factory({ ...source, breakpointLine: source.breakpointLine! }, publish, released);
    return { owner: { sessionId: owned.sessionId, buildId: source.sourceSha256, backend: "original" }, connection: owned.connection, transport: owned.connection.monitor, dispose: () => owned.connection.dispose() };
  };
  const view = mount(async () => ({ ...BUILD, result: { ...BUILD.result, summary: { ...BUILD.result.summary, sourceSha256: "c".repeat(64) } } }), [DRAGON], undefined, undefined, { originalProfile: qualifiedProfile, OriginalRuntime: factory });
  await begin(); await check(); fireEvent.click(screen.getByRole("button", { name: "Computer setup" }));
  fireEvent.change(screen.getByRole("combobox", { name: "Project emulator" }), { target: { value: "original" } });
  fireEvent.click(screen.getByRole("tab", { name: "Debug" })); fireEvent.change(screen.getByLabelText("BASIC breakpoint line"), { target: { value: "20" } });
  fireEvent.keyDown(window, { key: "F5" }); await screen.findByText("COUNT"); expect(screen.getByText("41")).toBeVisible();
  expect(screen.getByText(/^Original BASIC is using the current build\./)).toBeVisible();
  expect(screen.getByRole("textbox", { name: "Dragon 32 BASIC source" })).toHaveFocus();
  fireEvent.click(screen.getByRole("button", { name: "Continue" })); await waitFor(() => expect(debug.order).toEqual(["continue"]));
  fireEvent.click(screen.getAllByRole("button", { name: "Stop" })[0]!); expect(debug.release).toHaveBeenCalledOnce(); view.unmount(); expect(debug.release).toHaveBeenCalledOnce();
});

it("cancels a pending normal worker and ignores its late rejection after a fresh retry", async () => {
  let rejectOld: (error: Error) => void = () => {}; const disposeOld = vi.fn(), disposeNew = vi.fn();
  const factory = vi.fn<NonNullable<Parameters<typeof BrowserWorkbench>[0]["OriginalRuntime"]>>().mockImplementationOnce(source => ({ owner: { sessionId: 51, buildId: "kind" in source ? "a".repeat(64) : source.sourceSha256, backend: "original" }, transport: () => new Promise((_resolve, reject) => { rejectOld = reject; }), dispose: disposeOld })).mockImplementation(source => ({ owner: { sessionId: 52, buildId: "kind" in source ? "a".repeat(64) : source.sourceSha256, backend: "original" }, transport: async () => ({}), dispose: disposeNew }));
  const view = mount(async () => ({ ...BUILD, result: { ...BUILD.result, summary: { ...BUILD.result.summary, sourceSha256: "c".repeat(64) } } }), [DRAGON], undefined, undefined, { originalProfile: qualifiedProfile, OriginalRuntime: factory });
  await begin(); await check(); fireEvent.click(screen.getByRole("button", { name: "Computer setup" }));
  fireEvent.change(screen.getByRole("combobox", { name: "Project emulator" }), { target: { value: "original" } }); fireEvent.click(screen.getByRole("button", { name: "Play" }));
  await waitFor(() => expect(factory).toHaveBeenCalledOnce()); expect(screen.queryByRole("button", { name: "Load test cassette" })).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Stop" })); expect(disposeOld).toHaveBeenCalledOnce();
  expect(screen.getByText("Built-in Dragon 32 session ended. Play or Debug starts a fresh session.")).toBeVisible();
  fireEvent.click(screen.getByRole("button", { name: "Play" })); await screen.findByRole("button", { name: "Load test cassette" });
  await act(async () => { rejectOld(new Error("Old worker failed")); }); expect(screen.getByRole("button", { name: "Load test cassette" })).toBeVisible();
  expect(disposeNew).not.toHaveBeenCalled(); expect(screen.queryByText("Old worker failed")).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("tab", { name: "Results" })); expect(screen.getByText("Original BASIC is using the current build.")).toBeVisible(); view.unmount(); expect(disposeNew).toHaveBeenCalledOnce();
});

it("runs the retained machine-code import and invalidates it on edits or failed builds", async () => {
  const identity = { schemaVersion: 1, input: { format: "raw", loadAddress: 28672, executionAddress: 28672 }, inputSha256: "d".repeat(64), cassetteSha256: "e".repeat(64) };
  const bytes = new Uint8Array([0x86, 65, 0x39]);
  const originalFile = new File([bytes], "own.bin"); Object.defineProperty(originalFile, "arrayBuffer", { value: async () => bytes.buffer });
  const factory = vi.fn<NonNullable<Parameters<typeof BrowserWorkbench>[0]["OriginalRuntime"]>>(() => ({ owner: { sessionId: 78, buildId: "e".repeat(64), backend: "original" }, transport: async () => ({}), dispose: vi.fn() }));
  const request = vi.fn(async (request: BrowserRequest) => request.operation === "decompile" ? { schemaVersion: 1, ok: true, result: { kind: "assembly", source: "START\n lda #65\n rts\n", warnings: [], machineCode: identity, preview: { loadAddress: 28672, executionAddress: 28672 } } } : { schemaVersion: 1, ok: false, message: "Assembly rejected", diagnostics: [{ row: 2, column: 1, code: "DSW1024", message: "Unknown instruction" }] });
  mount(request, [DRAGON], undefined, undefined, { originalProfile: qualifiedProfile, OriginalRuntime: factory });
  fireEvent.click(screen.getByText("Advanced import"));
  fireEvent.change(screen.getByLabelText("Choose raw binary…"), { target: { files: [originalFile] } });
  await screen.findByRole("textbox", { name: "Imported source preview" });
  expect(factory).not.toHaveBeenCalled();
  expect(request.mock.calls[0]?.[0]).toMatchObject({ input: identity.input });
  fireEvent.click(screen.getByRole("button", { name: "Edit this copy" }));
  await screen.findByRole("textbox", { name: "6809 assembly source" });
  expect(screen.getByRole("button", { name: "Play" })).toBeEnabled();
  bytes.fill(0);
  fireEvent.click(screen.getByRole("button", { name: "Play" }));
  await waitFor(() => expect(factory).toHaveBeenCalledOnce());
  expect(factory.mock.calls[0]?.[0]).toMatchObject({ kind: "machine-code", identity, bytes: new Uint8Array([0x86, 65, 0x39]) });
  fireEvent.click(screen.getByRole("button", { name: "Stop test console" }));
  fireEvent.change(screen.getByRole("textbox", { name: "6809 assembly source" }), { target: { value: "START\n invalid\n" } });
  expect(screen.getByRole("button", { name: "Play" })).toBeDisabled();
  fireEvent.click(screen.getByRole("button", { name: "Check & build" }));
  await screen.findByRole("alert");
  expect(screen.getByRole("button", { name: "Play" })).toBeDisabled();
  expect(factory).toHaveBeenCalledOnce();
});

it("creates, builds and runs assembly with the bundled runtime and rejects stale output", async () => {
  const dispose = vi.fn();
  const factory = vi.fn<NonNullable<Parameters<typeof BrowserWorkbench>[0]["OriginalRuntime"]>>(() => ({ owner: { sessionId: 79, buildId: "f".repeat(64), backend: "original" }, transport: async () => ({}), dispose }));
  const assembly = { ...BUILD, result: { ...BUILD.result, summary: { ...BUILD.result.summary, sourceSha256: "a".repeat(64) }, assembly: { buildId: "f".repeat(64), loadAddress: 28672, entryAddress: 28672 } } };
  mount(async () => assembly, [DRAGON], undefined, undefined, { originalProfile: qualifiedProfile, OriginalRuntime: factory });
  fireEvent.click(screen.getByRole("button", { name: "Create new project" }));
  fireEvent.change(screen.getByLabelText("Start with"), { target: { value: "assembly" } });
  expect(within(screen.getByRole("dialog", { name: "Create new project" })).getByLabelText("Load address (decimal)")).toHaveValue(28672);
  expect(screen.getByLabelText("Entry symbol")).toHaveValue("START");
  fireEvent.click(screen.getByRole("button", { name: "Create project" }));
  await screen.findByRole("textbox", { name: "6809 assembly source" });
  expect(screen.getByRole("button", { name: "Play" })).toBeDisabled();
  await check(); fireEvent.click(screen.getByRole("button", { name: "Play" }));
  await waitFor(() => expect(factory).toHaveBeenCalledOnce());
  expect(factory.mock.calls[0]?.[0]).toMatchObject({ kind: "assembly-source", buildId: "f".repeat(64), project: { kind: "assembly", source: "START\n        lda #65\n        sta $0400\n        rts\n" } });
  expect(screen.queryByRole("dialog", { name: "Set up your virtual Dragon 32" })).not.toBeInTheDocument();
  fireEvent.change(screen.getByRole("textbox", { name: "6809 assembly source" }), { target: { value: "START\n rts\n" } });
  expect(dispose).toHaveBeenCalledOnce();
  expect(screen.getByRole("button", { name: "Play" })).toBeDisabled();
});

it("requires explicit cartridge interpretation and displays its unsupported result without launching", async () => {
  const request = vi.fn(async (_request: BrowserRequest) => ({ schemaVersion: 1, ok: false, message: "DSW1023: cartridges are unsupported on the website; use the desktop edition with XRoar" }));
  mount(request);
  fireEvent.click(screen.getByText("Advanced import"));
  fireEvent.change(screen.getByLabelText("Machine-code file format"), { target: { value: "cartridge" } });
  fireEvent.change(screen.getByLabelText("Choose raw binary…"), { target: { files: [file(2048, "own.rom")] } });
  await screen.findByText(/DSW1023: cartridges are unsupported on the website/);
  expect(request.mock.calls[0]?.[0]).toMatchObject({ input: { format: "cartridge" } });
  expect(screen.queryByRole("button", { name: "Edit this copy" })).not.toBeInTheDocument();
});


it("debugs freshly built assembly through the shared machine panel and invalidates its source map on edits", async () => {
  const release = vi.fn(), operations: string[] = [];
  let revision = 0, state = "stopped", pc = 28672;
  const factory = vi.fn<NonNullable<Parameters<typeof BrowserWorkbench>[0]["OriginalRuntime"]>>((source, publish) => {
    expect(source).toMatchObject({ kind: "assembly-source", debug: { kind: "source", sourceLine: 2 } });
    const owner = { sessionId: 88, buildId: "f".repeat(64), backend: "original" as const };
    const frame = () => ({ schemaVersion: 1, sessionId: 88, firmwareId: "eightbit-dragon32-basic-extended-1", state: "running", message: "Machine", text: " ".repeat(528), width: 256, height: 192, pixels: Array<number>(196608).fill(0), audio: [] });
    const encoded = () => JSON.stringify({ schemaVersion: 1, backend: "original", kind: "machine", revision: { ...owner, backend: undefined, revision: String(revision) }, session: frame(), status: { state, message: state === "stopped" ? "Machine breakpoint" : "Machine running", stopId: state === "stopped" ? String(revision) : null, breakpointAddress: 28672, breakpointVerified: state === "stopped", valuesValid: state === "stopped", disassembly: state === "stopped" ? [{ address: pc, bytes: [0x86, 65], mnemonic: "LDA", operand: "#65" }] : [], ...(state === "stopped" ? { registers: { a: 65, b: 0, cc: 0, dp: 0, x: 0, y: 0, u: 0, s: 32700, pc }, memory: { startAddress: pc, bytes: [0x86, 65] }, location: { sourceUri: "src/recovered.asm", sourceLine: pc === 28672 ? 2 : 3, runtimeAddress: pc, byteLength: 2 } } : {}) } });
    const connection = new OriginalHostConnection(owner, { monitor: async () => frame(), debug: async request => { if (request) { const operation = JSON.parse(request).command.operation; operations.push(operation); revision++; if (operation === "step") pc += 2; state = operation === "continue" ? "running" : "stopped"; } return encoded(); }, release }, publish);
    return { owner, connection, transport: connection.monitor, dispose: () => connection.dispose() };
  });
  const assembly = { ...BUILD, result: { ...BUILD.result, summary: { ...BUILD.result.summary, sourceSha256: "a".repeat(64) }, assembly: { buildId: "f".repeat(64), loadAddress: 28672, entryAddress: 28672 } } };
  mount(async () => assembly, [DRAGON], undefined, undefined, { originalProfile: { ...qualifiedProfile, profileId: "eightbit-dragon32-basic-extended-1" }, OriginalRuntime: factory });
  fireEvent.click(screen.getByRole("button", { name: "Create new project" }));
  fireEvent.change(screen.getByLabelText("Start with"), { target: { value: "assembly" } });
  fireEvent.click(screen.getByRole("button", { name: "Create project" }));
  await screen.findByRole("textbox", { name: "6809 assembly source" }); await check();
  fireEvent.click(screen.getByRole("tab", { name: "Debug" }));
  expect(screen.getByLabelText("6809 address breakpoint")).toHaveValue("$7000");
  expect(screen.queryByRole("button", { name: "Assembler Setup…" })).not.toBeInTheDocument();
  fireEvent.change(screen.getByLabelText("Assembly breakpoint line"), { target: { value: "2" } });
  fireEvent.keyDown(window, { key: "F5" });
  await waitFor(() => expect(factory).toHaveBeenCalledOnce());
  await waitFor(() => expect(document.body.textContent).toContain("Machine breakpoint"));
  expect(screen.getByRole("region", { name: "CPU Registers" })).toHaveTextContent("$7000");
  expect(screen.getByRole("textbox", { name: "6809 assembly source" })).toHaveFocus();
  fireEvent.click(screen.getByRole("button", { name: "Step" })); await waitFor(() => expect(operations).toEqual(["step"]));
  await waitFor(() => expect(screen.getByRole("region", { name: "CPU Registers" })).toHaveTextContent("$7002"));
  fireEvent.click(screen.getByRole("button", { name: "Continue" })); await screen.findByText("Machine running");
  expect(screen.getByRole("button", { name: "Step" })).toBeDisabled();
  fireEvent.click(screen.getByRole("button", { name: "Pause" })); await screen.findByText("Machine breakpoint");
  expect(operations).toEqual(["step", "continue", "pause"]);
  fireEvent.change(screen.getByRole("textbox", { name: "6809 assembly source" }), { target: { value: "START\n rts\n" } });
  expect(release).toHaveBeenCalledOnce();
  expect(screen.getByRole("button", { name: "Play" })).toBeDisabled();
  expect(screen.queryByRole("tab", { name: "Debug" })).not.toBeInTheDocument();
});


it.each(["Project home", "Close src/recovered.asm"])("releases imported executable ownership through %s and never runs it from the welcome screen", async closeLabel => {
  vi.spyOn(window, "confirm").mockReturnValue(true);
  const identity = { schemaVersion: 1, input: { format: "raw", loadAddress: 28672, executionAddress: 28672 }, inputSha256: "d".repeat(64), cassetteSha256: "e".repeat(64) };
  const factory = vi.fn<NonNullable<Parameters<typeof BrowserWorkbench>[0]["OriginalRuntime"]>>(() => ({ owner: { sessionId: 78, buildId: "e".repeat(64), backend: "original" }, transport: async () => ({}), dispose: vi.fn() }));
  mount(async () => ({ schemaVersion: 1, ok: true, result: { kind: "assembly", source: "START\n rts\n", warnings: [], machineCode: identity, preview: { loadAddress: 28672, executionAddress: 28672 } } }), [DRAGON], undefined, undefined, { originalProfile: qualifiedProfile, OriginalRuntime: factory });
  fireEvent.click(screen.getByText("Advanced import"));
  fireEvent.change(screen.getByLabelText("Choose raw binary…"), { target: { files: [file(1, "own.bin")] } });
  await screen.findByRole("textbox", { name: "Imported source preview" });
  fireEvent.click(screen.getByRole("button", { name: "Edit this copy" }));
  await screen.findByRole("textbox", { name: "6809 assembly source" });
  expect(screen.getByRole("button", { name: "Play" })).toBeEnabled();
  expect(screen.getByRole("tab", { name: "Debug" })).toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: closeLabel }));
  expect(screen.getByRole("heading", { name: "Get Started" })).toBeVisible();
  expect(screen.getByRole("button", { name: "Play" })).toBeDisabled();
  expect(screen.queryByRole("tab", { name: "Debug" })).not.toBeInTheDocument();
  fireEvent.keyDown(window, { key: "r", ctrlKey: true }); fireEvent.keyDown(window, { key: "F5" });
  expect(factory).not.toHaveBeenCalled();
});

it("opens each diagnostic's declared assembly source before selecting its location", async () => {
  const snapshot = { schemaVersion: 2, target: "dragon32", name: "Multi", kind: "assembly", source: "START nop\n bad_main\n", manifest: "native manifest retained", supportFiles: { "asm/other.asm": "OTHER nop\n rts\n bad_other\n", "src/MAIN.BAS": "10 REM desktop support\n" } };
  const request = vi.fn<BrowserEngine["request"]>(async input => {
    if (input.operation === "import-project") return { schemaVersion: 1, ok: true, result: { project: snapshot, entryPath: "asm/main.asm", sourcePaths: ["asm/main.asm", "asm/other.asm"] } };
    return { schemaVersion: 1, ok: false, message: "Correct assembly source", diagnostics: [
      { sourceUri: "asm/other.asm", row: 3, column: 2, code: "DSW1024", message: "Unknown secondary instruction" },
      { sourceUri: "asm/main.asm", row: 2, column: 3, code: "DSW1024", message: "Unknown primary instruction" },
      { sourceUri: "unlisted.asm", row: 1, column: 1, code: "DSW1024", message: "Unlisted source" },
    ] };
  });
  render(<BrowserWorkbench engine={{ request }} machines={[DRAGON]} Emulator={Emulator} assetBase="/studio/" />);
  fireEvent.click(screen.getByRole("button", { name: "Load existing project…" }));
  fireEvent.change(screen.getByLabelText("Choose project file…"), { target: { files: [file(0, "project.zip")] } });
  await screen.findByRole("textbox", { name: "6809 assembly source" });
  fireEvent.click(screen.getByRole("button", { name: "asm/other.asm" }));
  fireEvent.click(screen.getByRole("button", { name: "Check & build" }));
  await screen.findByRole("alert");
  expect(screen.getByRole("button", { name: "Play" })).toBeDisabled();
  fireEvent.click(screen.getByRole("tab", { name: "main.asm" }));
  fireEvent.click(screen.getByRole("button", { name: "Line 3, column 2: Unknown secondary instruction" }));
  const secondary = screen.getByRole("textbox", { name: "6809 assembly source" }) as HTMLTextAreaElement;
  expect(secondary).toHaveValue(snapshot.supportFiles["asm/other.asm"]);
  expect(secondary).toHaveFocus(); expect(secondary.selectionStart).toBe(16);
  fireEvent.click(screen.getByRole("button", { name: "dragon.toml" }));
  expect(screen.getByRole("textbox", { name: "Project file dragon.toml" })).toHaveAttribute("readonly");
  fireEvent.click(screen.getByRole("button", { name: "Line 2, column 3: Unknown primary instruction" }));
  const primary = screen.getByRole("textbox", { name: "6809 assembly source" }) as HTMLTextAreaElement;
  expect(primary).toHaveValue(snapshot.source);
  expect(primary).toHaveFocus(); expect(primary.selectionStart).toBe(12);
  fireEvent.click(screen.getByRole("button", { name: "Line 1, column 1: Unlisted source" }));
  expect(primary).toHaveValue(snapshot.source); expect(primary.selectionStart).toBe(12);
  expect(screen.getByRole("button", { name: "Play" })).toBeDisabled();
});

it("edits every declared assembly source, binds its breakpoint URI and revokes stale multi-file builds", async () => {
  vi.spyOn(window, "confirm").mockReturnValue(true);
  const snapshot = { schemaVersion: 2, target: "dragon32", name: "Multi", kind: "assembly", source: "START jsr OTHER\n rts\n", manifest: "native manifest retained", supportFiles: { "asm/other.asm": "OTHER nop\n rts\n", "src/MAIN.BAS": "10 REM desktop support\n", "AGENTS.md": "User guidance" } };
  const assembly = { ...BUILD, result: { ...BUILD.result, summary: { ...BUILD.result.summary, sourceSha256: "a".repeat(64) }, assembly: { buildId: "f".repeat(64), loadAddress: 8192, entryAddress: 8192 } } };
  let delayed: ((value: unknown) => void) | undefined, delay = false;
  const request = vi.fn<BrowserEngine["request"]>(async input => {
    if (input.operation === "import-project") return { schemaVersion: 1, ok: true, result: { project: snapshot, entryPath: "asm/main.asm", sourcePaths: ["asm/main.asm", "asm/other.asm"] } };
    if (input.operation === "test-project") return delay ? new Promise(resolve => { delayed = resolve; }) : assembly;
    if (input.operation === "export-project") return { schemaVersion: 1, ok: true, result: { archive: [80, 75] } };
    throw new Error("Unexpected request");
  });
  const release = vi.fn();
  const factory = vi.fn<NonNullable<Parameters<typeof BrowserWorkbench>[0]["OriginalRuntime"]>>((_input, publish) => {
    const owner = { sessionId: 99, buildId: "f".repeat(64), backend: "original" as const };
    const frame = () => ({ schemaVersion: 1, sessionId: 99, firmwareId: "eightbit-dragon32-basic-extended-1", state: "running", message: "Machine", text: " ".repeat(528), width: 256, height: 192, pixels: Array<number>(196608).fill(0), audio: [] });
    const inspection = () => JSON.stringify({ schemaVersion: 1, backend: "original", kind: "machine", revision: { sessionId: 99, buildId: owner.buildId, revision: "0" }, session: frame(), status: { state: "stopped", message: "Secondary breakpoint", stopId: "0", breakpointAddress: 8196, breakpointVerified: true, valuesValid: true, disassembly: [{ address: 8196, bytes: [0x12], mnemonic: "NOP", operand: "" }], registers: { a: 0, b: 0, cc: 0, dp: 0, x: 0, y: 0, u: 0, s: 32700, pc: 8196 }, memory: { startAddress: 8196, bytes: [0x12] }, location: { sourceUri: "asm/other.asm", sourceLine: 1, runtimeAddress: 8196, byteLength: 1 } } });
    const connection = new OriginalHostConnection(owner, { monitor: async () => frame(), debug: async () => inspection(), release }, publish);
    return { owner, connection, transport: connection.monitor, dispose: () => connection.dispose() };
  });
  render(<BrowserWorkbench engine={{ request }} machines={[DRAGON]} Emulator={Emulator} originalProfile={{ ...qualifiedProfile, profileId: "eightbit-dragon32-basic-extended-1" }} OriginalRuntime={factory} assetBase="/studio/" />);
  fireEvent.click(screen.getByRole("button", { name: "Load existing project…" }));
  fireEvent.change(screen.getByLabelText("Choose project file…"), { target: { files: [file(0, "project.zip")] } });
  await screen.findByRole("textbox", { name: "6809 assembly source" });
  expect(factory).not.toHaveBeenCalled(); expect(request.mock.calls.filter(([input]) => input.operation === "test-project")).toHaveLength(0);
  fireEvent.click(screen.getByRole("button", { name: "asm/other.asm" }));
  expect(screen.getByRole("textbox", { name: "6809 assembly source" })).toHaveValue(snapshot.supportFiles["asm/other.asm"]);
  const changed = "OTHER nop\n nop\n rts\n";
  fireEvent.change(screen.getByRole("textbox", { name: "6809 assembly source" }), { target: { value: changed } });
  await check();
  expect(request).toHaveBeenCalledWith(expect.objectContaining({ operation: "test-project", project: expect.objectContaining({ source: snapshot.source, manifest: snapshot.manifest, supportFiles: { ...snapshot.supportFiles, "asm/other.asm": changed } }) }), new Uint8Array(), expect.any(AbortSignal));
  fireEvent.click(screen.getByRole("tab", { name: "Debug" }));
  fireEvent.change(screen.getByLabelText("Assembly breakpoint line"), { target: { value: "1" } });
  fireEvent.keyDown(window, { key: "F5" });
  await waitFor(() => expect(factory).toHaveBeenCalledOnce());
  expect(factory.mock.calls[0]?.[0]).toMatchObject({ kind: "assembly-source", debug: { kind: "source", sourceUri: "asm/other.asm", sourceLine: 1 } });
  await screen.findByText("Secondary breakpoint");
  expect(screen.getByRole("textbox", { name: "6809 assembly source" })).toHaveValue(changed);
  expect(screen.getByRole("textbox", { name: "6809 assembly source" })).toHaveFocus();
  fireEvent.change(screen.getByRole("textbox", { name: "6809 assembly source" }), { target: { value: "OTHER rts\n" } });
  expect(release).toHaveBeenCalledOnce(); expect(screen.getByRole("button", { name: "Play" })).toBeDisabled();
  await waitFor(() => expect(recentProjects()[0]?.project.supportFiles["asm/other.asm"]).toBe("OTHER rts\n"));
  fireEvent.click(screen.getByRole("button", { name: "src/MAIN.BAS" }));
  expect(screen.getByRole("textbox", { name: "Project file src/MAIN.BAS" })).toHaveAttribute("readonly");
  fireEvent.click(screen.getByRole("button", { name: "asm/other.asm" }));
  delay = true; fireEvent.click(screen.getByRole("button", { name: "Check & build" }));
  await waitFor(() => expect(delayed).toBeDefined());
  fireEvent.change(screen.getByRole("textbox", { name: "6809 assembly source" }), { target: { value: "OTHER nop\n rts\n" } });
  await act(async () => delayed?.(assembly));
  expect(screen.getByRole("button", { name: "Play" })).toBeDisabled();
  fireEvent.click(screen.getByRole("button", { name: "Project home" }));
  fireEvent.keyDown(window, { key: "F5" });
  expect(factory).toHaveBeenCalledOnce();
});

it.each(["dragon64", "dragon64-native"] as const)("runs and revokes a source-bound %s project without historical firmware setup", async target => {
  const machine: BrowserMachine = { ...DRAGON, id: target, label: target, defaultRuntime: "original" };
  const profile = { ...qualifiedProfile, target, profileId: target === "dragon64" ? "eightbit-dragon64-basic-32k-dev-1" as const : "eightbit-dragon64-basic-64k-dev-1" as const };
  const dispose = vi.fn();
  const factory = vi.fn<NonNullable<Parameters<typeof BrowserWorkbench>[0]["OriginalRuntime"]>>(source => ({ owner: { sessionId: 64, buildId: "kind" in source ? "a".repeat(64) : source.sourceSha256, backend: "original" }, transport: vi.fn(async () => ({})), dispose }));
  const view = mount(async () => ({ ...BUILD, result: { ...BUILD.result, summary: { ...BUILD.result.summary, sourceSha256: "c".repeat(64) } } }), [DRAGON, machine], undefined, undefined, { originalProfiles: { [target]: profile }, OriginalRuntime: factory });
  fireEvent.click(screen.getByRole("button", { name: "Create new project" }));
  fireEvent.change(screen.getByLabelText("Computer"), { target: { value: target } });
  fireEvent.click(screen.getByRole("button", { name: "Create project" }));
  const editor = await screen.findByRole("textbox", { name: `${target} BASIC source` });
  await check(); fireEvent.click(screen.getByRole("button", { name: "Play" }));
  await waitFor(() => expect(factory).toHaveBeenCalledOnce());
  expect(factory.mock.calls[0]![3]).toEqual(profile);
  expect(screen.queryByRole("dialog", { name: /Set up your virtual/ })).not.toBeInTheDocument();
  fireEvent.change(editor, { target: { value: "10 END\n" } });
  expect(dispose).toHaveBeenCalledOnce();
  expect(screen.getByRole("button", { name: "Play" })).toBeDisabled();
  view.unmount();
});

it("keeps chips as the C64 default and binds an explicitly selected original run to the current build",async()=>{
 const launched=vi.fn();
 const C64Monitor:NonNullable<Parameters<typeof BrowserWorkbench>[0]["C64Monitor"]>=({source,onStop})=>{launched(source);return <button onClick={onStop}>Stop original C64 test</button>;};
 mount(async()=>({...BUILD,result:{...BUILD.result,summary:{...BUILD.result.summary,sourceSha256:"a".repeat(64)}}}),[C64],undefined,undefined,{C64Monitor});
 fireEvent.click(screen.getByRole("button",{name:"Create new project"}));
 fireEvent.change(screen.getByLabelText("Computer"),{target:{value:"c64-pal"}});
 fireEvent.click(screen.getByRole("button",{name:"Create project"}));
 const editor=await screen.findByRole("textbox",{name:`${C64.label} BASIC source`});
 await check();
 fireEvent.click(screen.getByRole("button",{name:"Play"}));
 expect(screen.getByRole("dialog",{name:"Set up your virtual Commodore 64 (PAL)"})).toBeVisible();
 closeMenu();fireEvent.click(screen.getByRole("button",{name:"Computer setup"}));
 const setup=screen.getByRole("region",{name:"System ROM"});
 expect(within(setup).getByLabelText("Project emulator")).toHaveValue("chips-c64");
 fireEvent.change(within(setup).getByLabelText("Project emulator"),{target:{value:"original"}});
 fireEvent.click(screen.getByRole("button",{name:"Play"}));
 await screen.findByRole("button",{name:"Stop original C64 test"});
 expect(launched).toHaveBeenCalledWith({schemaVersion:1,target:"c64-pal",source:C64.sample,sourceSha256:"a".repeat(64),prgSha256:"b".repeat(64)});
 fireEvent.click(screen.getByRole("button",{name:"Stop original C64 test"}));
 fireEvent.change(editor,{target:{value:"10 END\n"}});
 expect(screen.getByRole("button",{name:"Play"})).toBeDisabled();
});

it("builds and plays Vectrex assembly from its cartridge receipt without firmware setup", async () => {
  const launched = vi.fn();
  const machine: BrowserMachine = { id: "vectrex", label: "Vectrex", cpu: "6809", extension: "bin", manifest: "vectrex.toml", runtime: "original-vectrex", romSize: 8192, romNames: ["executive.rom"], sample: " org 0\n", assemblySample: " org 0\n" };
  const VectrexMonitor: NonNullable<Parameters<typeof BrowserWorkbench>[0]["VectrexMonitor"]> = ({ source, onStop }) => { launched(source); return <button onClick={onStop}>Stop original Vectrex test</button>; };
  mount(async () => ({ ...BUILD, result: { ...BUILD.result, summary: { ...BUILD.result.summary, sourceSha256: "a".repeat(64) }, roundTripVerified: false, cartridgeVerified: true, assembly: { buildId: "b".repeat(64), loadAddress: 0, entryAddress: 0 } } }), [DRAGON, machine], undefined, undefined, { VectrexMonitor });
  fireEvent.click(screen.getByRole("button", { name: "Create new project" }));
  fireEvent.change(screen.getByLabelText("Computer"), { target: { value: "vectrex" } });
  fireEvent.click(screen.getByRole("button", { name: "Create project" }));
  const editor = await screen.findByRole("textbox", { name: "6809 assembly source" });
  fireEvent.click(screen.getByRole("button", { name: "Check & build" }));
  await screen.findByText(/Cartridge assembled and header validated/);
  fireEvent.click(screen.getByRole("button", { name: "Play" }));
  await screen.findByRole("button", { name: "Stop original Vectrex test" });
  expect(launched).toHaveBeenCalledWith({ schemaVersion: 1, target: "vectrex", source: machine.sample, sourceSha256: "a".repeat(64), cartridgeSha256: "b".repeat(64) });
  expect(screen.queryByRole("dialog", { name: /Set up your virtual/ })).not.toBeInTheDocument();
  fireEvent.click(screen.getByRole("button", { name: "Stop original Vectrex test" }));
  fireEvent.change(editor, { target: { value: " org 0\n nop\n" } });
  expect(screen.getByRole("button", { name: "Play" })).toBeDisabled();
});
