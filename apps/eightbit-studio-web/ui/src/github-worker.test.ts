import worker, { GitHubSession, type Env } from "../../github-worker/index";
const SITE = "https://studio.example";
const ID = "12345678-1234-4123-8123-123456789abc";
const SHA = "a".repeat(40);
const PROJECT = { schemaVersion: 1, name: "My game", kind: "basic", source: "10 END\n", manifest: "schema_version = 1" };
const ACCESS = "ACCESS_CANARY_PRIVATE"; const REFRESH = "REFRESH_CANARY_PRIVATE"; const SECRET = "SECRET_CANARY_PRIVATE";
const json = (value: unknown, status = 200) => new Response(JSON.stringify(value), { status, headers: { "Content-Type": "application/json" } });
function environment() {
  const objects = new Map<string, GitHubSession>();
  const env: Env = { APP_ORIGIN: SITE, GITHUB_CLIENT_ID: "test", GITHUB_CLIENT_SECRET: SECRET, GITHUB_APP_SLUG: "test-app", SESSION_KEY: btoa("x".repeat(32)), SESSIONS: { idFromName: (name) => name, get: (id) => {
    const name = String(id); let object = objects.get(name);
    if (!object) { const data = new Map<string, unknown>(); object = new GitHubSession({ storage: { get: async <T>(key: string) => data.get(key) as T | undefined, put: async (key, value) => { data.set(key, value); }, deleteAll: async () => { data.clear(); }, setAlarm: async () => {} } }, env); objects.set(name, object); }
    return { fetch: (request) => object.fetch(request) };
  } } };
  return env;
}
function request(path: string, cookie = "", body?: unknown, from = SITE) {
  return new Request(`${SITE}/studio-api/${path}`, { headers: { Cookie: cookie, ...(body === undefined ? {} : { Origin: from, "Content-Type": "application/json", "X-Studio-Request": "1" }) }, ...(body === undefined ? {} : { method: "POST", body: JSON.stringify(body) }) });
}
async function login(env: Env) {
  const start = await worker.fetch(request("connect"), env);
  const location = new URL(start.headers.get("location") ?? "");
  expect(location.origin).toBe("https://github.com"); expect(location.searchParams.get("code_challenge_method")).toBe("S256");
  const cookie = start.headers.get("set-cookie")?.split(";")[0] ?? "";
  expect(start.headers.get("set-cookie")).toContain("HttpOnly"); expect(start.headers.get("set-cookie")).toContain("Secure");
  const result = await worker.fetch(request(`callback?state=${location.searchParams.get("state")}&code=test`, cookie), env);
  expect(result.status).toBe(302); expect(result.headers.get("location")).toBe(SITE + "/studio/#github");
  const cookies = result.headers.getSetCookie();
  const session = cookies.find((item) => item.startsWith("__Host-eightbit-session="))?.split(";")[0] ?? "";
  expect(session).not.toContain(ACCESS); expect(session).not.toContain(REFRESH);
  return session;
}
let remoteSha = SHA; let writes: { url: string; body: Record<string, unknown> }[];
function mockProvider() {
  vi.stubGlobal("fetch", vi.fn(async (url: string, init?: RequestInit) => {
    expect(init?.redirect).toBe("manual"); expect(init?.signal).toBeInstanceOf(AbortSignal);
    if (url === "https://github.com/login/oauth/access_token") return json({ access_token: ACCESS, refresh_token: REFRESH, expires_in: 28800, refresh_token_expires_in: 15811200 });
    expect(new Headers(init?.headers).get("Authorization")).toBe(`Bearer ${ACCESS}`);
    if (url.includes("/user/installations?")) return json({ total_count: 1, installations: [{ id: 1 }] });
    if (url.includes("/user/installations/1/repositories")) return json({ total_count: 2, repositories: [{ full_name: "me/private", private: true, permissions: { push: true } }, { full_name: "me/public", private: false, permissions: { push: true } }] });
    if (url === "https://api.github.com/repos/me/private") return json({ private: true, default_branch: "main", permissions: { push: true } });
    if (url === "https://api.github.com/repos/me/public") return json({ private: false, default_branch: "main", permissions: { push: true } });
    if (url.includes("/contents/eightbit-studio/projects?")) return json([{ type: "file", name: `My-game--${ID}.json`, sha: remoteSha }]);
    if (url.endsWith(`/git/blobs/${remoteSha}`)) return json({ sha: remoteSha, encoding: "base64", size: JSON.stringify(PROJECT).length, content: btoa(JSON.stringify(PROJECT)) });
    if (init?.method === "PUT") { const body: unknown = JSON.parse(String(init.body)); if (!body || typeof body !== "object" || Array.isArray(body)) throw new Error("Invalid PUT"); writes.push({ url, body: body as Record<string, unknown> }); remoteSha = "b".repeat(40); return json({ content: { sha: remoteSha } }, 201); }
    throw new Error(`Unexpected provider path: ${url}`);
  }));
}
beforeEach(() => { remoteSha = SHA; writes = []; mockProvider(); });
afterEach(() => { vi.unstubAllGlobals(); vi.useRealTimers(); });
it("implements PKCE login, private repository selection, bounded source-only open/save and cookie logout", async () => {
  const env = environment(); const cookie = await login(env);
  const repositories = await worker.fetch(request("repositories", cookie), env);
  expect(await repositories.json()).toEqual({ repositories: [{ name: "me/private" }] });
  const listing = await worker.fetch(request("projects?repository=me/private", cookie), env);
  expect(await listing.json()).toEqual({ projects: [{ id: ID, name: "My game" }] });
  const opened = await worker.fetch(request(`project?repository=me/private&id=${ID}`, cookie), env);
  expect(await opened.json()).toEqual({ sha: SHA, project: PROJECT });
  const saved = await worker.fetch(request("save", cookie, { repository: "me/private", id: ID, sha: SHA, project: PROJECT }), env);
  expect(await saved.json()).toEqual({ sha: "b".repeat(40) });
  expect(writes).toHaveLength(1); expect(writes[0]?.url).toBe(`https://api.github.com/repos/me/private/contents/eightbit-studio/projects/My-game--${ID}.json`);
  const outbound = JSON.stringify(writes); expect(outbound).not.toContain(SECRET); expect(outbound).not.toContain(ACCESS); expect(outbound).not.toContain(REFRESH);
  const decoded = atob(String(writes[0]?.body.content)); expect(JSON.parse(decoded)).toEqual(PROJECT); expect(decoded).not.toContain("firmware");
  const disconnected = await worker.fetch(request("disconnect", cookie, {}), env); expect(disconnected.status).toBe(200); expect(disconnected.headers.get("set-cookie")).toContain("Max-Age=0");
  expect((await worker.fetch(request("repositories", cookie), env)).status).toBe(401);
});

it("accepts a BBC BASIC source snapshot without accepting firmware or changing its target", async () => {
  const env = environment(); const cookie = await login(env);
  const project = { ...PROJECT, schemaVersion: 2, target: "bbc-model-b", supportFiles: { "AGENTS.md": "User-owned BBC guidance" }, manifest: 'schema_version = 1\nname = "My-game"\ntarget = "bbc-model-b"\nsource = "src/basic/main.bas"\n' };
  expect((await worker.fetch(request("save", cookie, { repository: "me/private", id: ID, sha: SHA, project }), env)).status).toBe(200);
  expect(JSON.parse(atob(String(writes[0]?.body.content)))).toEqual(project);
  expect((await worker.fetch(request("save", cookie, { repository: "me/private", id: ID, sha: remoteSha, project: { ...project, roms: [1] } }), env)).status).toBe(400);
  expect(writes).toHaveLength(1);
});
it("rejects stale revisions, public repositories, cross-origin writes, traversal and extra payload fields without writing", async () => {
  const env = environment(); const cookie = await login(env);
  for (const [body, from, status] of [
    [{ repository: "me/private", id: ID, sha: "c".repeat(40), project: PROJECT }, SITE, 409],
    [{ repository: "me/public", id: ID, sha: SHA, project: PROJECT }, SITE, 403],
    [{ repository: "me/private", id: ID, sha: SHA, project: PROJECT }, "https://attacker.example", 403],
    [{ repository: "../private", id: ID, sha: SHA, project: PROJECT }, SITE, 400],
    [{ repository: "me/private", id: ID, sha: SHA, project: { ...PROJECT, firmware: [1] } }, SITE, 400],
    [{ repository: "me/private", id: ID, sha: SHA, project: PROJECT, path: "secret" }, SITE, 400],
  ] as const) {
    const response = await worker.fetch(request("save", cookie, body, from), env); expect(response.status).toBe(status); expect(await response.text()).not.toMatch(/CANARY_PRIVATE/);
  }
  expect(writes).toEqual([]);
});
it("rejects missing or tampered state, unauthenticated access, expired sessions and unconfigured service", async () => {
  const env = environment();
  expect((await worker.fetch(request("repositories"), env)).status).toBe(401);
  expect((await worker.fetch(request("callback?state=wrong&code=code", "__Host-eightbit-oauth=tampered"), env)).status).toBe(401);
  expect(vi.mocked(fetch)).not.toHaveBeenCalled();
  const cookie = await login(env);
  vi.useFakeTimers(); vi.setSystemTime(Date.now() + 31 * 86400_000);
  expect((await worker.fetch(request("repositories", cookie), env)).status).toBe(401);
  vi.useRealTimers();
  const response = await worker.fetch(request("repositories"), { ...env, GITHUB_CLIENT_SECRET: "" }); expect(response.status).toBe(503); expect(await response.text()).not.toMatch(/CANARY_PRIVATE/);
});
it("serialises refresh rotation across concurrent requests and removes credentials at alarm expiry", async () => {
  const env = environment(); const cookie = await login(env);
  vi.useFakeTimers(); vi.setSystemTime(Date.now() + 9 * 3600_000);
  const responses = await Promise.all([worker.fetch(request("repositories", cookie), env), worker.fetch(request("repositories", cookie), env)]);
  expect(responses.every((response) => response.ok)).toBe(true);
  expect(vi.mocked(fetch).mock.calls.filter(([url]) => url === "https://github.com/login/oauth/access_token")).toHaveLength(2);
  const id = cookie.split("=")[1] ?? "";
  const stub = env.SESSIONS.get(env.SESSIONS.idFromName(id)); await stub.fetch(new Request("https://session/delete", { method: "POST" }));
  expect((await worker.fetch(request("repositories", cookie), env)).status).toBe(401);
});
it("redacts failed provider responses and enforces an explicit project size limit", async () => {
  const env = environment(); const cookie = await login(env);
  const tooLarge = await worker.fetch(request("save", cookie, { repository: "me/private", id: ID, sha: SHA, project: { ...PROJECT, source: "x".repeat(1024 * 1024 + 1) } }), env); expect(tooLarge.status).toBe(400);
  vi.stubGlobal("fetch", vi.fn(async () => json({ message: SECRET, access_token: ACCESS }, 500)));
  const response = await worker.fetch(request("repositories", cookie), env); expect(response.status).toBe(503); expect(await response.text()).not.toMatch(/CANARY_PRIVATE/);
});

it("rejects OAuth and API redirects without following or exposing provider data", async () => {
  const env = environment(); const cookie = await login(env);
  const redirect = vi.fn(async (_url: string, init?: RequestInit) => {
    expect(init?.redirect).toBe("manual");
    return new Response(SECRET, { status: 302, headers: { Location: `https://attacker.example/${ACCESS}` } });
  });
  vi.stubGlobal("fetch", redirect);
  const api = await worker.fetch(request("repositories", cookie), env);
  expect(api.status).toBe(503); expect(await api.text()).not.toMatch(/CANARY_PRIVATE|attacker/);
  const start = await worker.fetch(request("connect"), env);
  const location = new URL(start.headers.get("location") ?? "");
  const auth = start.headers.get("set-cookie")?.split(";")[0] ?? "";
  const callback = await worker.fetch(request(`callback?state=${location.searchParams.get("state")}&code=test`, auth), env);
  expect(callback.status).toBe(503); expect(await callback.text()).not.toMatch(/CANARY_PRIVATE|attacker/);
  expect(redirect).toHaveBeenCalledTimes(2);
  expect(redirect.mock.calls.every(([url]) => new URL(url).hostname !== "attacker.example")).toBe(true);
});

it("preserves schema-2 machine profiles in explicit cloud copies and rejects unsupported identities", async () => {
  const env = environment(); const cookie = await login(env);
  for (const target of ["c64-pal", "spectrum48", "zx81-16k"]) {
    const project = { ...PROJECT, schemaVersion: 2, target, supportFiles: {} };
    const saved = await worker.fetch(request("save", cookie, { repository: "me/private", id: ID, sha: remoteSha, project }), env);
    expect(saved.status).toBe(200);
    expect(JSON.parse(atob(String(writes.at(-1)?.body.content)))).toEqual(project);
  }
  for (const project of [{ ...PROJECT, target: "c64-pal" }, { ...PROJECT, schemaVersion: 2, target: "nes", supportFiles: {} }]) {
    expect((await worker.fetch(request("save", cookie, { repository: "me/private", id: ID, sha: remoteSha, project }), env)).status).toBe(400);
  }
  expect(writes).toHaveLength(3);
});
