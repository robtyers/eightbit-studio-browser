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
function mount(handler: BrowserEngine["request"], machines: BrowserMachine[] = [DRAGON]) {
  return render(<BrowserWorkbench machines={machines} engine={{ request: (request, bytes, signal) => request.operation === "presentation" ? Promise.resolve({ schemaVersion: 1, ok: true, result: { highlights: [] } }) : request.operation === "create-project" ? Promise.resolve({ schemaVersion: 1, ok: true, result: { entryPath: request.kind === "basic" ? "src/main.bas" : "src/recovered.asm", project: { schemaVersion: request.target === "dragon32" ? 1 : 2, ...(request.target !== "dragon32" ? { target: request.target, supportFiles: {} } : {}), name: request.name, kind: request.kind, source: request.source, manifest: request.kind === "basic" ? "test-manifest" : null } } }) : request.operation === "import-project" ? Promise.resolve({ schemaVersion: 1, ok: true, result: { entryPath: "src/main.bas", project: JSON.parse(new TextDecoder().decode(bytes)) } }) : handler(request, bytes, signal) }} Emulator={Emulator} assetBase="/studio/" />);
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

it("starts with a project welcome, inert shell and hides technical imports", async () => {
  const request = vi.fn(); mount(request);
  expect(screen.getByRole("navigation", { name: "Workbench views" })).toBeInTheDocument();
  expect(screen.getByRole("heading", { name: "Get Started" })).toBeInTheDocument();
  expect(screen.getByRole("img", { name: "Eightbit Studio" }).querySelector("img")?.getAttribute("src")).toBe(cpuMark);
  expect(screen.getByLabelText("Raw load and entry address")).not.toBeVisible();
  expect(screen.queryByRole("button", { name: "Debug" })).not.toBeInTheDocument();
  expect(screen.getByRole("button", { name: "Play" })).toBeDisabled();
  expect(screen.getByRole("button", { name: "Check & build" })).toBeDisabled();
  expect(request).not.toHaveBeenCalled();
  await begin(true);
  expect(screen.getByRole("textbox", { name: "Dragon 32 BASIC source" })).toHaveValue("");
});

it.each([DRAGON, C64, SPECTRUM, ZX81])("explains ROM setup and keeps source and licence links available for $label", (machine) => {
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
  expect(within(source).getByRole("link", { name: "Source code on GitHub" })).toHaveAttribute("href", "https://github.com/robtyers/eightbit-studio-browser/releases/tag/browser-0.5.2-20261010.1");
  expect(request).not.toHaveBeenCalled();
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

it("previews source and drafts, requires confirmation before replacement, and limits assembly to inspection", async () => {
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
  await screen.findByRole("textbox", { name: "Recovered 6809 assembly source" });
  expect(screen.getByRole("textbox", { name: "Recovered 6809 assembly source" })).toHaveValue("fcb $00");
  expect(screen.getByRole("button", { name: "Check & build" })).toBeDisabled();
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
