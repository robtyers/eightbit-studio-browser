import { record } from "./contracts";
import { snapshot, type ProjectSnapshot } from "./projects";
export interface Repository { name: string }
export interface RemoteProject { id: string; sha: string; project: ProjectSnapshot }
export class GitHubError extends Error { constructor(message: string, readonly status: number) { super(message); } }
export interface GitHubClient {
  repositories(signal: AbortSignal): Promise<{ repositories: Repository[]; nextPage?: number }>;
  moreRepositories(page: number, signal: AbortSignal): Promise<{ repositories: Repository[]; nextPage?: number }>;
  list(repository: string, signal: AbortSignal): Promise<{ id: string; name: string }[]>;
  open(repository: string, id: string, signal: AbortSignal): Promise<RemoteProject>;
  save(repository: string, id: string, sha: string | null, project: ProjectSnapshot, signal: AbortSignal): Promise<{ sha: string }>;
  disconnect(signal: AbortSignal): Promise<void>;
}
const idPattern = /^[a-f0-9-]{36}$/;
const shaPattern = /^[a-f0-9]{40,64}$/;
export class SameOriginGitHub implements GitHubClient {
  private async request(path: string, signal: AbortSignal, body?: unknown): Promise<unknown> {
    const timeout = new AbortController();
    const cancel = () => timeout.abort(); signal.addEventListener("abort", cancel, { once: true });
    if (signal.aborted) timeout.abort();
    const timer = setTimeout(cancel, 20_000);
    try {
      const response = await fetch(`/studio-api/${path}`, { signal: timeout.signal, credentials: "same-origin", redirect: "error", cache: "no-store", headers: { Accept: "application/json", ...(body === undefined ? {} : { "Content-Type": "application/json", "X-Studio-Request": "1" }) }, ...(body === undefined ? {} : { method: "POST", body: JSON.stringify(body) }) });
      const reader = response.body?.getReader();
      if (!reader) throw new GitHubError("GitHub saving is unavailable. Local projects and downloads still work.", 503);
      const chunks: Uint8Array[] = []; let size = 0;
      while (true) { const part = await reader.read(); if (part.done) break; size += part.value.byteLength; if (size > 8 * 1024 * 1024) { await reader.cancel(); throw new Error("GitHub response exceeds its limit."); } chunks.push(part.value); }
      const bytes = new Uint8Array(size); let offset = 0; for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.length; }
      let data: unknown; try { data = JSON.parse(new TextDecoder().decode(bytes)); } catch { throw new GitHubError("GitHub saving is not configured on this site yet. Local projects and downloads still work.", 503); }
      if (!response.ok) throw new GitHubError(record(data) && typeof data.message === "string" ? data.message.slice(0, 256) : "GitHub request failed. Your local work is preserved.", response.status);
      return data;
    } finally { clearTimeout(timer); signal.removeEventListener("abort", cancel); }
  }
  private async page(page: number, signal: AbortSignal) {
    const data = await this.request(`repositories?page=${page}`, signal);
    if (!record(data) || !Array.isArray(data.repositories) || data.repositories.length > 30 || data.repositories.some((item) => !record(item) || typeof item.name !== "string" || !/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(item.name)) || (data.nextPage !== undefined && (!Number.isSafeInteger(data.nextPage) || Number(data.nextPage) < 2 || Number(data.nextPage) > 100000))) throw new Error("Invalid repository response.");
    return { repositories: data.repositories.map((item: Record<string, unknown>) => ({ name: String(item.name) })), ...(data.nextPage === undefined ? {} : { nextPage: Number(data.nextPage) }) };
  }
  repositories(signal: AbortSignal) { return this.page(1, signal); }
  moreRepositories(page: number, signal: AbortSignal) { return this.page(page, signal); }
  async list(repository: string, signal: AbortSignal) {
    const data = await this.request(`projects?repository=${encodeURIComponent(repository)}`, signal);
    if (!record(data) || !Array.isArray(data.projects) || data.projects.length > 100 || data.projects.some((item) => !record(item) || typeof item.id !== "string" || !idPattern.test(item.id) || typeof item.name !== "string" || item.name.length > 64)) throw new Error("Invalid project list.");
    return data.projects.map((item: Record<string, unknown>) => ({ id: String(item.id), name: String(item.name) }));
  }
  async open(repository: string, id: string, signal: AbortSignal): Promise<RemoteProject> {
    const data = await this.request(`project?repository=${encodeURIComponent(repository)}&id=${encodeURIComponent(id)}`, signal);
    if (!record(data) || typeof data.sha !== "string" || !shaPattern.test(data.sha)) throw new Error("Invalid GitHub project response.");
    return { id, sha: data.sha, project: snapshot(data.project) };
  }
  async save(repository: string, id: string, sha: string | null, project: ProjectSnapshot, signal: AbortSignal) {
    const data = await this.request("save", signal, { repository, id, sha, project });
    if (!record(data) || typeof data.sha !== "string" || !shaPattern.test(data.sha)) throw new Error("Invalid GitHub save receipt.");
    return { sha: data.sha };
  }
  async disconnect(signal: AbortSignal) { await this.request("disconnect", signal, {}); }
}
