import { useCallback, useEffect, useRef, useState, type ComponentType } from "react";
import { WorkbenchShell } from "@eightbitstudio/workbench/components/WorkbenchShell";
import { EditorToolbar } from "@eightbitstudio/workbench/components/EditorToolbar";
import { EditorTabs } from "@eightbitstudio/workbench/components/EditorTabs";
import { FilesIcon, SettingsIcon, PlayIcon, SaveIcon } from "@eightbitstudio/workbench/components/Icons";
import type { EditorCommandId } from "@eightbitstudio/workbench/services/editorCommands";
import { BrowserCoreError, buildResult, decompiledSource, MAX_FILE_BYTES, MAX_SOURCE_BYTES, record, result, verifiedFirmware, type BrowserDiagnostic, type BrowserEngine, type BrowserRequest, type BuildResult, type DecompiledSource, type BrowserMachine } from "./contracts";
import { BrowserDialog } from "./BrowserDialog";
import { BrowserEditor } from "./BrowserEditor";
import { ProjectWelcome } from "./ProjectWelcome";
import { GitHubDialog } from "./GitHubDialog";
import { SameOriginGitHub, type GitHubClient, type RemoteProject } from "./github";
import { projectId, projectTarget, projectResult, recentProjects, saveProject, type CloudLink, type ProjectSnapshot, type StoredProject } from "./projects";
import SAMPLE_SOURCE from "./guess-the-number.bas?raw";
import sourceRelease from "./source-release.json";

export interface EmulatorProps {
  machine: BrowserMachine;
  session: number;
  firmware: Uint8Array;
  media: Uint8Array;
  onState: (state: "ready" | "failed") => void;
}
interface Props { machines: BrowserMachine[]; engine: BrowserEngine; Emulator: ComponentType<EmulatorProps>; assetBase: string; github?: GitHubClient }
const DRAFT_KEY = "eightbit-studio.browser.dragon32.draft.v1";
const DEFAULT_GITHUB = new SameOriginGitHub();
const NO_BYTES = new Uint8Array();
const SOURCE_RELEASE_URL = `https://github.com/${sourceRelease.repository}/releases/tag/${sourceRelease.tag}`;

/** Browser capabilities populate the existing shell; no native host service is loaded. */
export function BrowserWorkbench({ engine, Emulator, machines, github = DEFAULT_GITHUB }: Props) {
  const [project, setProject] = useState<StoredProject>();
  const localCopy = useRef<StoredProject | undefined>(undefined);
  const [recent, setRecent] = useState<StoredProject[]>(() => { try { return recentProjects(); } catch { return []; } });
  const [storageState, setStorageState] = useState("Recovery copies stay on this device");
  const [newTarget, setNewTarget] = useState("dragon32");
  const [recoveryTarget, setRecoveryTarget] = useState("dragon32");
  const machine = machines.find((item) => item.id === (project ? projectTarget(project.project) : newTarget)) ?? machines[0]!;
  const newMachine = machines.find((item) => item.id === newTarget) ?? machine;
  const [newName, setNewName] = useState("My first game");
  const [template, setTemplate] = useState("sample");
  const [selectedFile, setSelectedFile] = useState<string>();
  const [showHelp, setShowHelp] = useState(false);
  const [source, setSource] = useState("");
  const [hasDocument, setHasDocument] = useState(false);
  const [sampleHint, setSampleHint] = useState(false);
  const [kind, setKind] = useState<"basic" | "assembly">("basic");
  const [dirty, setDirty] = useState(false);
  const [sidebarOpen, setSidebarOpen] = useState(() => window.innerWidth > 700);
  const [view, setView] = useState<"explorer" | "settings">("explorer");
  const [panel, setPanel] = useState("output");
  const [panelOpen, setPanelOpen] = useState(false);
  const [message, setMessage] = useState("You can write, check and save code before setting up the virtual computer.");
  const [diagnostics, setDiagnostics] = useState<BrowserDiagnostic[]>([]);
  const [dialog, setDialog] = useState<"save" | "open" | "recover" | "rom" | "create" | "github">();
  const [busy, setBusy] = useState(false);
  const [built, setBuilt] = useState<{ revision: number; result: BuildResult }>();
  const [preview, setPreview] = useState<DecompiledSource>();
  const [firmwareReady, setFirmwareReady] = useState(false);
  const [running, setRunning] = useState<Omit<EmulatorProps, "onState">>();
  const [rawAddress, setRawAddress] = useState("1024");
  const revision = useRef(1);
  const recoveryProjectId = useRef<string | undefined>(undefined);
  const sequence = useRef(0);
  const runSequence = useRef(0);
  const work = useRef<AbortController | undefined>(undefined);
  const firmware = useRef<Uint8Array | undefined>(undefined);
  const firmwareTarget = useRef<string | undefined>(undefined);
  const editor = useRef<HTMLTextAreaElement>(null);
  const active = useRef(false);
  active.current = Boolean(running) || busy;
  const closeDialog = useCallback(() => { work.current?.abort(); sequence.current++; setBusy(false); setDialog(undefined); }, []);
  const reveal = (next = "output") => { setPanel(next); setPanelOpen(true); };

  const stop = useCallback(() => { setRunning(undefined); }, []);
  useEffect(() => {
    let compact = window.innerWidth <= 700;
    if (compact) setSidebarOpen(false);
    const resize = () => {
      const next = window.innerWidth <= 700;
      if (next && !compact) setSidebarOpen(false);
      compact = next;
    };
    window.addEventListener("resize", resize);
    return () => window.removeEventListener("resize", resize);
  }, []);
  useEffect(() => {
    const suspend = () => {
      if (document.visibilityState === "hidden") {
        work.current?.abort(); sequence.current++; setBusy(false); stop();
        if (active.current) setMessage("Paused while this page was hidden. Choose Play to start again, or repeat the cancelled check.");
      }
    };
    const leave = () => { work.current?.abort(); stop(); firmware.current?.fill(0); firmware.current = undefined; setFirmwareReady(false); };
    document.addEventListener("visibilitychange", suspend);
    window.addEventListener("pagehide", leave);
    return () => {
      document.removeEventListener("visibilitychange", suspend);
      window.removeEventListener("pagehide", leave);
      work.current?.abort(); sequence.current++; firmware.current?.fill(0); firmware.current = undefined; recoveryProjectId.current = undefined;
    };
  }, [stop]);
  useEffect(() => {
    const protect = (event: BeforeUnloadEvent) => { if (dirty && revision.current > 1) event.preventDefault(); };
    window.addEventListener("beforeunload", protect);
    return () => window.removeEventListener("beforeunload", protect);
  }, [dirty]);

  function cancel() {
    work.current?.abort(); sequence.current++; setBusy(false); setMessage("Cancelled. Your source is unchanged.");
  }
  const dismissRom = useCallback(() => {
    work.current?.abort(); sequence.current++; setBusy(false); setDialog(undefined);
    setMessage("Computer setup closed. Choose Play when you are ready to set it up.");
  }, []);
  function edit(next: string, nextKind = kind) {
    if (new TextEncoder().encode(next).length > MAX_SOURCE_BYTES) { setMessage("Source exceeds the 1 MiB browser limit."); return; }
    work.current?.abort(); sequence.current++; revision.current++;
    setBusy(false); setBuilt(undefined); setDiagnostics([]); stop(); setSource(next); setKind(nextKind); setDirty(true); setHasDocument(true);
  }
  const currentSnapshot: ProjectSnapshot | undefined = project ? { ...project.project, source, kind } : undefined;
  const recoveryQueue = useRef<Promise<void>>(Promise.resolve());
  function persist(snapshot: ProjectSnapshot, entry: StoredProject) {
    const atRevision = revision.current;
    recoveryQueue.current = recoveryQueue.current.then(async () => {
      if (revision.current !== atRevision || recoveryProjectId.current !== entry.id) return;
      try {
        const saved = await saveProject({ schemaVersion: 1, id: entry.id, project: snapshot, entryPath: entry.entryPath, ...(entry.cloud ? { cloud: entry.cloud } : {}) }, localCopy.current?.id === entry.id ? localCopy.current.version || undefined : undefined);
        if (recoveryProjectId.current !== entry.id) return;
        if (revision.current !== atRevision) { if (localCopy.current?.id === entry.id) localCopy.current = saved; return; }
        localCopy.current = saved; setProject(saved); setDirty(false); setStorageState("Saved on this device"); setRecent(recentProjects());
      } catch { if (revision.current === atRevision && recoveryProjectId.current === entry.id) { setDirty(true); setStorageState("Local recovery failed — download a copy"); } }
    });
  }
  useEffect(() => {
    if (!project) return;
    const previous = localCopy.current;
    if (previous?.id === project.id && previous.project.source === source && previous.project.kind === kind && previous.cloud?.sha === project.cloud?.sha && previous.cloud?.repository === project.cloud?.repository) return;
    setDirty(true); setStorageState("Saving on this device…");
    const snapshot = { ...project.project, source, kind };
    const timer = window.setTimeout(() => persist(snapshot, project), 350);
    return () => window.clearTimeout(timer);
  }, [project?.id, project?.cloud?.repository, project?.cloud?.sha, source, kind]);
  function canReplace() {
    return !dirty || window.confirm("The latest changes have not recovered locally. Download a project copy before switching, or continue to discard those changes.");
  }
  function installProject(next: { project: ProjectSnapshot; entryPath: string }, saved?: StoredProject, cloud?: CloudLink) {
    const nextTarget = projectTarget(next.project);
    if (!machines.some((item) => item.id === nextTarget)) throw new Error("This computer is unsupported in the browser edition.");
    if (nextTarget !== firmwareTarget.current) { firmware.current?.fill(0); firmware.current = undefined; firmwareTarget.current = undefined; setFirmwareReady(false); }
    setNewTarget(nextTarget); setRecoveryTarget(nextTarget);
    const entry: StoredProject = saved ?? { schemaVersion: 1, id: projectId(), version: "", updatedAt: Date.now(), ...next, ...(cloud ? { cloud } : {}) };
    recoveryProjectId.current = entry.id; localCopy.current = saved; setProject(entry); edit(next.project.source, next.project.kind);
    setSelectedFile(undefined); setSampleHint(false); setShowHelp(false); setPreview(undefined); setDialog(undefined); setPanelOpen(false); setSidebarOpen(window.innerWidth > 700);
    setMessage("Project open. Choose Check & build when you are ready to try your game.");
    persist(next.project, entry);
    window.requestAnimationFrame(() => editor.current?.focus());
  }
  function createProject(name = newName, listing = template === "sample" ? newMachine.sample : "", language = kind, target = newTarget) {
    if (!canReplace()) return;
    void operate({ schemaVersion: 1, operation: "create-project", name, source: listing, kind: language, target }, NO_BYTES, (value) => { installProject(projectResult(value)); setSampleHint(listing === SAMPLE_SOURCE); });
  }
  async function loadProject(file: File) {
    if (!canReplace()) return;
    const atRevision = revision.current; const ticket = ++sequence.current;
    try {
      const bytes = await read(file);
      if (atRevision !== revision.current || ticket !== sequence.current) return;
      void operate({ schemaVersion: 1, operation: "import-project" }, bytes, (value) => installProject(projectResult(value)));
    } catch { reveal(); setMessage("Cannot open this project file. Your current project is preserved."); }
  }
  function openRecent(entry: StoredProject, afterConnect = false) {
    if (!canReplace()) return;
    let latest: StoredProject | undefined;
    try { latest = recentProjects().find((item) => item.id === entry.id); } catch { reveal(); setMessage("Browser storage is unavailable. Open a project download instead."); return; }
    if (!latest) { reveal(); setMessage("This recovery copy is no longer available. Open a downloaded or GitHub copy."); return; }
    const saved = latest;
    void operate({ schemaVersion: 1, operation: "import-project" }, new TextEncoder().encode(JSON.stringify(saved.project)), (value) => { installProject(projectResult(value), saved); if (afterConnect) setDialog("github"); });
  }
  useEffect(() => {
    const match = /^#github(?:=([a-f0-9-]{36}))?$/.exec(window.location.hash);
    if (!match) return;
    const entry = recent.find((item) => item.id === match[1]);
    if (entry) openRecent(entry, true); else setDialog("github");
    window.history.replaceState(null, "", window.location.pathname + window.location.search);
    // The fragment is produced by the explicit GitHub connection callback only.
  }, []);
  function openRemote(remote: RemoteProject, repository: string) {
    if (!canReplace()) return;
    setDialog(undefined);
    void operate({ schemaVersion: 1, operation: "import-project" }, new TextEncoder().encode(JSON.stringify(remote.project)), (value) => installProject(projectResult(value), undefined, { repository, projectId: remote.id, sha: remote.sha }));
  }
  function savedRemote(cloud: CloudLink, savedSource: string) {
    if (!project) return;
    const next = { ...project, cloud }; setProject(next);
    persist({ ...project.project, source, kind }, next);
    setMessage(savedSource === source ? "Project saved to GitHub. Later changes stay local until you save again." : "The selected snapshot was saved to GitHub. Your newer changes still need a GitHub save.");
  }
  function exportProject() {
    if (!currentSnapshot) return;
    void operate({ schemaVersion: 1, operation: "export-project", project: currentSnapshot }, NO_BYTES, (value) => {
      const data = result(value);
      if (!Array.isArray(data.archive) || data.archive.length > MAX_FILE_BYTES || !data.archive.every((byte: unknown) => typeof byte === "number" && Number.isInteger(byte) && byte >= 0 && byte <= 255)) throw new Error("Invalid project archive response.");
      closeDialog(); download(Uint8Array.from(data.archive), `${project?.project.name.replaceAll(" ", "-") ?? "project"}.zip`, "application/zip");
    });
  }
  async function operate(request: BrowserRequest, bytes: Uint8Array, accept: (value: unknown, atRevision: number) => void) {
    if (work.current && !work.current.signal.aborted) return;
    const controller = new AbortController(); work.current = controller;
    const ticket = ++sequence.current; const atRevision = revision.current;
    setBusy(true); setDiagnostics([]); reveal(); setMessage(request.operation === "firmware" ? `Checking your ${machine.label} system ROM…` : request.operation === "decompile" ? "Recovering a source preview…" : request.operation !== "test-project" && request.operation.includes("project") ? "Preparing your project locally…" : "Checking your code and preparing the game file…");
    try {
      const value = await engine.request(request, bytes, controller.signal);
      if (!controller.signal.aborted && ticket === sequence.current && atRevision === revision.current) accept(value, atRevision);
    } catch (error: unknown) {
      if (!controller.signal.aborted && ticket === sequence.current) {
        setMessage(error instanceof BrowserCoreError ? error.summary : error instanceof Error ? error.message.slice(0, 2048) : "The browser operation failed.");
        setDiagnostics(error instanceof BrowserCoreError ? error.diagnostics : []);
        if (request.operation === "firmware") reveal("emulator");
      }
    } finally {
      if (ticket === sequence.current) { setBusy(false); work.current = undefined; }
    }
  }
  function build() {
    setBuilt(undefined);
    if (!currentSnapshot) return;
    void operate({ schemaVersion: 1, operation: "test-project", project: currentSnapshot }, NO_BYTES, (value, atRevision) => {
      const artefact = buildResult(value);
      if (!artefact.roundTripVerified) throw new Error("The game file check did not complete. Choose Check & build to try again.");
      setBuilt({ revision: atRevision, result: artefact });
      setMessage("Game file ready. Code and programme file checks passed. Choose Play to try it, or Save work to keep a copy.");
    });
  }
  function download(bytes: BlobPart, filename: string, type: string) {
    try {
      const url = URL.createObjectURL(new Blob([bytes], { type }));
      const link = document.createElement("a"); link.href = url; link.download = filename;
      document.body.append(link); link.click(); link.remove();
      // Safari needs the URL to remain available after the click dispatch.
      window.setTimeout(() => URL.revokeObjectURL(url), 60_000);
      reveal(); setMessage(`Download requested: ${filename}. Check Files or Downloads to confirm it was saved.`);
    } catch { reveal(); setMessage("Download failed. Your code remains open; try again."); }
  }
  function run() {
    if (!built || built.revision !== revision.current || busy || running || kind !== "basic") return;
    if (!firmware.current || firmwareTarget.current !== machine.id) {
      reveal("emulator"); setMessage(`Choose your ${machine.label} system ROM to play. Your game file is ready.`); setDialog("rom"); return;
    }
    reveal("emulator"); setMessage(`Starting your virtual ${machine.label}…`);
    setRunning({ machine, session: ++runSequence.current, firmware: firmware.current.slice(), media: built.result.cassette.slice() });
  }
  function command(id: EditorCommandId) {
    if (id === "save" && hasDocument) setDialog("save");
    if (id === "build" && hasDocument && kind === "basic" && !busy && !running) build();
    if (id === "run") run();
    if (id === "stop") { stop(); reveal(); setMessage("Game stopped. Your code and game file are ready to use again."); }
  }
  const availability = {
    save: hasDocument,
    build: hasDocument && kind === "basic" && !busy && !running,
    run: built?.revision === revision.current && !busy && !running && kind === "basic",
    stop: Boolean(running), debug: false,
  };

  async function read(file: File): Promise<Uint8Array> {
    if (file.size > MAX_FILE_BYTES) throw new Error("Choose a file no larger than 8 MiB.");
    return new Uint8Array(await file.arrayBuffer());
  }
  async function openSource(file: File) {
    const atRevision = revision.current;
    const ticket = ++sequence.current;
    try {
      const bytes = await read(file);
      if (atRevision !== revision.current || ticket !== sequence.current) return;
      const text = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
      if (bytes.length > MAX_SOURCE_BYTES) throw new Error("Source exceeds the 1 MiB browser limit.");
      const importedKind = file.name.toLowerCase().endsWith(".asm") ? "assembly" : "basic";
      if (importedKind === "assembly" && recoveryTarget !== "dragon32") throw new Error("This browser edition supports BASIC for this computer. Open assembly source in the desktop edition.");
      setPreview({ kind: importedKind, source: text, warningCount: 0, target: recoveryTarget });
      reveal("import"); setMessage("Preview ready. Choose Edit this copy to open it; the original file is preserved.");
    } catch (error: unknown) { if (ticket === sequence.current) { reveal(); setMessage(error instanceof Error ? error.message : "Cannot read this source file."); } }
  }
  async function decompile(file: File, raw: boolean) {
    const atRevision = revision.current;
    const ticket = ++sequence.current;
    try {
      const bytes = await read(file);
      if (atRevision !== revision.current || ticket !== sequence.current) return;
      void operate({ schemaVersion: 1, operation: "decompile", target: raw ? "dragon32" : recoveryTarget, input: raw ? { format: "raw", loadAddress: Number(rawAddress), executionAddress: Number(rawAddress) } : { format: "cassette" } }, bytes, (value) => {
        setPreview({ ...decompiledSource(value), target: raw ? "dragon32" : recoveryTarget }); setPanel("import");
        setMessage("Recovered code is ready to preview. The original file is preserved.");
      });
    } catch (error: unknown) { if (ticket === sequence.current) { reveal(); setMessage(error instanceof Error ? error.message : "Cannot read this programme."); } }
  }
  async function selectFirmware(files: File[]) {
    firmware.current?.fill(0); firmware.current = undefined; setFirmwareReady(false); stop();
    const atRevision = revision.current; const ticket = ++sequence.current;
    let bytes: Uint8Array | undefined;
    try {
      const ordered = files.length === 1 ? files : machine.romNames.map((name) => {
        const file = files.find((file) => file.name === name);
        if (!file) throw new Error(`Choose all required ROM files: ${machine.romNames.join(", ")}.`);
        return file;
      });
      if (files.length !== ordered.length || ordered.reduce((size, file) => size + file.size, 0) !== machine.romSize) throw new Error(`Choose the supported ${machine.label} ROM files (${machine.romSize / 1024} KiB in total).`);
      bytes = new Uint8Array(machine.romSize); let offset = 0;
      for (const file of ordered) { const part = await read(file); bytes.set(part, offset); offset += part.length; part.fill(0); }
      if (atRevision !== revision.current || ticket !== sequence.current) { bytes.fill(0); return; }
      const selected = bytes;
      void operate({ schemaVersion: 1, operation: "firmware", target: machine.id }, selected, (value) => {
        if (!verifiedFirmware(value)) throw new Error(`This ROM does not match the supported ${machine.label} firmware revision.`);
        firmware.current = selected.slice(); firmwareTarget.current = machine.id; setFirmwareReady(true); closeDialog(); reveal("emulator"); setMessage("System ROM checked. Choose Play to start your game.");
      }).finally(() => selected.fill(0));
    } catch (error: unknown) { bytes?.fill(0); if (ticket === sequence.current) { reveal("emulator"); setMessage(error instanceof Error ? error.message : "Cannot inspect these ROMs."); } }
  }
  const romPickerLabel = `Choose ROM ${machine.romNames.length > 1 ? "files" : "file"}…`;
  const romPicker = <label className="browser-file-control">{romPickerLabel}<input type="file" aria-label={`${romPickerLabel} for ${machine.label}`} accept={machine.id === "bbc-model-b" ? undefined : ".rom,.bin"} multiple={machine.romNames.length > 1} disabled={busy || Boolean(running)} onChange={(event) => { const files = Array.from(event.currentTarget.files ?? []); event.currentTarget.value = ""; if (files.length) void selectFirmware(files); }} /></label>;
  function restoreLegacyDraft() {
    reveal();
    try {
      const text = localStorage.getItem(DRAFT_KEY);
      if (!text || text.length > MAX_SOURCE_BYTES * 6 + 256) throw new Error("No valid legacy browser draft is available.");
      const data: unknown = JSON.parse(text);
      if (!record(data) || data.schemaVersion !== 1 || (data.kind !== "basic" && data.kind !== "assembly") || typeof data.source !== "string" || new TextEncoder().encode(data.source).length > MAX_SOURCE_BYTES) throw new Error("The stored draft is invalid.");
      setPreview({ kind: data.kind, source: data.source, warningCount: 0, target: "dragon32" }); setPanel("import"); setPanelOpen(true);
      setMessage("Legacy draft ready to preview. Edit this copy creates a named project; the original draft is preserved.");
    } catch (error: unknown) { setMessage(error instanceof Error ? error.message : "Browser storage is unavailable. Open a downloaded copy instead."); }
  }
  function usePreview() {
    if (!preview || !canReplace()) return;
    createProject(preview.kind === "basic" ? "Imported game" : "Recovered code", preview.source, preview.kind, preview.target ?? machine.id);
    setMessage(preview.kind === "assembly" ? "Machine code is available to inspect and save. To assemble it, use the desktop edition." : "Your code is ready to edit. Choose Check & build, then Play.");
  }
  useEffect(() => {
    const shortcut = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || event.altKey || event.shiftKey) return;
      const id = event.key.toLowerCase() === "s" ? "save" : event.key.toLowerCase() === "b" ? "build" : event.key.toLowerCase() === "r" ? "run" : undefined;
      if (id) { event.preventDefault(); if (!dialog && availability[id]) command(id); }
    };
    window.addEventListener("keydown", shortcut);
    return () => window.removeEventListener("keydown", shortcut);
  });
  const runtimeState = useCallback((state: "ready" | "failed") => {
    setMessage(state === "ready" ? "Your game is running. Tap the screen for keyboard input; Stop ends the session." : "The virtual computer could not start. Your code and game file remain available; choose Play to try again.");
    if (state === "failed") { stop(); setPanel("output"); }
  }, [stop]);
  function fileControl(label: string, accept: string, callback: (file: File) => void) {
    return <label className="browser-file-control"><FilesIcon /><span>{label}</span><input type="file" accept={accept} disabled={busy || Boolean(running)} onChange={(event) => { const file = event.currentTarget.files?.[0]; event.currentTarget.value = ""; if (file) callback(file); }} /></label>;
  }
  const filename = project?.entryPath ?? (kind === "basic" ? "game.bas" : "recovered.asm");
  const nextStep = !hasDocument ? "Try a sample game or open your work to get started." : kind === "assembly" ? "Recovered machine code can be inspected and saved. Assembly builds are available in the desktop edition." : running ? "Your game is running. Stop it before checking another build." : busy ? "Working locally on your device. You can cancel in Results." : !built ? "Choose Check & build before playing your code." : !firmwareReady ? `Game file ready. Choose Play to set up your virtual ${machine.label}.` : "Game file ready. Choose Play, or Save work to keep a copy.";
  function goToDiagnostic(item: BrowserDiagnostic) {
    const lines = source.split("\n");
    const row = Math.min(item.row - 1, lines.length - 1);
    const offset = lines.slice(0, row).reduce((total, line) => total + line.length + 1, 0) + Math.min(item.column - 1, lines[row]?.length ?? 0);
    editor.current?.focus(); editor.current?.setSelectionRange(offset, offset);
  }
  const romSetup = <div className="browser-rom-setup">
    {dialog !== "rom" && <h2>{firmwareReady ? `Your virtual ${machine.label} is ready` : `Set up your virtual ${machine.label}`}</h2>}
    <p>{firmwareReady ? "System ROM checked. Choose Play to start your game." : `To play, choose your ${machine.label} system ROM: the original computer’s built-in software.`}</p>
    {!firmwareReady && romPicker}
    <p className="browser-muted">{machine.romNames.join(" + ")} · {machine.romSize / 1024} KiB total. Select the named files together, or one combined file in that order.</p>
    <p className="browser-muted">Your ROM stays on this device, only while this page is open. You can edit, check and save code without it.</p>
  </div>;
  const sidebar = <aside id="primary-sidebar" className="sidebar" aria-label={view === "explorer" ? "Explorer" : "Computer setup"}>
    <div className="sidebar-heading">{view === "explorer" ? "EXPLORER" : "COMPUTER SETUP"}</div>
    <div className="browser-sidebar-content">
      <strong>Eightbit Studio</strong><span className="browser-muted">{machine.label} · Browser edition</span>
      {view === "explorer" ? <>
        {hasDocument && <strong>{project?.project.name}</strong>}
        {hasDocument && <button type="button" onClick={() => { setSelectedFile(undefined); window.requestAnimationFrame(() => editor.current?.focus()); }}><FilesIcon />{filename}</button>}
        {hasDocument && project?.project.manifest && <button type="button" onClick={() => setSelectedFile(machine.manifest)}><FilesIcon />{machine.manifest}</button>}
        {hasDocument && Object.keys(project?.project.supportFiles ?? {}).map((path) => <button type="button" key={path} onClick={() => setSelectedFile(path)}><FilesIcon />{path}</button>)}
        <button type="button" disabled={busy || Boolean(running)} onClick={() => setDialog("open")}><FilesIcon />Load existing project…</button>
        <button type="button" disabled={busy || Boolean(running)} onClick={() => { setNewName("My next game"); setDialog("create"); }}>Create new project…</button>
        {hasDocument && <button type="button" onClick={() => { if (canReplace()) { stop(); work.current?.abort(); sequence.current++; setBusy(false); setProject(undefined); localCopy.current = undefined; recoveryProjectId.current = undefined; setHasDocument(false); setDirty(false); setBuilt(undefined); setPanelOpen(false); setPreview(undefined); } }}>Project home</button>}
        {hasDocument && <button type="button" onClick={() => setShowHelp((value) => !value)} aria-expanded={showHelp}>Getting started help</button>}
        <button type="button" disabled={busy || Boolean(running)} onClick={() => setDialog("recover")}>Recover source…</button>
        <details hidden={machine.cpu !== "6809"} className="browser-advanced"><summary>Advanced import</summary><p>Raw machine-code files need a known memory address: where the computer loads and starts the code. BASIC editing does not need this.</p>
          <label>Load / start address (decimal)<input aria-label="Raw load and entry address" type="number" min="1" max="65535" value={rawAddress} disabled={busy || Boolean(running)} onChange={(event) => setRawAddress(event.currentTarget.value)} /></label>
          {fileControl("Choose raw binary…", ".bin", (file) => { void decompile(file, true); })}
        </details>
      </> : <>
        <section className="browser-settings-section" aria-labelledby="browser-rom-heading">
          <h2 id="browser-rom-heading">System ROM</h2>
          <p>The computer’s built-in software. Add your own to play; editing and saving work without it.</p>
          {romPicker}
          {firmwareReady && <p>ROM ready · kept only until this page closes.</p>}
          <button type="button" disabled={!firmwareReady} onClick={() => { stop(); firmware.current?.fill(0); firmware.current = undefined; setFirmwareReady(false); reveal(); setMessage("System ROM removed from this page. Choose it again when you want to play."); }}>Remove ROM</button>
        </section>
        <section className="browser-settings-section" aria-labelledby="browser-source-heading">
          <h2 id="browser-source-heading">Open-source software</h2>
          <p>Explore the code, build guide and software licences.</p>
          <a href={SOURCE_RELEASE_URL} target="_blank" rel="noreferrer">Source code on GitHub</a>
        </section>
      </>}
    </div>
  </aside>;
  const output = <div className="browser-output"><p role={diagnostics.length ? "alert" : "status"}>{message}</p>
    {diagnostics.length > 0 && <ul className="browser-diagnostics">{diagnostics.map((item, index) => <li key={index}><button type="button" onClick={() => goToDiagnostic(item)}>Line {item.row}, column {item.column}: {item.message}</button></li>)}</ul>}
    {busy && <button type="button" onClick={cancel}>Cancel operation</button>}
    {(built || diagnostics.length > 0) && <details><summary>Technical details</summary>{built && <><p>{built.result.tokenBytes} token bytes · {built.result.roundTripVerified ? "Programme file, tokens and listing round trip verified" : "Built without a round-trip check"}</p><p>Cassette SHA-256: <code>{built.result.cassetteSha256}</code></p></>}{diagnostics.map((item, index) => <p key={index}><code>{item.code}</code> · Line {item.row}, column {item.column}</p>)}</details>}
  </div>;
  const welcome = <ProjectWelcome machines={machines} recent={recent} busy={busy} onCreate={() => { setNewName("My first game"); setDialog("create"); }} onOpen={() => setDialog("open")} onRecent={openRecent} onSettings={() => { setView("settings"); setSidebarOpen(true); }} onGitHub={() => setDialog("github")} message={storageState.includes("failed") ? storageState : ""} />;
  return <div className="browser-workbench"><WorkbenchShell sidebarOpen={sidebarOpen} sidebar={sidebar}
    activity={<><button type="button" className={view === "explorer" && sidebarOpen ? "active" : ""} aria-label="Explorer" aria-expanded={view === "explorer" && sidebarOpen} onClick={() => { setView("explorer"); setSidebarOpen(view !== "explorer" || !sidebarOpen); }}><FilesIcon /></button><div className="activity-spacer"/><button type="button" aria-label="Computer setup" className={view === "settings" && sidebarOpen ? "active" : ""} aria-expanded={view === "settings" && sidebarOpen} onClick={() => { setView("settings"); setSidebarOpen(view !== "settings" || !sidebarOpen); }}><SettingsIcon /></button></>}
    toolbar={<EditorToolbar enabled={availability} sessionActive={Boolean(running)} supportsDebug={false} keepSaveVisible labels={{ save: "Save work", build: "Check & build", run: "Play" }} onCommand={command}>{machine.label} / {hasDocument ? project?.project.name ?? filename : "Projects"}</EditorToolbar>}
    editor={<>{hasDocument && <EditorTabs tabs={[{ path: filename, dirty, badge: kind === "basic" ? "B" : "A", badgeClass: "basic" }, ...(selectedFile ? [{ path: selectedFile, dirty: false, badge: "", badgeClass: "" }] : [])]} activePath={selectedFile ?? filename} onSelect={(path) => setSelectedFile(path === filename ? undefined : path)} onClose={(path) => { if (path !== filename) { setSelectedFile(undefined); return; } if (canReplace()) { stop(); work.current?.abort(); sequence.current++; setBusy(false); setBuilt(undefined); setProject(undefined); localCopy.current = undefined; recoveryProjectId.current = undefined; setHasDocument(false); setDirty(false); setPreview(undefined); setPanelOpen(false); } }} />}
      {hasDocument && showHelp && <div className="browser-guide" aria-label="Game creation steps"><ol><li aria-current={!built && !running && !busy ? "step" : undefined}>1 Edit</li><li aria-current={busy ? "step" : undefined}>2 Check &amp; build</li><li aria-current={built ? "step" : undefined}>3 Play</li><li>4 Save</li></ol><p>{nextStep}</p></div>}
      {hasDocument ? <>{kind === "basic" && showHelp && <p className="browser-editor-tip">{sampleHint ? "Try changing the winning number on line 40. " : ""}BASIC line numbers set the order of instructions; they are not memory addresses.</p>}{selectedFile ? <div className="browser-project-file"><p className="browser-muted">Project guidance and settings are preserved in downloads and GitHub saves. Edit these files in the desktop edition; this browser edits the programme source.</p><textarea aria-label={`Project file ${selectedFile}`} readOnly value={selectedFile === machine.manifest ? project?.project.manifest ?? "" : project?.project.supportFiles[selectedFile] ?? ""} /></div> : <BrowserEditor target={machine.id} label={machine.label} source={source} basic={kind === "basic"} busy={busy} engine={engine} inputRef={editor} onChange={edit} />}</> : welcome}</>}
    panel={{ open: panelOpen, active: panel, onSelect: (id) => reveal(id), onClose: () => { setPanelOpen(false); stop(); }, label: panel === "emulator" ? "Game" : panel === "import" ? "Source preview" : "Results", tabs: [{ id: "output", label: "Results" }, { id: "emulator", label: "Game" }, { id: "import", label: "Source preview" }], content: <>
      {running && <div hidden={panel !== "emulator"}><Emulator key={running.session} {...running} onState={runtimeState} /></div>}
      {panel === "emulator" && !running && <div className="browser-output">{!busy && dialog !== "rom" && romSetup}<p role="status">{busy ? message : built ? message : "Choose Check & build to prepare a game, then Play."}</p>{built && !busy && firmwareReady && <button type="button" onClick={run}><PlayIcon />Play game</button>}{busy && <button type="button" onClick={cancel}>Cancel operation</button>}</div>}
      {panel === "output" && output}
      {panel === "import" && (preview ? <div className="browser-import"><div className="browser-import-heading"><p>{preview.kind === "basic" ? "Recovered editable BASIC code" : "Reconstructed machine code (6809 assembly)"} · {preview.warningCount} warnings</p><button type="button" disabled={busy || Boolean(running)} onClick={usePreview}>Edit this copy</button></div>{preview.kind === "assembly" && <p className="browser-muted">Original names and comments cannot be recovered. Inspect or save this copy; assembly builds use the desktop edition.</p>}<textarea aria-label="Imported source preview" readOnly value={preview.source} /></div> : <div className="browser-output"><p>Open your work or recover source from a game to inspect a copy.</p><button type="button" disabled={busy || Boolean(running)} onClick={() => setDialog("recover")}>Recover source…</button></div>)}
    </> }}
    status={<><span>{machine.label}</span><span>{kind === "basic" ? "BASIC" : "6809 assembly"}</span><span>{busy ? "Working…" : running ? "Playing" : "On your device"}</span><button type="button" onClick={() => reveal()}>Results</button><span className="browser-storage-state">{hasDocument ? storageState : "Projects"}</span></>} />
    {dialog === "github" && <GitHubDialog client={github} localProjectId={project?.id} project={currentSnapshot} cloud={project?.cloud} onClose={closeDialog} onOpen={openRemote} onSaved={savedRemote} />}
    {dialog && dialog !== "github" && <BrowserDialog title={dialog === "create" ? "Create new project" : dialog === "save" ? "Save your work" : dialog === "open" ? "Load existing project" : dialog === "rom" ? `Set up your virtual ${machine.label}` : "Recover source from a game"} onClose={dialog === "rom" ? dismissRom : closeDialog}>
      {dialog === "create" ? <form onSubmit={(event) => { event.preventDefault(); createProject(newName, template === "sample" ? newMachine.sample : "", "basic", newTarget); }}>
        <label className="browser-field">Project name<input autoFocus required maxLength={64} value={newName} onChange={(event) => setNewName(event.currentTarget.value)} /></label>
        <label className="browser-field">Computer<select value={newTarget} onChange={(event) => setNewTarget(event.currentTarget.value)}>{["6809", "6502", "Z80"].map((cpu) => <optgroup key={cpu} label={cpu}>{machines.filter((machine) => machine.cpu === cpu).map((machine) => <option key={machine.id} value={machine.id}>{machine.label}</option>)}</optgroup>)}</select></label>
        <label className="browser-field">Start with<select value={template} onChange={(event) => setTemplate(event.currentTarget.value)}><option value="sample">{newTarget === "dragon32" ? "Guess the number — sample game" : "Starter BASIC programme"}</option><option value="blank">Blank BASIC project</option></select></label>
        <p className="browser-muted">{newMachine.label} BASIC. Use letters, numbers, spaces or hyphens. Your project will recover automatically on this device.</p>
        <button className="primary-button" type="submit" disabled={busy}>Create project</button><p role="status">{busy ? "Creating your project…" : message.startsWith("DSW1010") ? message : ""}</p>
      </form> : dialog === "rom" ? <>{romSetup}<p role="status">{message}</p>{busy && <button type="button" onClick={cancel}>Cancel operation</button>}</> : dialog === "save" ? <><p>{storageState}. Keep a backup or continue on another device.</p><div className="browser-save-options">
        <button type="button" disabled={busy} onClick={exportProject}><FilesIcon /><span>Download project ZIP<small>Includes source and desktop-compatible settings · reopen here or extract for the desktop edition</small></span></button>
        <button type="button" onClick={() => setDialog("github")}><SaveIcon /><span>Save to GitHub<small>Keep a copy in your private repository · continue on another device</small></span></button>
        <button type="button" onClick={() => { closeDialog(); download(source, filename.split("/").at(-1) ?? "game.bas", "text/plain;charset=utf-8"); }}><SaveIcon /><span>Save editable code<small>{filename} · keep this to continue editing</small></span></button>
        <button type="button" disabled={!built || busy} onClick={() => { if (built?.revision === revision.current) { closeDialog(); download(new Uint8Array(built.result.cassette), `game.${machine.extension}`, "application/octet-stream"); } }}><PlayIcon /><span>Save playable game file<small>game.{machine.extension} · for a compatible {machine.label} emulator{!built ? " · check & build first" : ""}</small></span></button>
        <button type="button" onClick={() => { if (currentSnapshot && project) { closeDialog(); const copy = { ...project, id: projectId(), version: "", cloud: undefined }; recoveryProjectId.current = copy.id; localCopy.current = undefined; setProject(copy); persist(currentSnapshot, copy); } }}><FilesIcon /><span>Keep a separate local copy<small>Preserve both copies if another tab changed this project</small></span></button>
      </div><p className="browser-muted">Local recovery can be cleared by the browser and is not synchronised with iCloud. Choose your exported file’s destination in Files or Downloads.</p></> : dialog === "open" ? <><p>Choose a saved project ZIP or browser project file. Source files and legacy drafts open as a preview.</p>
        {fileControl("Choose project file…", ".zip,.json", (file) => { closeDialog(); void loadProject(file); })}
        <button type="button" onClick={() => setDialog("github")}>Open from GitHub</button>
        <p className="browser-muted">Local recovery stays in this browser. Download or use GitHub for a durable copy. Single-source BASIC projects for the listed computers are supported; use the desktop edition for other project types.</p><label className="browser-field">Computer for source files<select value={recoveryTarget} onChange={(event) => setRecoveryTarget(event.currentTarget.value)}>{machines.map((machine) => <option key={machine.id} value={machine.id}>{machine.label}</option>)}</select></label>{fileControl("Choose source file…", recoveryTarget === "dragon32" ? ".bas,.asm,.txt" : ".bas,.txt", (file) => { closeDialog(); void openSource(file); })}<button type="button" disabled={busy || Boolean(running)} onClick={() => { closeDialog(); restoreLegacyDraft(); }}>Restore browser draft</button></> : <><label className="browser-field">Computer<select value={recoveryTarget} onChange={(event) => setRecoveryTarget(event.currentTarget.value)}>{machines.map((machine) => <option key={machine.id} value={machine.id}>{machine.label}</option>)}</select></label><p>Choose a BASIC programme file for this computer. Preview the recovered code, then edit a copy.</p>{fileControl("Choose game or recording…", ".cas,.wav,.prg,.tap,.p,.ssd", (file) => { closeDialog(); void decompile(file, false); })}<p className="browser-muted">BASIC games can recover editable code. Dragon machine-code games recover 6809 assembly for inspection and export. Originals are preserved.</p></>}
    </BrowserDialog>}
  </div>;
}
