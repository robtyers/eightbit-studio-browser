/** Optional same-origin GitHub boundary. No browser or emulator module imports this file. */
interface Storage {
  get<T>(key: string): Promise<T | undefined>; put(key: string, value: unknown): Promise<void>;
  deleteAll(): Promise<void>; setAlarm(time: number): Promise<void>;
}
interface Namespace { idFromName(name: string): unknown; get(id: unknown): { fetch(request: Request): Promise<Response> } }
export interface Env { APP_ORIGIN: string; GITHUB_CLIENT_ID: string; GITHUB_CLIENT_SECRET: string; GITHUB_APP_SLUG: string; SESSION_KEY: string; SESSIONS: Namespace; ALLOW_LOCAL_HTTP?: string }
interface Tokens { access: string; refresh: string; accessExpires: number; refreshExpires: number; expires: number }
const PREFIX = "/studio-api/";
const DIRECTORY = "eightbit-studio/projects";
const MAX_PROJECT = 7 * 1024 * 1024;
const UUID = /^[a-f0-9]{8}-[a-f0-9]{4}-4[a-f0-9]{3}-[89ab][a-f0-9]{3}-[a-f0-9]{12}$/;
const SHA = /^[a-f0-9]{40,64}$/;
class Failure extends Error { constructor(readonly status: number, message: string) { super(message); } }
function object(value: unknown): value is Record<string, unknown> { return value !== null && typeof value === "object" && !Array.isArray(value); }
function json(value: unknown, status = 200): Response { return new Response(JSON.stringify(value), { status, headers: { "Content-Type": "application/json", "Cache-Control": "no-store", "X-Content-Type-Options": "nosniff", "Content-Security-Policy": "default-src 'none'; frame-ancestors 'none'", "Referrer-Policy": "no-referrer" } }); }
async function bounded(response: Response | Request, limit: number): Promise<string> {
  if (Number(response.headers.get("Content-Length")) > limit) throw new Failure(413, "Project or response exceeds its size limit.");
  const reader = response.body?.getReader(); if (!reader) return "";
  const parts: Uint8Array[] = []; let size = 0; let timedOut = false;
  const timer = setTimeout(() => { timedOut = true; void reader.cancel().catch(() => {}); }, 12_000);
  try { while (true) { const part = await reader.read(); if (part.done) break; size += part.value.byteLength; if (size > limit) throw new Failure(413, "Project or response exceeds its size limit."); parts.push(part.value); } }
  finally { clearTimeout(timer); if (size > limit) await reader.cancel(); reader.releaseLock(); }
  if (timedOut) throw new Failure(408, "Request timed out. Your local work is preserved.");
  const bytes = new Uint8Array(size); let offset = 0; for (const part of parts) { bytes.set(part, offset); offset += part.byteLength; }
  return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
}
async function parsed(response: Response | Request, limit: number): Promise<unknown> { try { return JSON.parse(await bounded(response, limit)); } catch (error) { if (error instanceof Failure) throw error; throw new Failure(400, "Invalid request or provider response."); } }
async function provider(url: string, init: RequestInit, limit = 256 * 1024): Promise<{ status: number; data: unknown }> {
  const controller = new AbortController(); const timer = setTimeout(() => controller.abort(), 12_000);
  try {
    // workerd supports manual/follow only; reject redirects before reading or forwarding credentials.
    const response = await fetch(url, { ...init, signal: controller.signal, redirect: "manual" });
    if (response.status >= 300 && response.status < 400) {
      await response.body?.cancel();
      throw new Failure(503, "GitHub is unavailable or timed out. Your local work is preserved.");
    }
    return { status: response.status, data: await parsed(response, limit) };
  }
  catch (error) { if (error instanceof Failure) throw error; throw new Failure(503, "GitHub is unavailable or timed out. Your local work is preserved."); }
  finally { clearTimeout(timer); }
}
function base64(bytes: Uint8Array): string { let text = ""; for (let offset = 0; offset < bytes.length; offset += 8192) text += String.fromCharCode(...bytes.subarray(offset, offset + 8192)); return btoa(text); }
function unbase64(value: string): Uint8Array { const text = atob(value); return Uint8Array.from(text, (letter) => letter.charCodeAt(0)); }
function random(): string { return base64(crypto.getRandomValues(new Uint8Array(32))).replaceAll("+", "-").replaceAll("/", "_").replaceAll("=", ""); }
function origin(env: Env): string {
  const url = new URL(env.APP_ORIGIN);
  if (url.origin !== env.APP_ORIGIN || (url.protocol !== "https:" && !(env.ALLOW_LOCAL_HTTP === "true" && url.protocol === "http:" && ["localhost", "127.0.0.1"].includes(url.hostname)))) throw new Failure(503, "GitHub saving is not configured on this site yet.");
  return url.origin;
}
function cookieName(env: Env, auth = false) { return `${origin(env).startsWith("https:") ? "__Host-" : ""}eightbit-${auth ? "oauth" : "session"}`; }
function cookie(env: Env, value: string, seconds: number, auth = false): string { return `${cookieName(env, auth)}=${value}; Path=/; HttpOnly; SameSite=Lax; Max-Age=${seconds}${origin(env).startsWith("https:") ? "; Secure" : ""}`; }
function readCookie(request: Request, name: string): string { return request.headers.get("Cookie")?.split(";").map((item) => item.trim()).find((item) => item.startsWith(name + "="))?.slice(name.length + 1) ?? ""; }
async function key(env: Env) { let bytes: Uint8Array; try { bytes = unbase64(env.SESSION_KEY); } catch { throw new Failure(503, "GitHub saving is not configured on this site yet."); } if (bytes.length !== 32) throw new Failure(503, "GitHub saving is not configured on this site yet."); return crypto.subtle.importKey("raw", new Uint8Array(bytes), "AES-GCM", false, ["encrypt", "decrypt"]); }
async function seal(env: Env, data: unknown): Promise<string> { const iv = crypto.getRandomValues(new Uint8Array(12)); const encrypted = new Uint8Array(await crypto.subtle.encrypt({ name: "AES-GCM", iv }, await key(env), new TextEncoder().encode(JSON.stringify(data)))); const output = new Uint8Array(iv.length + encrypted.length); output.set(iv); output.set(encrypted, iv.length); return base64(output); }
async function unseal(env: Env, value: string): Promise<unknown> { if (value.length > 2048) throw new Failure(401, "GitHub connection expired. Connect again."); try { const bytes = unbase64(value); return JSON.parse(new TextDecoder().decode(await crypto.subtle.decrypt({ name: "AES-GCM", iv: new Uint8Array(bytes.slice(0, 12)) }, await key(env), new Uint8Array(bytes.slice(12))))); } catch { throw new Failure(401, "GitHub connection expired. Connect again."); } }
async function exchange(env: Env, fields: Record<string, string>, expires = Date.now() + 30 * 86400_000): Promise<Tokens> {
  const response = await provider("https://github.com/login/oauth/access_token", { method: "POST", headers: { Accept: "application/json", "Content-Type": "application/json" }, body: JSON.stringify({ client_id: env.GITHUB_CLIENT_ID, client_secret: env.GITHUB_CLIENT_SECRET, ...fields }) });
  const data = response.data;
  if (response.status !== 200 || !object(data) || typeof data.access_token !== "string" || !data.access_token.length || data.access_token.length > 1024 || typeof data.refresh_token !== "string" || !data.refresh_token.length || data.refresh_token.length > 1024 || !Number.isSafeInteger(data.expires_in) || Number(data.expires_in) <= 0 || Number(data.expires_in) > 86400 || !Number.isSafeInteger(data.refresh_token_expires_in) || Number(data.refresh_token_expires_in) <= 0 || Number(data.refresh_token_expires_in) > 366 * 86400) throw new Failure(401, "GitHub authorisation expired or failed. Connect again.");
  return { access: data.access_token, refresh: data.refresh_token, accessExpires: Date.now() + Number(data.expires_in) * 1000, refreshExpires: Date.now() + Number(data.refresh_token_expires_in) * 1000, expires };
}
/** One opaque session per object; queued refreshes cannot reuse a rotated refresh token. */
export class GitHubSession {
  private queue: Promise<unknown> = Promise.resolve(); private waiting = 0;
  constructor(private readonly state: { storage: Storage }, private readonly env: Env) {}
  async alarm() { await this.state.storage.deleteAll(); }
  fetch(request: Request): Promise<Response> {
    if (this.waiting >= 16) return Promise.resolve(json({ message: "Too many session requests." }, 429));
    this.waiting++;
    const task = this.queue.then(async () => {
      const path = new URL(request.url).pathname;
      if (path === "/store" && request.method === "POST") { const tokens = await request.json() as Tokens; await this.state.storage.put("tokens", tokens); await this.state.storage.setAlarm(tokens.expires); return json({ ok: true }); }
      if (path === "/delete" && request.method === "POST") { await this.state.storage.deleteAll(); return json({ ok: true }); }
      if (path !== "/token" || request.method !== "GET") return json({}, 404);
      let tokens = await this.state.storage.get<Tokens>("tokens");
      if (!tokens || tokens.expires <= Date.now() || tokens.refreshExpires <= Date.now()) { await this.state.storage.deleteAll(); return json({}, 401); }
      if (tokens.accessExpires < Date.now() + 60_000) {
        tokens = await exchange(this.env, { grant_type: "refresh_token", refresh_token: tokens.refresh }, tokens.expires);
        await this.state.storage.put("tokens", tokens);
      }
      return json({ token: tokens.access });
    }).catch((error: unknown) => json({ message: error instanceof Failure ? error.message : "Session unavailable." }, error instanceof Failure ? error.status : 503)).finally(() => { this.waiting--; });
    this.queue = task; return task;
  }
}
function session(env: Env, id: string) { return env.SESSIONS.get(env.SESSIONS.idFromName(id)); }
async function token(request: Request, env: Env): Promise<string> {
  const id = readCookie(request, cookieName(env));
  if (!/^[A-Za-z0-9_-]{43}$/.test(id)) throw new Failure(401, "Connect GitHub to open or save a project. Local projects work without an account.");
  const response = await session(env, id).fetch(new Request("https://session/token"));
  const value: unknown = await response.json();
  if (!response.ok || !object(value) || typeof value.token !== "string") throw new Failure(response.status === 401 ? 401 : 503, response.status === 401 ? "Your GitHub session expired. Connect again; your local work is preserved." : "GitHub session is unavailable. Try again later.");
  return value.token;
}
async function api(access: string, path: string, body?: unknown, limit = 256 * 1024) {
  const response = await provider(`https://api.github.com${path}`, { method: body === undefined ? "GET" : "PUT", headers: { Authorization: `Bearer ${access}`, Accept: "application/vnd.github+json", "X-GitHub-Api-Version": "2026-03-10", "User-Agent": "Eightbit-Studio", ...(body === undefined ? {} : { "Content-Type": "application/json" }) }, ...(body === undefined ? {} : { body: JSON.stringify(body) }) }, limit);
  if (response.status >= 400) throw new Failure(response.status === 409 || response.status === 422 ? 409 : response.status === 401 ? 401 : response.status === 403 || response.status === 404 ? 403 : 503, response.status === 409 || response.status === 422 ? "GitHub has a different copy, or could not accept this save. Your local project is preserved. Open the GitHub copy separately or save a separate copy." : response.status === 401 ? "GitHub authorisation expired. Connect again." : response.status === 403 || response.status === 404 ? "This repository is unavailable or not authorised. Select a private repository installed for Eightbit Studio." : "GitHub could not complete the request. Your local work is preserved.");
  return response.data;
}
function repository(value: unknown): string { if (typeof value !== "string" || value.length > 200 || !/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(value) || value.split("/").some((part) => part === "." || part === "..")) throw new Failure(400, "Choose a valid repository."); return value; }
async function repo(access: string, name: string): Promise<string> {
  const data = await api(access, `/repos/${name}`);
  if (!object(data) || data.private !== true || !object(data.permissions) || data.permissions.push !== true || typeof data.default_branch !== "string" || data.default_branch.length > 256 || /[\x00-\x1f]/.test(data.default_branch)) throw new Failure(403, "Choose an authorised private repository you can write to.");
  return data.default_branch;
}
function snapshot(value: unknown): Record<string, unknown> {
  if (!object(value) || !["kind,manifest,name,schemaVersion,source", "kind,manifest,name,schemaVersion,source,supportFiles", "kind,manifest,name,schemaVersion,source,supportFiles,target"].includes(Object.keys(value).sort().join()) || (value.schemaVersion !== 1 && value.schemaVersion !== 2) || (value.schemaVersion === 1 && value.target !== undefined) || (value.schemaVersion === 2 && (typeof value.target !== "string" || !["dragon32", "c64-pal", "bbc-model-b", "spectrum16", "spectrum48", "spectrum128", "spectrum-plus2", "zx81-1k", "zx81-16k"].includes(value.target))) || typeof value.name !== "string" || !/^[A-Za-z0-9][A-Za-z0-9 _-]{0,63}$/.test(value.name) || (value.kind !== "basic" && value.kind !== "assembly") || typeof value.source !== "string" || new TextEncoder().encode(value.source).length > 1024 * 1024 || (value.kind === "basic" ? typeof value.manifest !== "string" || new TextEncoder().encode(value.manifest).length > 65_536 : value.manifest !== null)) throw new Failure(400, "Invalid project snapshot. Only source and project settings can be saved.");
  const files = value.supportFiles ?? {};
  if (!object(files) || Object.keys(files).length > 32 || Object.entries(files).some(([path, content]) => !["AGENTS.md", "CLAUDE.md", "README.md", "tests/README.md", ".gitignore"].includes(path) && !/^tests\/[A-Za-z0-9._-]{1,100}\.dragon-test\.toml$/.test(path) || typeof content !== "string" || new TextEncoder().encode(content).length > 65_536) || Object.values(files).reduce<number>((total, item) => total + new TextEncoder().encode(String(item)).length, 0) > 512 * 1024) throw new Failure(400, "Invalid project support files.");
  if (new TextEncoder().encode(JSON.stringify(value)).length > MAX_PROJECT) throw new Failure(413, "Project snapshot exceeds its limit.");
  return value;
}
interface Entry { id: string; name: string; path: string; sha: string }
async function entries(access: string, name: string, branch: string): Promise<Entry[]> {
  // Directory-not-found is an empty project collection, not permission to bypass repository checks.
  const response = await provider(`https://api.github.com/repos/${name}/contents/${DIRECTORY}?ref=${encodeURIComponent(branch)}`, { headers: { Authorization: `Bearer ${access}`, Accept: "application/vnd.github+json", "X-GitHub-Api-Version": "2026-03-10", "User-Agent": "Eightbit-Studio" } });
  if (response.status === 404) return [];
  if (response.status !== 200 || !Array.isArray(response.data) || response.data.length > 100) throw new Failure(400, "This repository's project collection is unavailable or exceeds 100 files.");
  const output: Entry[] = [];
  for (const item of response.data) {
    if (!object(item) || item.type !== "file" || typeof item.name !== "string" || typeof item.sha !== "string" || !SHA.test(item.sha)) continue;
    const match = /^([A-Za-z0-9_-]{1,64})--([a-f0-9-]{36})\.json$/.exec(item.name);
    if (!match || !UUID.test(match[2] ?? "")) continue;
    const id = match[2] ?? ""; if (output.some((entry) => entry.id === id)) throw new Failure(409, "The GitHub collection contains duplicate project identities. Open a downloaded copy instead.");
    output.push({ id, name: (match[1] ?? "Project").replaceAll("-", " "), path: `${DIRECTORY}/${item.name}`, sha: item.sha });
  }
  if (response.data.length === 100 && output.length !== 100) throw new Failure(400, "The project collection is full. Remove unrelated files in GitHub or choose another repository.");
  return output;
}
async function repositories(access: string, page: number) {
  const index = Math.floor(page / 1000); const repositoryPage = page % 1000;
  if (!Number.isSafeInteger(page) || repositoryPage < 1 || repositoryPage > 100 || index > 99) throw new Failure(400, "Invalid repository page.");
  const installations = await api(access, "/user/installations?per_page=100&page=1");
  if (!object(installations) || !Array.isArray(installations.installations) || Number(installations.total_count) > 100) throw new Failure(400, "GitHub installation list exceeds its limit.");
  const installation = installations.installations[index];
  if (!object(installation) || !Number.isSafeInteger(installation.id)) return { repositories: [] };
  const data = await api(access, `/user/installations/${installation.id}/repositories?per_page=30&page=${repositoryPage}`);
  if (!object(data) || !Array.isArray(data.repositories) || data.repositories.length > 30 || !Number.isSafeInteger(data.total_count)) throw new Failure(503, "Invalid GitHub repository response.");
  const names = data.repositories.filter((item) => object(item) && item.private === true && object(item.permissions) && item.permissions.push === true).map((item: Record<string, unknown>) => ({ name: repository(item.full_name) }));
  const nextPage = Number(data.total_count) > repositoryPage * 30 && repositoryPage < 100 ? page + 1 : index + 1 < installations.installations.length ? (index + 1) * 1000 + 1 : undefined;
  return { repositories: names, ...(nextPage ? { nextPage } : {}) };
}
function redirect(location: string, cookies: string[] = []) { const headers = new Headers({ Location: location, "Cache-Control": "no-store", "Referrer-Policy": "no-referrer" }); for (const value of cookies) headers.append("Set-Cookie", value); return new Response(null, { status: 302, headers }); }
async function handle(request: Request, env: Env): Promise<Response> {
  const url = new URL(request.url); const site = origin(env);
  if (!env.GITHUB_CLIENT_ID || !env.GITHUB_CLIENT_SECRET || !env.SESSION_KEY || !env.SESSIONS) throw new Failure(503, "GitHub saving is not configured on this site yet. Local projects and downloads still work.");
  const route = url.pathname.slice(PREFIX.length);
  if (!url.pathname.startsWith(PREFIX)) throw new Failure(404, "Unknown request.");
  if (request.method === "POST" && (request.headers.get("Origin") !== site || request.headers.get("X-Studio-Request") !== "1" || request.headers.get("Content-Type") !== "application/json")) throw new Failure(403, "This request must come from Eightbit Studio on this site.");
  if (request.method !== "GET" && request.method !== "POST") throw new Failure(405, "Unsupported request method.");
  if (request.method === "GET" && route === "install") { if (!/^[a-z0-9-]{1,100}$/.test(env.GITHUB_APP_SLUG)) throw new Failure(503, "GitHub App installation is not configured."); return redirect(`https://github.com/apps/${env.GITHUB_APP_SLUG}/installations/new`); }
  if (request.method === "GET" && route === "connect") {
    const state = random(); const verifier = random();
    const challenge = base64(new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(verifier)))).replaceAll("+", "-").replaceAll("/", "_").replaceAll("=", "");
    const target = new URL("https://github.com/login/oauth/authorize");
    for (const [name, value] of Object.entries({ client_id: env.GITHUB_CLIENT_ID, redirect_uri: `${site}${PREFIX}callback`, state, code_challenge: challenge, code_challenge_method: "S256" })) target.searchParams.set(name, value);
    return redirect(target.href, [cookie(env, await seal(env, { state, verifier, returnProject: UUID.test(url.searchParams.get("project") ?? "") ? url.searchParams.get("project") : null, expires: Date.now() + 600_000 }), 600, true)]);
  }
  if (request.method === "GET" && route === "callback") {
    const auth = await unseal(env, readCookie(request, cookieName(env, true)));
    const code = url.searchParams.get("code");
    if (!object(auth) || auth.state !== url.searchParams.get("state") || typeof auth.verifier !== "string" || !/^[A-Za-z0-9_-]{43}$/.test(auth.verifier) || typeof auth.expires !== "number" || auth.expires < Date.now() || !code || code.length > 1024) throw new Failure(401, "GitHub connection did not complete. Return to the studio and connect again.");
    const tokens = await exchange(env, { code, redirect_uri: `${site}${PREFIX}callback`, code_verifier: auth.verifier });
    const id = random(); const stored = await session(env, id).fetch(new Request("https://session/store", { method: "POST", body: JSON.stringify(tokens) }));
    if (!stored.ok) throw new Failure(503, "Cannot retain the GitHub session. Try connecting again.");
    return redirect(`${site}/studio/#github${typeof auth.returnProject === "string" && UUID.test(auth.returnProject) ? "=" + auth.returnProject : ""}`, [cookie(env, id, 30 * 86400), cookie(env, "", 0, true)]);
  }
  if (request.method === "POST" && route === "disconnect") {
    const id = readCookie(request, cookieName(env)); if (/^[A-Za-z0-9_-]{43}$/.test(id)) { const deleted = await session(env, id).fetch(new Request("https://session/delete", { method: "POST" })); if (!deleted.ok) throw new Failure(503, "Cannot disconnect yet. Try again later."); }
    const response = json({ ok: true }); response.headers.append("Set-Cookie", cookie(env, "", 0)); return response;
  }
  const access = await token(request, env);
  if (request.method === "GET" && route === "repositories") return json(await repositories(access, Number(url.searchParams.get("page") ?? 1)));
  if (request.method === "GET" && (route === "projects" || route === "project")) {
    const name = repository(url.searchParams.get("repository")); const branch = await repo(access, name); const files = await entries(access, name, branch);
    if (route === "projects") return json({ projects: files.map(({ id, name }) => ({ id, name })) });
    const id = url.searchParams.get("id") ?? ""; if (!UUID.test(id)) throw new Failure(400, "Invalid project identity.");
    const entry = files.find((item) => item.id === id); if (!entry) throw new Failure(404, "This GitHub project is no longer available.");
    const blob = await api(access, `/repos/${name}/git/blobs/${entry.sha}`, undefined, 12 * 1024 * 1024);
    if (!object(blob) || blob.sha !== entry.sha || blob.encoding !== "base64" || typeof blob.content !== "string" || Number(blob.size) > MAX_PROJECT) throw new Failure(400, "Invalid GitHub project file.");
    let project: unknown; try { project = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(unbase64(blob.content.replaceAll("\n", "")))); } catch { throw new Failure(400, "Invalid GitHub project file."); }
    return json({ sha: entry.sha, project: snapshot(project) });
  }
  if (request.method === "POST" && route === "save") {
    const body = await parsed(request, MAX_PROJECT + 4096);
    if (!object(body) || Object.keys(body).sort().join() !== "id,project,repository,sha" || typeof body.id !== "string" || !UUID.test(body.id) || !(body.sha === null || typeof body.sha === "string" && SHA.test(body.sha))) throw new Failure(400, "Invalid save request.");
    const project = snapshot(body.project); const name = repository(body.repository); const branch = await repo(access, name);
    const files = await entries(access, name, branch); const existing = files.find((item) => item.id === body.id);
    if (existing ? existing.sha !== body.sha : body.sha !== null) throw new Failure(409, "GitHub has a different copy. Your local project is preserved. Open the GitHub copy separately or save a separate copy.");
    if (!existing && files.length >= 100) throw new Failure(400, "This repository already contains 100 projects. Choose another repository.");
    const path = existing?.path ?? `${DIRECTORY}/${String(project.name).replaceAll(" ", "-")}--${body.id}.json`;
    const saved = await api(access, `/repos/${name}/contents/${path}`, { message: `Save Eightbit Studio project: ${project.name}`, content: base64(new TextEncoder().encode(JSON.stringify(project))), branch, ...(body.sha ? { sha: body.sha } : {}) });
    if (!object(saved) || !object(saved.content) || typeof saved.content.sha !== "string" || !SHA.test(saved.content.sha)) throw new Failure(503, "GitHub did not return a save receipt. Open the remote copy before retrying.");
    return json({ sha: saved.content.sha });
  }
  throw new Failure(404, "Unknown request.");
}
export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    try { return await handle(request, env); }
    catch (error: unknown) { return json({ message: error instanceof Failure ? error.message : "GitHub request failed. Your local work is preserved." }, error instanceof Failure ? error.status : 503); }
  },
};
