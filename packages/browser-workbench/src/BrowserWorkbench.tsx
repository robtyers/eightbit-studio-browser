import type { VectrexSourceLaunch } from "./contracts";
import { originalDragonTarget, originalSpectrumTarget, spectrumModelName, type SpectrumMachine } from "@eightbitstudio/workbench/services/monitorFrame";
import { MachineDebugPanel } from "@eightbitstudio/workbench/components/MachineDebugPanel";
import { isMachineInspection, type OriginalHostConnection, type OriginalHostReply } from "@eightbitstudio/workbench/services/originalMachineDebug";
import { BasicDebugPanel } from "@eightbitstudio/workbench/components/BasicDebugPanel";
import { MonitorConsole } from "@eightbitstudio/workbench/components/MonitorConsole";
import type { DevelopmentHostConnection } from "@eightbitstudio/workbench/services/developmentHost";
import type { OriginalDebugReply } from "@eightbitstudio/workbench/services/originalBasicDebug";
import { originalRuntimeLabel, type OriginalRuntimeProfile } from "@eightbitstudio/workbench/services/originalRuntime";
import type { MonitorTransport } from "@eightbitstudio/workbench/services/monitorProtocol";
import type { C64SourceLaunch, SpectrumSourceLaunch } from "./contracts";
import {spectrumInspection,type SpectrumInspection,type SpectrumDebuggerPort} from "./spectrumDebug";
export interface VectrexMonitorProps {source:VectrexSourceLaunch;onStop:()=>void;onRuntime?:(release?:()=>void)=>void}
export interface C64MonitorProps {source: C64SourceLaunch; onStop: () => void; onRuntime?: (release?: () => void) => void}
export interface SpectrumMonitorProps {target?:SpectrumMachine;onStop:()=>void;onRuntime?:(release?:()=>void)=>void;source?:SpectrumSourceLaunch;onDebugger?:(port?:SpectrumDebuggerPort)=>void;onInspection?:(frame:unknown,sessionId:number,source?:SpectrumSourceLaunch)=>void}
type MonitorFirmware = "monitor" | "basic-development" | "spectrum-development" | "c64-original" | "vectrex-original";
export type { OriginalRuntimeSource } from "./contracts";
export type OriginalRuntime = (source: OriginalRuntimeInput, publish: (reply: OriginalHostReply) => void, released: () => void, profile: OriginalRuntimeProfile) => { owner: { sessionId: number; buildId: string; backend: "original" }; transport: MonitorTransport; connection?: Pick<OriginalHostConnection, "monitor" | "debug" | "dispose">; dispose: () => void };
export interface DevelopmentSource { schemaVersion: 1; source: string; sourceSha256: string; cassetteSha256: string; breakpointLine: number }
export type DevelopmentDebugger = (source: DevelopmentSource, publish: (reply: OriginalDebugReply) => void, released: () => void) => { sessionId: number; connection: DevelopmentHostConnection };
import { useCallback, useEffect, useRef, useState, type ComponentType } from "react";
import { WorkbenchShell } from "@eightbitstudio/workbench/components/WorkbenchShell";
import { EditorToolbar } from "@eightbitstudio/workbench/components/EditorToolbar";
import { EditorTabs } from "@eightbitstudio/workbench/components/EditorTabs";
import { FilesIcon, SettingsIcon, PlayIcon, SaveIcon } from "@eightbitstudio/workbench/components/Icons";
import type { EditorCommandId } from "@eightbitstudio/workbench/services/editorCommands";
import { BrowserCoreError, buildResult, cassetteAudio, decompiledSource, MAX_FILE_BYTES, MAX_SOURCE_BYTES, record, result, verifiedFirmware, type BrowserDiagnostic, type BrowserEngine, type BrowserRequest, type BuildResult, type DecompiledSource, type BrowserMachine, type OriginalRuntimeInput, type OriginalRuntimeMedia, type MediaInput } from "./contracts";
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
interface Props { originalProfiles?: Readonly<Record<string, OriginalRuntimeProfile>>; originalProfile?: OriginalRuntimeProfile | null; OriginalRuntime?: OriginalRuntime; DevelopmentDebugger?: DevelopmentDebugger; machines: BrowserMachine[]; engine: BrowserEngine; Emulator: ComponentType<EmulatorProps>; Monitor?: ComponentType<{ onStop: () => void; onRuntime?: (release?: () => void) => void; firmware?: "monitor" | "basic-development"; cassette?: Uint8Array; cassetteSha256?: string }>; SpectrumMonitor?: ComponentType<SpectrumMonitorProps>; C64Monitor?: ComponentType<C64MonitorProps>; VectrexMonitor?:ComponentType<VectrexMonitorProps>; assetBase: string; github?: GitHubClient }
const DRAFT_KEY = "eightbit-studio.browser.dragon32.draft.v1";
const DEFAULT_GITHUB = new SameOriginGitHub();
const NO_BYTES = new Uint8Array();
const SOURCE_RELEASE_URL = `https://github.com/${sourceRelease.repository}/releases/tag/${sourceRelease.tag}`;

/** Browser capabilities populate the existing shell; no native host service is loaded. */
export function BrowserWorkbench({ engine, Emulator, Monitor, SpectrumMonitor, C64Monitor, VectrexMonitor, DevelopmentDebugger, OriginalRuntime, originalProfile: defaultOriginalProfile, originalProfiles, machines, github = DEFAULT_GITHUB }: Props) {
  const [project, setProject] = useState<StoredProject>();
  const localCopy = useRef<StoredProject | undefined>(undefined);
  const [recent, setRecent] = useState<StoredProject[]>(() => { try { return recentProjects(); } catch { return []; } });
  const [storageState, setStorageState] = useState("Recovery copies stay on this device");
  const [newTarget, setNewTarget] = useState("dragon32");
  const [recoveryTarget, setRecoveryTarget] = useState("dragon32");
  const machine = machines.find((item) => item.id === (project ? projectTarget(project.project) : newTarget)) ?? machines[0]!;
  const originalProfile = originalProfiles?.[machine.id] ?? (machine.id === "dragon32" ? defaultOriginalProfile : undefined);
  const newMachine = machines.find((item) => item.id === newTarget) ?? machine;
  const [newName, setNewName] = useState("My first game");
  const [template, setTemplate] = useState("sample");
  const [assemblyLoad, setAssemblyLoad] = useState("28672");
  const [assemblyEntry, setAssemblyEntry] = useState("START");
  const [selectedFile, setSelectedFile] = useState<string>();
  const [sourcePaths, setSourcePaths] = useState<string[]>([]);
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
  const [diagnosticLocation, setDiagnosticLocation] = useState<{ sourceUri: string; row: number; column: number; revision: number }>();
  const [dialog, setDialog] = useState<"save" | "open" | "recover" | "rom" | "create" | "github">();
  const [busy, setBusy] = useState(false);
  const [built, setBuilt] = useState<{ revision: number; result: BuildResult }>();
  const [preview, setPreview] = useState<DecompiledSource>();
  const [machineMedia, setMachineMedia] = useState<{ revision: number; media: OriginalRuntimeMedia }>();
  const [firmwareReady, setFirmwareReady] = useState(false);
  const [running, setRunning] = useState<Omit<EmulatorProps, "onState">>();
  const [monitorOpen, setMonitorOpen] = useState<MonitorFirmware | undefined>(undefined);
  const [vectrexSource,setVectrexSource]=useState<VectrexSourceLaunch>();
  const vectrexSelected=machine.id === "vectrex" && Boolean(VectrexMonitor);
  const [c64Source,setC64Source] = useState<C64SourceLaunch>();
  const [c64Backend,setC64Backend] = useState<"chips-c64"|"original">("chips-c64");
  const c64Selected = machine.id === "c64-pal" && Boolean(C64Monitor) && c64Backend === "original";
  const [spectrumSource, setSpectrumSource] = useState<SpectrumSourceLaunch>();
  const spectrumLaunch=useRef<SpectrumSourceLaunch|undefined>(undefined);
  const spectrumPort=useRef<SpectrumDebuggerPort|undefined>(undefined);
  const spectrumPrevious=useRef("");
  const [spectrumStatus,setSpectrumStatus]=useState<SpectrumInspection>();
  const ownSpectrumDebugger=useCallback((port?:SpectrumDebuggerPort)=>{spectrumPort.current=port;},[]);
  const inspectSpectrum=useCallback((frame:unknown,id:number,launch?:SpectrumSourceLaunch)=>{
    if(!launch||spectrumLaunch.current!==launch)return;
    try {let state=spectrumInspection(frame,id,launch.revision,launch.target);if(!state&&launch.breakpointLine!==undefined&&record(frame)&&frame.debug===null&&frame.state==="running")state={sessionId:id,revision:launch.revision,state:"ended",stopId:0,valuesValid:false,breakpointVerified:false,breakpointAddress:0,sourceLine:null,registers:null,memory:null,disassembly:[],message:"Reset ended debugging. Stop, then Debug to load fresh source."};const key=JSON.stringify(state)??"";if(key===spectrumPrevious.current)return;spectrumPrevious.current=key;setSpectrumStatus(state);if(state?.state==="stopped"){setPanel("debug");setPanelOpen(true);}}
    catch {setSpectrumStatus(undefined);setMessage("Invalid or stale Spectrum debugger inspection. Stop and build again.");}
  },[]);
  const [monitorCassette, setMonitorCassette] = useState<{ bytes: Uint8Array; sha256: string }>();
  const monitorRelease = useRef<(() => void) | undefined>(undefined);
  const ownMonitor = useCallback((release?: () => void) => { monitorRelease.current = release; }, []);
  const [projectBackend, setProjectBackend] = useState<"xroar" | "original">("original");
  const [originalSession, setOriginalSession] = useState<ReturnType<OriginalRuntime>>();
  const originalSequence = useRef(0);
  const originalRef = useRef<ReturnType<OriginalRuntime> | undefined>(undefined);
  const originalMachine = useRef(machine.id); originalMachine.current = machine.id;
  const originalEligible = Boolean(originalProfile && OriginalRuntime && originalProfile.target === machine.id && ["dragon32", "dragon64", "dragon64-native"].includes(machine.id));
  const originalSelected = originalEligible && (machine.id !== "dragon32" || projectBackend === "original" || kind === "assembly");
  const spectrumEligible = Boolean(SpectrumMonitor && originalSpectrumTarget(machine.id) && machine.defaultRuntime === "original");
  const [spectrumBackend, setSpectrumBackend] = useState<"original" | "mame-z80">("original");
  const spectrumSelected = spectrumEligible && (spectrumBackend === "original" || kind === "assembly");
  const builtInSelected = originalSelected || spectrumSelected || c64Selected || vectrexSelected;
  const [developmentDraft, setDevelopmentDraft] = useState<DevelopmentSource>();
  const [development, setDevelopment] = useState<ReturnType<DevelopmentDebugger>>();
  const developmentRef = useRef<ReturnType<DevelopmentDebugger> | undefined>(undefined);
  const [debugState, setDebugState] = useState<OriginalHostReply>();
  const [debugBusy, setDebugBusy] = useState(false);
  const [breakpointLine, setBreakpointLine] = useState<number | null>(null);
  const [machineBreakpoint, setMachineBreakpoint] = useState<number | null>(null);
  const [machineSourceLine, setMachineSourceLine] = useState<number | null>(null);
  const [machineSourceUri, setMachineSourceUri] = useState<string>();
  const [rawAddress, setRawAddress] = useState("28672");
  const [rawEntry, setRawEntry] = useState("28672");
  const [binaryFormat, setBinaryFormat] = useState<"raw" | "decb" | "cartridge">("raw");
  const revision = useRef(1);
  const recoveryProjectId = useRef<string | undefined>(undefined);
  const sequence = useRef(0);
  const runSequence = useRef(0);
  const work = useRef<AbortController | undefined>(undefined);
  const firmware = useRef<Uint8Array | undefined>(undefined);
  const firmwareTarget = useRef<string | undefined>(undefined);
  const editor = useRef<HTMLTextAreaElement>(null);
  const active = useRef(false);
  active.current = Boolean(running) || Boolean(monitorOpen) || Boolean(development) || Boolean(originalSession) || busy;
  const closeDialog = useCallback(() => { work.current?.abort(); sequence.current++; setBusy(false); setDialog(undefined); }, []);
  const reveal = (next = "output") => { setPanel(next); setPanelOpen(true); };

  const stop = useCallback(() => { spectrumLaunch.current=undefined;spectrumPort.current=undefined;spectrumPrevious.current="";setSpectrumStatus(undefined); originalSequence.current++; const normal = originalRef.current; originalRef.current = undefined; normal?.dispose(); setOriginalSession(undefined); if (normal) setMessage(`Built-in ${originalMachine.current.startsWith("dragon64") ? "Dragon 64" : "Dragon 32"} session ended. Play or Debug starts a fresh session.`); monitorRelease.current?.(); monitorRelease.current = undefined; developmentRef.current?.connection.dispose(); developmentRef.current = undefined; setDevelopment(undefined); setDevelopmentDraft(undefined); setDebugState(undefined); setDebugBusy(false); setPanel(current => current === "debug" ? "output" : current); setRunning(undefined); setMonitorOpen(undefined); setMonitorCassette(undefined); }, []);
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
      const normal = originalRef.current; originalRef.current = undefined; normal?.dispose();
      developmentRef.current?.connection.dispose(); developmentRef.current = undefined;
      work.current?.abort(); sequence.current++; firmware.current?.fill(0); firmware.current = undefined;
    };
  }, [stop]);
  useEffect(() => {
    // Restore the retained project owner when React restarts effects after a refresh.
    const id = project?.id;
    recoveryProjectId.current = id;
    return () => { if (recoveryProjectId.current === id) recoveryProjectId.current = undefined; };
  }, [project?.id]);
  useEffect(() => {
    const protect = (event: BeforeUnloadEvent) => { if (dirty && revision.current > 1) event.preventDefault(); };
    window.addEventListener("beforeunload", protect);
    return () => window.removeEventListener("beforeunload", protect);
  }, [dirty]);

  const filename = project?.entryPath ?? (kind === "basic" ? "game.bas" : "recovered.asm");
  const activeSourcePath = selectedFile && sourcePaths.includes(selectedFile) ? selectedFile : filename;
  const activeSource = activeSourcePath === filename ? source : project?.project.supportFiles[activeSourcePath] ?? "";
  const sourceSelected = selectedFile === undefined || sourcePaths.includes(selectedFile);
  function selectFile(path?: string) { setSelectedFile(path); setMachineSourceLine(null); }
  useEffect(() => {
    if (!diagnosticLocation || !hasDocument || diagnosticLocation.revision !== revision.current) return;
    if (!sourceSelected || diagnosticLocation.sourceUri !== activeSourcePath) return;
    // Select only after React has mounted the diagnostic's source document.
    const lines = activeSource.split("\n");
    const row = Math.min(diagnosticLocation.row - 1, lines.length - 1);
    const offset = lines.slice(0, row).reduce((total, line) => total + line.length + 1, 0) + Math.min(diagnosticLocation.column - 1, lines[row]?.length ?? 0);
    editor.current?.focus(); editor.current?.setSelectionRange(offset, offset);
    setDiagnosticLocation(undefined);
  }, [diagnosticLocation, hasDocument, sourceSelected, activeSourcePath, activeSource]);
  useEffect(() => {
    if ((!developmentRef.current && !originalRef.current) || debugState?.status.state !== "stopped" || !debugState.status.location) return;
    // A stop may replace a project-settings tab; select only after the source mounts.
    editor.current?.focus();
    if (isMachineInspection(debugState)) {
      const line = debugState.status.location?.sourceLine;
      if (!line || debugState.status.location?.sourceUri !== activeSourcePath) return;
      const rows = activeSource.split("\n");
      const start = rows.slice(0, line - 1).reduce((offset, row) => offset + row.length + 1, 0);
      editor.current?.setSelectionRange(start, start + (rows[line - 1]?.length ?? 0));
    } else if (debugState.status.location) editor.current?.setSelectionRange(debugState.status.location.sourceStart, debugState.status.location.sourceEnd);
  }, [debugState, activeSource, activeSourcePath]);
  useEffect(()=>{
    const line=spectrumStatus?.valuesValid?spectrumStatus.sourceLine:null;
    if(!line||!spectrumLaunch.current||activeSourcePath!==filename)return;
    const rows=activeSource.split("\n");const start=rows.slice(0,line-1).reduce((n,row)=>n+row.length+1,0);
    editor.current?.focus();editor.current?.setSelectionRange(start,start+(rows[line-1]?.length??0));
  },[spectrumStatus,activeSource,activeSourcePath,filename]);

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
    setBusy(false); setBuilt(undefined); setMachineMedia(undefined); setMachineBreakpoint(null); setMachineSourceLine(null); setDiagnostics([]); stop(); setSource(next); setKind(nextKind); setDirty(true); setHasDocument(true);
  }
  function editSource(next: string) {
    if (activeSourcePath === filename) { edit(next); return; }
    if (!project || !sourcePaths.includes(activeSourcePath)) return;
    if (new TextEncoder().encode(next).length > MAX_SOURCE_BYTES) { setMessage("Source exceeds the 1 MiB browser limit."); return; }
    work.current?.abort(); sequence.current++; revision.current++;
    setBusy(false); setBuilt(undefined); setMachineMedia(undefined); setMachineBreakpoint(null); setMachineSourceLine(null); setDiagnostics([]); stop(); setDirty(true);
    setProject({ ...project, project: { ...project.project, supportFiles: { ...project.project.supportFiles, [activeSourcePath]: next } } });
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
    if (previous?.id === project.id && previous.project.source === source && previous.project.kind === kind && previous.project.supportFiles === project.project.supportFiles && previous.cloud?.sha === project.cloud?.sha && previous.cloud?.repository === project.cloud?.repository) return;
    setDirty(true); setStorageState("Saving on this device…");
    const snapshot = { ...project.project, source, kind };
    const timer = window.setTimeout(() => persist(snapshot, project), 350);
    return () => window.clearTimeout(timer);
  }, [project?.id, project?.cloud?.repository, project?.cloud?.sha, project?.project.supportFiles, source, kind]);
  function canReplace() {
    return !dirty || window.confirm("The latest changes have not recovered locally. Download a project copy before switching, or continue to discard those changes.");
  }
  function closeProject() {
    if (!canReplace()) return;
    stop(); work.current?.abort(); sequence.current++; revision.current++;
    setBusy(false); setBuilt(undefined); setMachineMedia(undefined);
    setMachineBreakpoint(null); setMachineSourceLine(null);
    setProject(undefined); localCopy.current = undefined; recoveryProjectId.current = undefined;
    setHasDocument(false); setDirty(false); setPreview(undefined); setSelectedFile(undefined); setPanelOpen(false);
  }
  function installProject(next: ReturnType<typeof projectResult>, saved?: StoredProject, cloud?: CloudLink) {
    const nextTarget = projectTarget(next.project);
    if (!machines.some((item) => item.id === nextTarget)) throw new Error("This computer is unsupported in the browser edition.");
    if (nextTarget !== firmwareTarget.current) { firmware.current?.fill(0); firmware.current = undefined; firmwareTarget.current = undefined; setFirmwareReady(false); }
    setNewTarget(nextTarget); setRecoveryTarget(nextTarget);
    const entry: StoredProject = saved ?? { schemaVersion: 1, id: projectId(), version: "", updatedAt: Date.now(), ...next, ...(cloud ? { cloud } : {}) };
    recoveryProjectId.current = entry.id; localCopy.current = saved; setProject(entry); edit(next.project.source, next.project.kind);
    setSourcePaths(next.sourcePaths); setSelectedFile(undefined); setSampleHint(false); setShowHelp(false); setPreview(undefined); setDialog(undefined); setPanelOpen(false); setSidebarOpen(window.innerWidth > 700);
    setMessage("Project open. Choose Check & build when you are ready to try your game.");
    persist(next.project, entry);
    window.requestAnimationFrame(() => editor.current?.focus());
  }
  function createProject(name = newName, listing = template === "sample" ? newMachine.sample : "", language = kind, target = newTarget, assembly?: { loadAddress: number; entrySymbol: string }) {
    if (!canReplace()) return;
    void operate({ schemaVersion: 1, operation: "create-project", name, source: listing, kind: language, target, ...(assembly ? { assembly } : {}) }, NO_BYTES, (value) => { installProject(projectResult(value)); setSampleHint(listing === SAMPLE_SOURCE); });
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
  function savedRemote(cloud: CloudLink, savedSnapshot: ProjectSnapshot) {
    if (!project) return;
    const next = { ...project, cloud }; setProject(next);
    persist({ ...project.project, source, kind }, next);
    setMessage(JSON.stringify(savedSnapshot) === JSON.stringify(currentSnapshot) ? "Project saved to GitHub. Later changes stay local until you save again." : "The selected snapshot was saved to GitHub. Your newer changes still need a GitHub save.");
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
    setBusy(true); setDiagnostics([]); reveal(); setMessage(request.operation === "cassette-audio" ? "Preparing cassette loading audio…" : request.operation === "firmware" ? `Checking your ${machine.label} system ROM…` : request.operation === "decompile" ? "Recovering a source preview…" : request.operation !== "test-project" && request.operation.includes("project") ? "Preparing your project locally…" : "Checking your code and preparing the game file…");
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
    setBuilt(undefined); setMachineMedia(undefined);
    if (!currentSnapshot) return;
    void operate({ schemaVersion: 1, operation: "test-project", project: currentSnapshot }, NO_BYTES, (value, atRevision) => {
      const artefact = buildResult(value);
      if (!artefact.roundTripVerified && !(machine.id === "vectrex" && artefact.cartridgeVerified === true)) throw new Error("The game file check did not complete. Choose Check & build to try again.");
      setBuilt({ revision: atRevision, result: artefact });
      if (artefact.assembly) setMachineBreakpoint(artefact.assembly.entryAddress);
      setMessage(machine.id === "vectrex" ? "Cartridge assembled and header validated. Choose Play to run the original development runtime." : "Game file ready. Code and programme file checks passed. Choose Play to try it, or Save work to keep a copy.");
    });
  }
  function downloadCassetteAudio() {
    if (!originalDragonTarget(machine.id) || !built || built.revision !== revision.current || busy) return;
    closeDialog();
    void operate({ schemaVersion: 1, operation: "cassette-audio", target: machine.id, cassetteSha256: built.result.cassetteSha256 }, built.result.cassette, (value) => {
      download(new Uint8Array(cassetteAudio(value)), "game.wav", "audio/wav");
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
  function runC64Original() {
    if (!C64Monitor || machine.id !== "c64-pal" || kind !== "basic" || !built || built.revision !== revision.current || !built.result.sourceSha256 || busy || running || monitorOpen || developmentRef.current || originalRef.current) return;
    setC64Source({schemaVersion:1,target:"c64-pal",source,sourceSha256:built.result.sourceSha256,prgSha256:built.result.cassetteSha256});
    setMonitorOpen("c64-original"); closeDialog(); reveal("emulator");
  }
  function run() {
    if (vectrexSelected) {
      if (!built || built.revision !== revision.current || !built.result.sourceSha256 || busy || running || monitorOpen) return;
      setVectrexSource({schemaVersion:1,target:"vectrex",source,sourceSha256:built.result.sourceSha256,cartridgeSha256:built.result.cassetteSha256});
      setMonitorOpen("vectrex-original");closeDialog();reveal("emulator");return;
    }
    if (c64Selected) { runC64Original(); return; }
    if (spectrumSelected) { runSpectrumDevelopment(); return; }
    if (originalSelected) { void originalAction(); return; }
    if (!built || built.revision !== revision.current || busy || running || monitorOpen || development || originalSession || kind !== "basic") return;
    if (!firmware.current || firmwareTarget.current !== machine.id) {
      reveal("emulator"); setMessage(`Choose your ${machine.label} system ROM to play. Your game file is ready.`); setDialog("rom"); return;
    }
    setDevelopmentDraft(undefined); setDebugState(undefined);
    reveal("emulator"); setMessage(`Starting your virtual ${machine.label}…`);
    setRunning({ machine, session: ++runSequence.current, firmware: firmware.current.slice(), media: built.result.cassette.slice() });
  }
  async function originalAction(debug = false, resume = false) {
    const currentMedia = machineMedia?.revision === revision.current ? machineMedia.media : undefined;
    const basicBuild = kind === "basic" && built?.revision === revision.current ? built.result : undefined;
    const assemblyBuild = kind === "assembly" && built?.revision === revision.current && built.result.assembly ? built.result : undefined;
    if (!hasDocument || !OriginalRuntime || !originalProfile || !originalSelected || (!currentMedia && !basicBuild?.sourceSha256 && !assemblyBuild) || busy || debugBusy || running || monitorOpen || developmentRef.current || developmentDraft || (debug && (kind === "basic" ? !breakpointLine : machineBreakpoint === null && machineSourceLine === null))) return;
    const input: OriginalRuntimeInput = assemblyBuild && currentSnapshot
      ? { kind: "assembly-source", schemaVersion: 1, project: currentSnapshot, buildId: assemblyBuild.assembly!.buildId, cassetteSha256: assemblyBuild.cassetteSha256, ...(debug ? { debug: machineSourceLine !== null ? { kind: "source" as const, sourceLine: machineSourceLine, sourceUri: machineSourceUri ?? activeSourcePath } : { kind: "address" as const, address: machineBreakpoint! } } : {}) }
      : currentMedia ? { ...currentMedia, ...(debug ? { breakpointAddress: machineBreakpoint! } : {}) }
      : { schemaVersion: 1, source, sourceSha256: basicBuild!.sourceSha256!, cassetteSha256: basicBuild!.cassetteSha256, ...(debug ? { breakpointLine: breakpointLine! } : {}) };
    if (originalRef.current && !debug) return;
    const atRevision = revision.current;
    const attempt = ++originalSequence.current;
    setDebugBusy(true);
    try {
      if (resume && originalRef.current?.connection) { await originalRef.current.connection.debug("continue"); return; }
      const previous = originalRef.current; originalRef.current = undefined; previous?.dispose(); setDebugState(undefined);
      let owned: ReturnType<OriginalRuntime>;
      let previousStatus = "";
      owned = OriginalRuntime(input, reply => {
        if (originalRef.current !== owned || revision.current !== atRevision) return;
        const key = JSON.stringify(reply.status); if (key === previousStatus) return;
        previousStatus = key; setDebugState(reply);
        if (reply.status.state === "stopped") { reveal("debug"); if (reply.status.location) setSelectedFile(isMachineInspection(reply) && reply.status.location.sourceUri !== filename && sourcePaths.includes(reply.status.location.sourceUri) ? reply.status.location.sourceUri : undefined); }
      }, () => {
        if (originalRef.current !== owned || revision.current !== atRevision) return;
        originalRef.current = undefined; setOriginalSession(undefined); setDebugState(undefined); setDebugBusy(false);
        setMessage(`Built-in ${machine.id.startsWith("dragon64") ? "Dragon 64" : "Dragon 32"} session ended. Play or Debug starts a fresh session.`);
      }, originalProfile);
      originalRef.current = owned;
      reveal("emulator"); setMessage(currentMedia ? "Original Dragon 32 is running the selected machine-code file." : assemblyBuild ? "Original Dragon 32 is using the current assembly build." : "Original BASIC is using the current build.");
      await owned.transport({ operation: "state" });
      if (originalRef.current !== owned || revision.current !== atRevision) { owned.dispose(); return; }
      setOriginalSession(owned);
    } catch (error) {
      if (revision.current === atRevision && originalSequence.current === attempt) {
        const owned = originalRef.current; originalRef.current = undefined; owned?.dispose(); setOriginalSession(undefined); setDebugState(undefined);
        setMessage(error instanceof Error ? error.message : "Original BASIC could not start. Play retries the current build."); reveal();
      }
    } finally { if (revision.current === atRevision && originalSequence.current === attempt) setDebugBusy(false); }
  }
  async function machineDebugAction(operation: "step" | "continue" | "pause") {
    const owned = originalRef.current;
    if (!owned?.connection || busy || debugBusy) return;
    const atRevision = revision.current;
    setDebugBusy(true);
    try { await owned.connection.debug(operation); }
    catch (error) { if (originalRef.current === owned && revision.current === atRevision) setMessage(error instanceof Error ? error.message : "Machine debugging failed."); }
    finally { if (originalRef.current === owned && revision.current === atRevision) setDebugBusy(false); }
  }
  function startOriginal(firmware: MonitorFirmware) {
    if (!(firmware === "spectrum-development" ? SpectrumMonitor && originalSpectrumTarget(machine.id) : Monitor && machine.id === "dragon32") || busy || running || monitorOpen || developmentDraft || developmentRef.current || originalRef.current) return;
    setSpectrumSource(undefined);spectrumLaunch.current=undefined;
    setMonitorOpen(firmware); closeDialog(); reveal("emulator");
  }
  function runSpectrumDevelopment(debug = false) {
    if (!SpectrumMonitor || !originalSpectrumTarget(machine.id) || !built || built.revision !== revision.current || !built.result.sourceSha256 || busy || running || monitorOpen || developmentDraft || developmentRef.current || originalRef.current) return;
    if(debug && !machineSourceLine)return;
    const launch:SpectrumSourceLaunch={schemaVersion:1,target:machine.id,kind,source,sourcePath:filename,revision:built.result.assembly?.buildId ?? built.result.sourceSha256,sourceSha256:built.result.sourceSha256,cassetteSha256:built.result.cassetteSha256,...(debug?{breakpointLine:machineSourceLine!}:{})};
    spectrumLaunch.current=launch;spectrumPrevious.current="";setSpectrumStatus(undefined);setSpectrumSource(launch);
    setMonitorOpen("spectrum-development"); closeDialog(); reveal("emulator");
  }
  function mountDevelopmentBuild() {
    if (!Monitor || machine.id !== "dragon32" || !built || built.revision !== revision.current || kind !== "basic" || busy || running || monitorOpen || developmentDraft || developmentRef.current || originalRef.current) return;
    if (DevelopmentDebugger && built.result.sourceSha256) {
      setDevelopmentDraft({ schemaVersion: 1, source, sourceSha256: built.result.sourceSha256, cassetteSha256: built.result.cassetteSha256, breakpointLine: 1 });
    }
    setMonitorCassette({ bytes: built.result.cassette.slice(), sha256: built.result.cassetteSha256 }); setMonitorOpen("basic-development"); closeDialog(); reveal("emulator");
    setMessage("Development cassette mounted. Use CLOAD then RUN in Game, or choose a breakpoint in Debug to prepare a fresh debug session. Normal Play continues to use XRoar.");
  }
  async function developmentAction(action: "start" | "continue") {
    if (busy || running || (monitorOpen && monitorOpen !== "basic-development") || debugBusy || !developmentDraft || !breakpointLine) return;
    setDebugBusy(true);
    const atRevision = revision.current;
    try {
      if (!developmentRef.current) {
        if (!DevelopmentDebugger || built?.revision !== atRevision) throw new Error("Build the current source before debugging.");
        monitorRelease.current?.(); monitorRelease.current = undefined; setMonitorOpen(undefined); setMonitorCassette(undefined);
        let owned: ReturnType<DevelopmentDebugger>;
        let previousStatus = "";
        owned = DevelopmentDebugger({ ...developmentDraft, breakpointLine }, reply => {
          if (developmentRef.current !== owned || revision.current !== atRevision) return;
          const statusKey = JSON.stringify(reply.status);
          if (statusKey === previousStatus) return;
          previousStatus = statusKey; setDebugState(reply);
          if (reply.status.state === "stopped") {
            reveal("debug");
            if (reply.status.location) setSelectedFile(isMachineInspection(reply) && reply.status.location.sourceUri !== filename && sourcePaths.includes(reply.status.location.sourceUri) ? reply.status.location.sourceUri : undefined);
          }
        }, () => {
          if (developmentRef.current !== owned || revision.current !== atRevision) return;
          developmentRef.current = undefined; setDevelopment(undefined); setDebugState(undefined); setDebugBusy(false);
          setMessage("Development session ended. Choose Debug to mount this build again, or change the source and rebuild.");
        });
        developmentRef.current = owned; setDevelopment(owned);
        await owned.connection.monitor({ operation: "state" });
        if (developmentRef.current !== owned || revision.current !== atRevision) { owned.connection.dispose(); return; }
        reveal("emulator"); setMessage("Development cassette mounted. Enter CLOAD in Game; when the prompt returns, choose Debug to run to the selected BASIC line.");
      } else {
        const owned = developmentRef.current;
        if (action === "start") await owned.connection.start(); else await owned.connection.debug("continue");
        if (developmentRef.current === owned && revision.current === atRevision) reveal("debug");
      }
    } catch (error) { if (revision.current === atRevision) { setMessage(error instanceof Error ? error.message : "Development debug failed."); reveal("debug"); } }
    finally { if (revision.current === atRevision) setDebugBusy(false); }
  }
  async function spectrumDebugAction(operation:"step"|"continue"|"pause") {
    const port=spectrumPort.current;const launch=spectrumLaunch.current;
    if(!port||!launch||debugBusy||operation!=="pause"&&!spectrumStatus?.valuesValid)return;
    setDebugBusy(true);
    try {await port.command(operation,spectrumStatus?.stopId??0);}
    catch(error){if(spectrumLaunch.current===launch)setMessage(error instanceof Error?error.message:"Spectrum debugger failed.");}
    finally{if(spectrumLaunch.current===launch)setDebugBusy(false);}
  }
  function command(id: EditorCommandId) {
    if (id === "save" && hasDocument) setDialog("save");
    if (id === "build" && hasDocument && (kind === "basic" || machine.id === "dragon32" || machine.id === "vectrex" || originalSpectrumTarget(machine.id)) && !busy && !running && !monitorOpen && !development && !originalSession) build();
    if (id === "run") run();
    if (id === "debug") { if (spectrumDebugAvailable) runSpectrumDevelopment(true); else if (originalSelected) void originalAction(true); else if (developmentDraft) void developmentAction("start"); }
    if (id === "stop") { const normal = Boolean(originalRef.current); stop(); reveal(); if (!normal) setMessage(monitorOpen === "c64-original" ? "Original C64 session stopped." : monitorOpen ? "Original monitor stopped." : "Game stopped. Your code and game file are ready to use again."); }
  }
  const spectrumDebugAvailable=Boolean(SpectrumMonitor && originalSpectrumTarget(machine.id) && (!spectrumEligible || spectrumSelected) && hasDocument && built?.revision===revision.current && built.result.sourceSha256);
  const machineDebugAvailable = Boolean(hasDocument && originalSelected && kind === "assembly" && (built?.revision === revision.current && built.result.assembly || machineMedia?.revision === revision.current));
  const basicDebugAvailable = Boolean(hasDocument && (developmentDraft || originalSelected && kind === "basic" && built?.revision === revision.current));
  const machineStatus = debugState && isMachineInspection(debugState) ? debugState.status : undefined;
  const basicStatus = debugState && !isMachineInspection(debugState) ? debugState.status : undefined;
  const availability = {
    save: hasDocument,
    build: hasDocument && (kind === "basic" || machine.id === "dragon32" || machine.id === "vectrex" || originalSpectrumTarget(machine.id)) && !debugBusy && !busy && !running && !monitorOpen && !development && !originalSession,
    run: hasDocument && ((built?.revision === revision.current && (kind === "basic" || builtInSelected && Boolean(built.result.assembly))) || (originalSelected && machineMedia?.revision === revision.current)) && !debugBusy && !busy && !running && !monitorOpen && !development && !originalSession,
    stop: Boolean(running) || Boolean(originalSession) || Boolean(monitorOpen) || Boolean(developmentDraft) || (originalSelected && debugBusy), debug: Boolean((spectrumDebugAvailable ? machineSourceLine!==null : machineDebugAvailable ? machineBreakpoint !== null || machineSourceLine !== null : basicDebugAvailable && breakpointLine) && !busy && !running && (!monitorOpen || (developmentDraft && monitorOpen === "basic-development")) && !debugBusy && ((!development && !originalSession) || debugState?.status.state === "ready")),
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
      if (importedKind === "assembly" && recoveryTarget !== "dragon32" && recoveryTarget !== "vectrex" && !originalSpectrumTarget(recoveryTarget)) throw new Error("This browser edition supports BASIC for this computer. Open assembly source in the desktop edition.");
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
      const input: MediaInput | { format: "cartridge" } = raw ? binaryFormat === "raw" ? { format: "raw", loadAddress: Number(rawAddress), executionAddress: Number(rawEntry) } : { format: binaryFormat } : { format: "cassette" };
      void operate({ schemaVersion: 1, operation: "decompile", target: raw ? "dragon32" : recoveryTarget, input }, bytes, (value) => {
        const recovered = decompiledSource(value);
        setPreview({ ...recovered, ...(recovered.machineCode ? { originalMedia: bytes.slice() } : {}), target: raw ? "dragon32" : recoveryTarget }); setPanel("import");
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
    const selected = preview;
    void operate({ schemaVersion: 1, operation: "create-project", name: selected.kind === "basic" ? "Imported game" : "Recovered code", source: selected.source, kind: selected.kind, target: selected.target ?? machine.id, ...(selected.assembly ? { assembly: { loadAddress: selected.assembly.loadAddress, entrySymbol: "START" } } : {}) }, NO_BYTES, value => {
      installProject(projectResult(value));
      if (selected.machineCode && selected.originalMedia) {
        setMachineMedia({ revision: revision.current, media: { kind: "machine-code", identity: selected.machineCode, bytes: selected.originalMedia.slice() } });
        setMachineBreakpoint(selected.assembly?.entryAddress ?? null);
        setMessage("Selected machine-code file ready. Play uses the original bytes; editing this listing requires a new build.");
      }
    });
  }
  useEffect(() => {
    const shortcut = (event: KeyboardEvent) => {
      if (event.key === "F5" && !event.metaKey && !event.ctrlKey && !event.altKey && !event.shiftKey && (basicDebugAvailable || machineDebugAvailable)) {
        event.preventDefault();
        if (!event.repeat && !dialog && availability.debug) command("debug");
        return;
      }
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
  const nextStep = !hasDocument ? "Try a sample game or open your work to get started." : kind === "assembly" ? machineMedia?.revision === revision.current ? "Selected machine-code file ready. Play runs its original bytes; editing requires a new build." : built?.revision === revision.current ? "Assembly build ready. Choose Play to run it." : "Choose Check & build to assemble this source, then Play." : running ? "Your game is running. Stop it before checking another build." : busy ? "Working locally on your device. You can cancel in Results." : !built ? "Choose Check & build before playing your code." : !firmwareReady && !builtInSelected ? `Game file ready. Choose Play to set up your virtual ${machine.label}.` : "Game file ready. Choose Play, or Save work to keep a copy.";
  function goToDiagnostic(item: BrowserDiagnostic) {
    const sourceUri = item.sourceUri ?? filename;
    if (sourceUri !== filename && !sourcePaths.includes(sourceUri)) return;
    selectFile(sourceUri === filename ? undefined : sourceUri);
    setDiagnosticLocation({ sourceUri, row: item.row, column: item.column, revision: revision.current });
  }
  const developmentBuild = Monitor && machine.id === "dragon32" && built?.revision === revision.current && kind === "basic" ? <button type="button" disabled={busy || Boolean(running) || Boolean(originalSession) || Boolean(monitorOpen) || Boolean(developmentDraft)} onClick={mountDevelopmentBuild}>Mount built cassette in original BASIC (development)</button> : null;
  const dragonSelection = originalEligible && <label>Emulator<select aria-label="Project emulator" value={originalSelected ? "original" : projectBackend} disabled={busy || debugBusy || Boolean(running) || Boolean(monitorOpen) || Boolean(developmentDraft) || Boolean(originalSession)} onChange={event => setProjectBackend(event.target.value === "original" ? "original" : "xroar")}>{machine.id === "dragon32" && <option value="xroar" disabled={kind === "assembly"}>XRoar</option>}<option value="original">{originalProfile ? originalRuntimeLabel(originalProfile) : "Original BASIC"}</option></select></label>;
  const c64Selection = C64Monitor && machine.id === "c64-pal" && <label>Emulator<select aria-label="Project emulator" value={c64Backend} disabled={busy || Boolean(running) || Boolean(monitorOpen)} onChange={event=>setC64Backend(event.target.value === "original" ? "original" : "chips-c64")}><option value="chips-c64">chips · supplied ROM</option><option value="original">Original C64 (development)</option></select></label>;
  const emulatorSelection = (vectrexSelected ? <p>Original Vectrex development executive · built-in assembler, vectors, sound and controls. Compatibility qualification is incomplete.</p> : null) || c64Selection || (spectrumEligible ? <label>Emulator<select aria-label="Project emulator" value={spectrumSelected ? "original" : spectrumBackend} disabled={busy || debugBusy || Boolean(running) || Boolean(monitorOpen) || Boolean(developmentDraft) || Boolean(originalSession)} onChange={event => setSpectrumBackend(event.target.value === "mame-z80" ? "mame-z80" : "original")}><option value="original">Original {originalSpectrumTarget(machine.id) ? spectrumModelName(machine.id) : "Spectrum"}</option><option value="mame-z80" disabled={kind === "assembly"}>MAME · supplied ROM</option></select></label> : dragonSelection);
  const spectrumDevelopmentControls = SpectrumMonitor && originalSpectrumTarget(machine.id) && !monitorOpen && <><p>{c64Selected ? "Original C64 uses its bundled development firmware. No system ROM is needed; chips remains available with supplied firmware." : spectrumSelected ? "Original Spectrum uses its bundled firmware. BASIC and assembly run without setup; compatibility qualification is in progress." : "Original Spectrum is available for development without a system ROM. MAME remains the compatibility option below."}</p><button type="button" disabled={busy || Boolean(running) || Boolean(originalSession) || Boolean(developmentDraft)} onClick={() => startOriginal("spectrum-development")}>Start original Spectrum (development)</button>{!spectrumSelected && built?.revision === revision.current && built.result.sourceSha256 && <button type="button" disabled={busy || Boolean(running) || Boolean(originalSession) || Boolean(developmentDraft)} onClick={()=>runSpectrumDevelopment()}>Run current build in original Spectrum (development)</button>}</>;
  const romSetup = <div className="browser-rom-setup">
    {spectrumDevelopmentControls}
    {c64Selected && <p>Original C64 uses bundled development firmware. Check &amp; build, then Play. Use chips with your supplied ROM for the retained compatibility route.</p>}
    {emulatorSelection}
    {originalSelected && <p>{originalProfile && originalRuntimeLabel(originalProfile)} uses its bundled firmware. Choose Play after checking the current source or selecting a machine-code file. Cartridges are unsupported on the website; use the desktop edition with XRoar.</p>}
    {Monitor && machine.id === "dragon32" && !monitorOpen && <><p>{originalEligible ? "Original firmware needs no system ROM. XRoar uses the system ROM you choose below." : "Original firmware needs no system ROM. Original BASIC is in development; BASIC games can use XRoar below."}</p><button type="button" disabled={busy || Boolean(running) || Boolean(originalSession) || Boolean(monitorOpen) || Boolean(developmentDraft)} onClick={() => startOriginal("monitor")}>Start original monitor</button><button type="button" disabled={busy || Boolean(running) || Boolean(originalSession) || Boolean(monitorOpen) || Boolean(developmentDraft)} onClick={() => startOriginal("basic-development")}>Start original BASIC (development)</button></>}
    {!monitorOpen && developmentBuild}
    {!builtInSelected && <>{dialog !== "rom" && <h2>{firmwareReady ? `Your virtual ${machine.label} is ready` : `Set up your virtual ${machine.label}`}</h2>}
    <p>{firmwareReady ? "System ROM checked. Choose Play to start your game." : `To play, choose your ${machine.label} system ROM: the original computer’s built-in software.`}</p>
    {!firmwareReady && romPicker}
    <p className="browser-muted">{machine.romNames.join(" + ")} · {machine.romSize / 1024} KiB total. Select the named files together, or one combined file in that order.</p>
    <p className="browser-muted">Your ROM stays on this device, only while this page is open. You can edit, check and save code without it.</p></>}
  </div>;
  const sidebar = <aside id="primary-sidebar" className="sidebar" aria-label={view === "explorer" ? "Explorer" : "Computer setup"}>
    <div className="sidebar-heading">{view === "explorer" ? "EXPLORER" : "COMPUTER SETUP"}</div>
    <div className="browser-sidebar-content">
      <strong>Eightbit Studio</strong><span className="browser-muted">{machine.label} · Browser edition</span>
      {view === "explorer" ? <>
        {hasDocument && <strong>{project?.project.name}</strong>}
        {hasDocument && <button type="button" onClick={() => { selectFile(undefined); window.requestAnimationFrame(() => editor.current?.focus()); }}><FilesIcon />{filename}</button>}
        {hasDocument && project?.project.manifest && <button type="button" onClick={() => selectFile(machine.manifest)}><FilesIcon />{machine.manifest}</button>}
        {hasDocument && Object.keys(project?.project.supportFiles ?? {}).map((path) => <button type="button" key={path} onClick={() => selectFile(path)}><FilesIcon />{path}</button>)}
        <button type="button" disabled={busy || Boolean(running)} onClick={() => setDialog("open")}><FilesIcon />Load existing project…</button>
        <button type="button" disabled={busy || Boolean(running)} onClick={() => { setNewName("My next game"); setDialog("create"); }}>Create new project…</button>
        {hasDocument && <button type="button" onClick={closeProject}>Project home</button>}
        {hasDocument && <button type="button" onClick={() => setShowHelp((value) => !value)} aria-expanded={showHelp}>Getting started help</button>}
        <button type="button" disabled={busy || Boolean(running)} onClick={() => setDialog("recover")}>Recover source…</button>
        <details hidden={machine.cpu !== "6809" || machine.id === "vectrex"} className="browser-advanced"><summary>Advanced import</summary><p>Raw machine-code files need a known memory address: where the computer loads and starts the code. BASIC editing does not need this.</p>
          <label>File format<select aria-label="Machine-code file format" value={binaryFormat} onChange={event => setBinaryFormat(event.target.value === "decb" ? "decb" : event.target.value === "cartridge" ? "cartridge" : "raw")}><option value="raw">Raw binary</option><option value="decb">DECB binary</option><option value="cartridge">Cartridge (unsupported on website)</option></select></label>
          {binaryFormat === "raw" && <><label>Load address (decimal)<input aria-label="Raw load address" type="number" min="0" max="65535" value={rawAddress} disabled={busy || Boolean(running)} onChange={(event) => setRawAddress(event.currentTarget.value)} /></label><label>Start address (decimal)<input aria-label="Raw entry address" type="number" min="0" max="65535" value={rawEntry} disabled={busy || Boolean(running)} onChange={(event) => setRawEntry(event.currentTarget.value)} /></label></>}
          {fileControl("Choose raw binary…", ".bin,.rom", (file) => { void decompile(file, true); })}
        </details>
      </> : <>
        <section className="browser-settings-section" aria-labelledby="browser-rom-heading">
          <h2 id="browser-rom-heading">System ROM</h2>
          {emulatorSelection}
          {Monitor && machine.id === "dragon32" && <button type="button" disabled={busy || Boolean(running) || Boolean(originalSession) || Boolean(monitorOpen) || Boolean(developmentDraft)} onClick={() => startOriginal("monitor")}>Start original monitor</button>}
          {Monitor && machine.id === "dragon32" && <button type="button" disabled={busy || Boolean(running) || Boolean(originalSession) || Boolean(monitorOpen) || Boolean(developmentDraft)} onClick={() => startOriginal("basic-development")}>Start original BASIC (development)</button>}
          {developmentBuild}
          {spectrumDevelopmentControls}
          <p>{c64Selected ? "Original C64 uses its bundled development firmware. No system ROM is needed; chips remains available with supplied firmware." : spectrumSelected ? "Original Spectrum uses its bundled firmware. No system ROM or external assembler is needed. Choose MAME for the separate supplied-ROM option." : originalSelected ? "Original BASIC uses its bundled firmware. Cartridges are unsupported on the website; use the desktop edition with XRoar." : "The computer’s built-in software. Add your own to play; editing and saving work without it."}</p>
          {!spectrumSelected && !c64Selected && !vectrexSelected && romPicker}
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
    {(built || diagnostics.length > 0) && <details><summary>Technical details</summary>{built && <><p>{built.result.tokenBytes} {built.result.assembly ? machine.id === "vectrex" ? "machine-code bytes · cartridge header validated" : "machine-code bytes · assembly and cassette addresses verified" : `token bytes · ${built.result.roundTripVerified ? "Programme file, tokens and listing round trip verified" : "Built without a round-trip check"}`}</p><p>{machine.id === "vectrex" ? "Cartridge" : "Cassette"} SHA-256: <code>{built.result.cassetteSha256}</code></p></>}{diagnostics.map((item, index) => <p key={index}><code>{item.code}</code> · {item.sourceUri ? `${item.sourceUri} · ` : ""}Line {item.row}, column {item.column}</p>)}</details>}
  </div>;
  const welcome = <ProjectWelcome machines={machines} recent={recent} busy={busy} onCreate={() => { setNewName("My first game"); setDialog("create"); }} onOpen={() => setDialog("open")} onRecent={openRecent} onSettings={() => { setView("settings"); setSidebarOpen(true); }} onGitHub={() => setDialog("github")} message={storageState.includes("failed") ? storageState : ""} />;
  return <div className="browser-workbench"><WorkbenchShell sidebarOpen={sidebarOpen} sidebar={sidebar}
    activity={<><button type="button" className={view === "explorer" && sidebarOpen ? "active" : ""} aria-label="Explorer" aria-expanded={view === "explorer" && sidebarOpen} onClick={() => { setView("explorer"); setSidebarOpen(view !== "explorer" || !sidebarOpen); }}><FilesIcon /></button><div className="activity-spacer"/><button type="button" aria-label="Computer setup" className={view === "settings" && sidebarOpen ? "active" : ""} aria-expanded={view === "settings" && sidebarOpen} onClick={() => { setView("settings"); setSidebarOpen(view !== "settings" || !sidebarOpen); }}><SettingsIcon /></button></>}
    toolbar={<EditorToolbar enabled={availability} sessionActive={Boolean(running) || Boolean(monitorOpen) || Boolean(development) || Boolean(originalSession) || (originalSelected && debugBusy)} supportsDebug={Boolean(basicDebugAvailable || machineDebugAvailable || spectrumDebugAvailable)} keepSaveVisible labels={{ save: "Save work", build: "Check & build", run: "Play" }} onCommand={command}>{machine.label} / {hasDocument ? project?.project.name ?? filename : "Projects"}</EditorToolbar>}
    editor={<>{hasDocument && <EditorTabs tabs={[{ path: filename, dirty, badge: kind === "basic" ? "B" : "A", badgeClass: "basic" }, ...(selectedFile ? [{ path: selectedFile, dirty: dirty && sourcePaths.includes(selectedFile), badge: "", badgeClass: "" }] : [])]} activePath={selectedFile ?? filename} onSelect={(path) => selectFile(path === filename ? undefined : path)} onClose={(path) => { if (path !== filename) { selectFile(undefined); return; } closeProject(); }} />}
      {hasDocument && showHelp && <div className="browser-guide" aria-label="Game creation steps"><ol><li aria-current={!built && !running && !busy ? "step" : undefined}>1 Edit</li><li aria-current={busy ? "step" : undefined}>2 Check &amp; build</li><li aria-current={built ? "step" : undefined}>3 Play</li><li>4 Save</li></ol><p>{nextStep}</p></div>}
      {hasDocument ? <>{kind === "basic" && showHelp && <p className="browser-editor-tip">{sampleHint ? "Try changing the winning number on line 40. " : ""}BASIC line numbers set the order of instructions; they are not memory addresses.</p>}{!sourceSelected && selectedFile ? <div className="browser-project-file"><p className="browser-muted">Project guidance and settings are preserved in downloads and GitHub saves. Edit these files in the desktop edition; this browser edits the programme source.</p><textarea aria-label={`Project file ${selectedFile}`} readOnly value={selectedFile === machine.manifest ? project?.project.manifest ?? "" : project?.project.supportFiles[selectedFile] ?? ""} /></div> : <BrowserEditor target={machine.id} label={machine.label} source={activeSource} basic={kind === "basic"} busy={busy} engine={engine} inputRef={editor} onChange={editSource} />}</> : welcome}</>}
    panel={{ open: panelOpen, active: panel, onSelect: (id) => reveal(id), onClose: () => { setPanelOpen(false); stop(); }, label: panel === "debug" ? "Debug" : panel === "emulator" ? "Game" : panel === "import" ? "Source preview" : "Results", tabs: [...((basicDebugAvailable || machineDebugAvailable || spectrumDebugAvailable) ? [{ id: "debug", label: "Debug" }] : []), { id: "output", label: "Results" }, { id: "emulator", label: "Game" }, { id: "import", label: "Source preview" }], content: <>
      {originalSession && <div className="browser-output" hidden={panel !== "emulator"}><MonitorConsole machine={originalProfile?.target ?? "dragon32"} sessionId={originalSession.owner.sessionId} transport={originalSession.transport} onStop={stop} /></div>}
      {development && <div className="browser-output" hidden={panel !== "emulator"}><MonitorConsole sessionId={development.sessionId} transport={development.connection.monitor} onStop={stop} /></div>}
      {panel === "debug" && spectrumDebugAvailable && <MachineDebugPanel cpuLabel="Z80" runtimeMissing="Build the current source" assemblerMissing="Build the current source" addressPlaceholder="$6000" addressBreakpoints={false} byteRegisters={[]} showDisassembly={false} busy={busy||debugBusy} active={monitorOpen==="spectrum-development"&&spectrumSource?.breakpointLine!==undefined} runtime="Original Spectrum (development)" assembler={kind==="basic"?"Built-in BASIC compiler":"Built-in Z80 assembler"} breakpoint={machineSourceLine===null?null:{sourceUri:filename,sourceLine:machineSourceLine}} addressBreakpoint={null} status={spectrumStatus??null} continuing={monitorOpen==="spectrum-development"&&spectrumSource?.breakpointLine!==undefined&&!spectrumStatus} onAddressBreakpoint={()=>{}} onStart={()=>runSpectrumDevelopment(true)} onStep={()=>void spectrumDebugAction("step")} onContinue={()=>void spectrumDebugAction("continue")} onPause={()=>void spectrumDebugAction("pause")} onTerminate={stop} onFocus={()=>reveal("emulator")} focusLabel="Show Game" sourceInput={<label>Source breakpoint line<input aria-label="Spectrum source breakpoint line" type="number" min="1" value={machineSourceLine??""} disabled={busy||debugBusy||Boolean(monitorOpen)} onChange={event=>{const line=Number(event.target.value);setMachineSourceLine(Number.isSafeInteger(line)&&line>0?line:null);}}/></label>}/>}
      {panel === "debug" && machineDebugAvailable && <MachineDebugPanel cpuLabel="6809" runtimeMissing="Build the current assembly source" assemblerMissing="Imported machine-code file" addressPlaceholder="$7000" byteRegisters={["a", "b", "cc", "dp"]} busy={busy || debugBusy} active={Boolean(originalSession)} runtime={originalProfile ? originalRuntimeLabel(originalProfile) : null} assembler="Built-in 6809 assembler" breakpoint={machineSourceLine === null ? null : { sourceUri: machineSourceUri ?? activeSourcePath, sourceLine: machineSourceLine }} addressBreakpoint={machineBreakpoint} status={machineStatus ? { ...machineStatus, registers: machineStatus.registers ? { ...machineStatus.registers } : null } : null} continuing={machineStatus?.state === "running"} onAddressBreakpoint={address => { setMachineBreakpoint(address); setMachineSourceLine(null); }} onStart={() => void originalAction(true)} onStep={() => void machineDebugAction("step")} onContinue={() => void machineDebugAction("continue")} onPause={() => void machineDebugAction("pause")} onTerminate={stop} onFocus={() => reveal("emulator")} focusLabel="Show Game" sourceInput={built?.result.assembly ? <label>Assembly breakpoint line · {activeSourcePath}<input aria-label="Assembly breakpoint line" type="number" min="1" value={machineSourceLine ?? ""} disabled={!sourceSelected || busy || debugBusy || Boolean(originalSession)} onChange={event => { const line = Number(event.target.value); setMachineSourceLine(Number.isSafeInteger(line) && line > 0 ? line : null); setMachineSourceUri(activeSourcePath); }} /></label> : undefined} />}
      {panel === "debug" && basicDebugAvailable && <BasicDebugPanel busy={busy || debugBusy} active={Boolean(development || originalSession)} canStart={(!development && !originalSession) || debugState?.status.state === "ready"} description={originalSelected ? `${message} Original BASIC · each new Debug action builds and loads the current source.` : `${message} Original BASIC development; full compatibility is not yet qualified.`} breakpointLine={breakpointLine} status={basicStatus ?? null} onBreakpointLine={setBreakpointLine} onStart={() => { if (originalSelected) void originalAction(true); else void developmentAction("start"); }} onContinue={() => { if (originalSelected) void originalAction(true, true); else void developmentAction("continue"); }} onTerminate={stop} />}
      {monitorOpen && monitorOpen !== "spectrum-development" && monitorOpen !== "c64-original" && monitorOpen !== "vectrex-original" && Monitor && <div className="browser-output" hidden={panel !== "emulator"}><Monitor key={monitorOpen} firmware={monitorOpen} cassette={monitorCassette?.bytes} cassetteSha256={monitorCassette?.sha256} onRuntime={ownMonitor} onStop={stop} /></div>}
      {monitorOpen === "vectrex-original" && VectrexMonitor && vectrexSource && <div className="browser-output" hidden={panel !== "emulator"}><VectrexMonitor source={vectrexSource} onRuntime={ownMonitor} onStop={stop}/></div>}
      {monitorOpen === "c64-original" && C64Monitor && c64Source && <div className="browser-output" hidden={panel !== "emulator"}><C64Monitor source={c64Source} onRuntime={ownMonitor} onStop={stop}/></div>}
      {monitorOpen === "spectrum-development" && SpectrumMonitor && <div className="browser-output" hidden={panel !== "emulator"}><SpectrumMonitor target={originalSpectrumTarget(machine.id) ? machine.id : undefined} source={spectrumSource} onRuntime={ownMonitor} onDebugger={ownSpectrumDebugger} onInspection={inspectSpectrum} onStop={stop}/></div>}
      {running && <div hidden={panel !== "emulator"}><Emulator key={running.session} {...running} onState={runtimeState} /></div>}
      {panel === "emulator" && !running && !monitorOpen && !development && !originalSession && <div className="browser-output">{!busy && dialog !== "rom" && romSetup}<p role="status">{busy ? message : built ? message : "Choose Check & build to prepare a game, then Play."}</p>{built && !busy && (firmwareReady || builtInSelected) && <button type="button" onClick={run}><PlayIcon />Play game</button>}{busy && <button type="button" onClick={cancel}>Cancel operation</button>}</div>}
      {panel === "output" && output}
      {panel === "import" && (preview ? <div className="browser-import"><div className="browser-import-heading"><p>{preview.kind === "basic" ? "Recovered editable BASIC code" : "Reconstructed machine code (6809 assembly)"} · {preview.warningCount} warnings</p><button type="button" disabled={busy || Boolean(running)} onClick={usePreview}>Edit this copy</button></div>{preview.kind === "assembly" && <p className="browser-muted">Original names and comments cannot be recovered. Open a copy to inspect or save it. Play uses the selected original file until you edit the listing.</p>}<textarea aria-label="Imported source preview" readOnly value={preview.source} /></div> : <div className="browser-output"><p>Open your work or recover source from a game to inspect a copy.</p><button type="button" disabled={busy || Boolean(running)} onClick={() => setDialog("recover")}>Recover source…</button></div>)}
    </> }}
    status={<><span>{machine.label}</span><span>{kind === "basic" ? "BASIC" : `${machine.cpu} assembly`}</span><span>{busy ? "Working…" : running || monitorOpen ? "Playing" : "On your device"}</span><button type="button" onClick={() => reveal()}>Results</button><span className="browser-storage-state">{hasDocument ? storageState : "Projects"}</span></>} />
    {dialog === "github" && <GitHubDialog client={github} localProjectId={project?.id} project={currentSnapshot} cloud={project?.cloud} onClose={closeDialog} onOpen={openRemote} onSaved={savedRemote} />}
    {dialog && dialog !== "github" && <BrowserDialog title={dialog === "create" ? "Create new project" : dialog === "save" ? "Save your work" : dialog === "open" ? "Load existing project" : dialog === "rom" ? `Set up your virtual ${machine.label}` : "Recover source from a game"} onClose={dialog === "rom" ? dismissRom : closeDialog}>
      {dialog === "create" ? <form onSubmit={(event) => { event.preventDefault(); createProject(newName, template === "assembly" ? (newTarget === "vectrex" || originalSpectrumTarget(newTarget)) ? newMachine.assemblySample ?? "" : "START\n        lda #65\n        sta $0400\n        rts\n" : template === "sample" ? newMachine.sample : "", (newTarget === "vectrex" || template === "assembly" && (newTarget === "dragon32" || originalSpectrumTarget(newTarget))) ? "assembly" : "basic", newTarget, template === "assembly" && newTarget === "dragon32" ? { loadAddress: Number(assemblyLoad), entrySymbol: assemblyEntry } : undefined); }}>
        <label className="browser-field">Project name<input autoFocus required maxLength={64} value={newName} onChange={(event) => setNewName(event.currentTarget.value)} /></label>
        <label className="browser-field">Computer<select value={newTarget} onChange={(event) => { setNewTarget(event.currentTarget.value); if (event.currentTarget.value !== "dragon32" && !originalSpectrumTarget(event.currentTarget.value) && template === "assembly") setTemplate("sample"); }}>{["6809", "6502", "Z80"].map((cpu) => <optgroup key={cpu} label={cpu}>{machines.filter((machine) => machine.cpu === cpu).map((machine) => <option key={machine.id} value={machine.id}>{machine.label}</option>)}</optgroup>)}</select></label>
        <label className="browser-field">Start with<select value={template} onChange={(event) => setTemplate(event.currentTarget.value)}><option value="sample">{newTarget === "vectrex" ? "Vector, sound and controls" : newTarget === "dragon32" ? "Guess the number — sample game" : "Starter BASIC programme"}</option><option value="blank">{newTarget === "vectrex" ? "Blank assembly project" : "Blank BASIC project"}</option>{(newTarget === "dragon32" || newMachine.assemblySample) && <option value="assembly">{newMachine.cpu} assembly project</option>}</select></label>
        {newTarget === "dragon32" && template === "assembly" && <><label className="browser-field">Load address (decimal)<input required type="number" min="1024" max="32767" value={assemblyLoad} onChange={event => setAssemblyLoad(event.currentTarget.value)} /></label><label className="browser-field">Entry symbol<input required maxLength={64} value={assemblyEntry} onChange={event => setAssemblyEntry(event.currentTarget.value)} /></label></>}
        <p className="browser-muted">{newMachine.label} {newTarget === "vectrex" || template === "assembly" ? `${newMachine.cpu} assembly` : "BASIC"}. Use letters, numbers, spaces or hyphens. Your project will recover automatically on this device.</p>
        <button className="primary-button" type="submit" disabled={busy}>Create project</button><p role="status">{busy ? "Creating your project…" : message.startsWith("DSW1010") ? message : ""}</p>
      </form> : dialog === "rom" ? <>{romSetup}<p role="status">{message}</p>{busy && <button type="button" onClick={cancel}>Cancel operation</button>}</> : dialog === "save" ? <><p>{storageState}. Keep a backup or continue on another device.</p><div className="browser-save-options">
        <button type="button" disabled={busy} onClick={exportProject}><FilesIcon /><span>Download project ZIP<small>Includes source and desktop-compatible settings · reopen here or extract for the desktop edition</small></span></button>
        <button type="button" onClick={() => setDialog("github")}><SaveIcon /><span>Save to GitHub<small>Keep a copy in your private repository · continue on another device</small></span></button>
        <button type="button" onClick={() => { closeDialog(); download(activeSource, activeSourcePath.split("/").at(-1) ?? "game.bas", "text/plain;charset=utf-8"); }}><SaveIcon /><span>Save editable code<small>{activeSourcePath} · keep this to continue editing</small></span></button>
        <button type="button" disabled={!built || built.revision !== revision.current || busy} onClick={() => { if (built?.revision === revision.current) { closeDialog(); download(new Uint8Array(built.result.cassette), `game.${machine.extension}`, "application/octet-stream"); } }}><PlayIcon /><span>Save playable game file<small>game.{machine.extension} · for a compatible {machine.label} emulator{!built ? " · check & build first" : ""}</small></span></button>
        {originalDragonTarget(machine.id) && <button type="button" disabled={!built || built.revision !== revision.current || busy} onClick={downloadCassetteAudio}><SaveIcon /><span>Save cassette loading audio<small>game.wav · play through the Dragon cassette input · {kind === "assembly" ? "CLOADM, then EXEC" : "CLOAD, then RUN"}{!built ? " · check & build first" : ""}</small></span></button>}
        <button type="button" onClick={() => { if (currentSnapshot && project) { closeDialog(); const copy = { ...project, id: projectId(), version: "", cloud: undefined }; recoveryProjectId.current = copy.id; localCopy.current = undefined; setProject(copy); persist(currentSnapshot, copy); } }}><FilesIcon /><span>Keep a separate local copy<small>Preserve both copies if another tab changed this project</small></span></button>
      </div><p className="browser-muted">Local recovery can be cleared by the browser and is not synchronised with iCloud. Choose your exported file’s destination in Files or Downloads.</p></> : dialog === "open" ? <><p>Choose a saved project ZIP or browser project file. Source files and legacy drafts open as a preview.</p>
        {fileControl("Choose project file…", ".zip,.json", (file) => { closeDialog(); void loadProject(file); })}
        <button type="button" onClick={() => setDialog("github")}>Open from GitHub</button>
        <p className="browser-muted">Local recovery stays in this browser. Download or use GitHub for a durable copy. Single-source BASIC, Dragon 32 RAM assembly, Spectrum assembly and Vectrex cartridge source projects are supported. Other assembly project types use the desktop edition.</p><label className="browser-field">Computer for source files<select value={recoveryTarget} onChange={(event) => setRecoveryTarget(event.currentTarget.value)}>{machines.map((machine) => <option key={machine.id} value={machine.id}>{machine.label}</option>)}</select></label>{fileControl("Choose source file…", recoveryTarget === "vectrex" ? ".asm,.txt" : recoveryTarget === "dragon32" || originalSpectrumTarget(recoveryTarget) ? ".bas,.asm,.txt" : ".bas,.txt", (file) => { closeDialog(); void openSource(file); })}<button type="button" disabled={busy || Boolean(running)} onClick={() => { closeDialog(); restoreLegacyDraft(); }}>Restore browser draft</button></> : <><label className="browser-field">Computer<select value={recoveryTarget} onChange={(event) => setRecoveryTarget(event.currentTarget.value)}>{machines.map((machine) => <option key={machine.id} value={machine.id}>{machine.label}</option>)}</select></label><p>{recoveryTarget === "vectrex" ? "Vectrex cartridge recovery is unavailable. Load a source file or project ZIP instead." : "Choose a BASIC programme file for this computer. Preview the recovered code, then edit a copy."}</p>{recoveryTarget !== "vectrex" && fileControl("Choose game or recording…", ".cas,.wav,.prg,.tap,.p,.ssd", (file) => { closeDialog(); void decompile(file, false); })}<p className="browser-muted">BASIC games can recover editable code. Dragon machine-code files can run using the bundled emulator and ROM. Their recovered 6809 assembly is an editable copy. Cartridge recovery is unsupported on the website. Originals are preserved.</p></>}
    </BrowserDialog>}
  </div>;
}
