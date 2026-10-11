import { MAX_SOURCE_BYTES, record, result, type BrowserEngine } from "./contracts";
export interface ProjectSnapshot { schemaVersion: 1 | 2; target?: string; name: string; kind: "basic" | "assembly"; source: string; manifest: string | null; supportFiles: Record<string, string> }
export interface CloudLink { repository: string; projectId: string; sha: string }
export interface StoredProject { schemaVersion: 1; id: string; version: string; updatedAt: number; project: ProjectSnapshot; entryPath: string; cloud?: CloudLink }
export const PROJECT_PREFIX = "eightbit-studio.browser.project.v1.";
const MAX_JSON = 7 * 1024 * 1024;
export const projectId = () => crypto.randomUUID();
export const projectTarget = (project: ProjectSnapshot) => project.target ?? "dragon32";
export function snapshot(value: unknown): ProjectSnapshot {
  if (!record(value) || !["kind,manifest,name,schemaVersion,source", "kind,manifest,name,schemaVersion,source,supportFiles", "kind,manifest,name,schemaVersion,source,supportFiles,target"].includes(Object.keys(value).sort().join()) || (value.schemaVersion !== 1 && value.schemaVersion !== 2) || (value.schemaVersion === 1 && value.target !== undefined) || (value.schemaVersion === 2 && (typeof value.target !== "string" || !["dragon32", "c64-pal", "bbc-model-b", "spectrum16", "spectrum48", "spectrum128", "spectrum-plus2", "zx81-1k", "zx81-16k"].includes(value.target))) || typeof value.name !== "string" || value.name.length < 1 || value.name.length > 64 || (value.kind !== "basic" && value.kind !== "assembly") || typeof value.source !== "string" || new TextEncoder().encode(value.source).length > MAX_SOURCE_BYTES || !(value.manifest === null || typeof value.manifest === "string" && new TextEncoder().encode(value.manifest).length <= 65_536)) throw new Error("Invalid project snapshot.");
  const files = value.supportFiles ?? {};
  if (!record(files) || Object.keys(files).length > 32 || Object.entries(files).some(([path, text]) => path.length > 256 || typeof text !== "string" || new TextEncoder().encode(text).length > 65_536)) throw new Error("Invalid project support files.");
  const supportFiles: Record<string, string> = {};
  for (const [path, text] of Object.entries(files)) supportFiles[path] = String(text);
  return { supportFiles, schemaVersion: value.schemaVersion, ...(value.schemaVersion === 2 ? { target: String(value.target) } : {}), name: value.name, kind: value.kind, source: value.source, manifest: value.manifest };
}
export function projectResult(value: unknown): { project: ProjectSnapshot; entryPath: string } {
  const data = result(value);
  if (typeof data.entryPath !== "string" || data.entryPath.length > 4096) throw new Error("Invalid project response.");
  return { project: snapshot(data.project), entryPath: data.entryPath };
}
export async function importProject(engine: BrowserEngine, bytes: Uint8Array, signal: AbortSignal) {
  return projectResult(await engine.request({ schemaVersion: 1, operation: "import-project" }, bytes, signal));
}
function stored(text: string): StoredProject {
  if (text.length > MAX_JSON) throw new Error("Stored project exceeds its limit.");
  const value: unknown = JSON.parse(text);
  if (!record(value) || value.schemaVersion !== 1 || typeof value.id !== "string" || !/^[a-f0-9-]{36}$/.test(value.id) || typeof value.version !== "string" || !Number.isSafeInteger(value.updatedAt) || typeof value.entryPath !== "string") throw new Error("Invalid stored project.");
  let cloud: CloudLink | undefined;
  if (value.cloud !== undefined) {
    if (!record(value.cloud) || typeof value.cloud.repository !== "string" || !/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(value.cloud.repository) || typeof value.cloud.projectId !== "string" || !/^[a-f0-9-]{36}$/.test(value.cloud.projectId) || typeof value.cloud.sha !== "string" || !/^[a-f0-9]{40,64}$/.test(value.cloud.sha)) throw new Error("Invalid saved GitHub location.");
    cloud = { repository: value.cloud.repository, projectId: value.cloud.projectId, sha: value.cloud.sha };
  }
  return { schemaVersion: 1, id: value.id, version: value.version, updatedAt: Number(value.updatedAt), project: snapshot(value.project), entryPath: value.entryPath, ...(cloud ? { cloud } : {}) };
}
/** Each replacement is a single atomic storage write; a second tab cannot silently replace a newer copy. */
export async function saveProject(project: Omit<StoredProject, "version" | "updatedAt">, expectedVersion?: string): Promise<StoredProject> {
  const key = PROJECT_PREFIX + project.id;
  if (!navigator.locks) throw new Error("Safe local recovery is unavailable in this browser. Download your project instead.");
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 3000);
  try { return await navigator.locks.request(key, { signal: controller.signal }, () => {
    const previous = localStorage.getItem(key);
    if (previous && stored(previous).version !== expectedVersion) throw new Error("Another tab changed this project. Download your project or keep a separate local copy before reopening it.");
    const next = { ...project, version: projectId(), updatedAt: Date.now() };
    const json = JSON.stringify(next);
    if (json.length > MAX_JSON) throw new Error("Project exceeds the browser storage limit. Download it instead.");
    localStorage.setItem(key, json);
    return next;
  }); } finally { clearTimeout(timer); }
}
export function recentProjects(): StoredProject[] {
  const projects: StoredProject[] = [];
  const count = Math.min(localStorage.length, 1000);
  for (let index = 0; index < count; index++) {
    const key = localStorage.key(index);
    if (!key?.startsWith(PROJECT_PREFIX)) continue;
    try { const text = localStorage.getItem(key); if (text) { const value = stored(text); if (key === PROJECT_PREFIX + value.id) projects.push(value); } } catch { /* Damaged records do not hide the remaining recovery copies. */ }
  }
  return projects.sort((a, b) => b.updatedAt - a.updatedAt).slice(0, 8);
}
