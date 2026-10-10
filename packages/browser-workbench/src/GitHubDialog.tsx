import { useEffect, useRef, useState } from "react";
import { BrowserDialog } from "./BrowserDialog";
import { GitHubError, type GitHubClient, type Repository, type RemoteProject } from "./github";
import { projectId, type CloudLink, type ProjectSnapshot } from "./projects";
export function GitHubDialog({ client, project, cloud, onClose, onOpen, onSaved, localProjectId }: {
  localProjectId?: string; client: GitHubClient; project?: ProjectSnapshot; cloud?: CloudLink; onClose: () => void;
  onOpen: (project: RemoteProject, repository: string) => void;
  onSaved: (link: CloudLink, source: string) => void;
}) {
  const [repositories, setRepositories] = useState<Repository[]>([]);
  const [repository, setRepository] = useState(cloud?.repository ?? "");
  const [nextPage, setNextPage] = useState<number>();
  const [projects, setProjects] = useState<{ id: string; name: string }[]>([]);
  const [message, setMessage] = useState("Connecting to your saved GitHub session…");
  const [configured, setConfigured] = useState(false);
  const [needsLogin, setNeedsLogin] = useState(false);
  const [busy, setBusy] = useState(false);
  const pending = useRef<AbortController | undefined>(undefined);
  const alive = useRef(true);
  async function request<T>(operation: (signal: AbortSignal) => Promise<T>, accept: (value: T) => void) {
    pending.current?.abort(); const controller = new AbortController(); pending.current = controller; setBusy(true);
    try { const value = await operation(controller.signal); if (alive.current && !controller.signal.aborted) accept(value); }
    catch (error: unknown) { if (alive.current && !controller.signal.aborted) { setConfigured(error instanceof GitHubError && error.status === 401); setNeedsLogin(error instanceof GitHubError && error.status === 401); setMessage(error instanceof Error ? error.message : "GitHub could not complete the request. Your local work is preserved."); } }
    finally { if (alive.current && pending.current === controller) { pending.current = undefined; setBusy(false); } }
  }
  useEffect(() => {
    alive.current = true;
    void request((signal) => client.repositories(signal), (data) => { setConfigured(true); setRepositories(data.repositories); setNextPage(data.nextPage); setMessage(data.repositories.length ? "Choose a private repository you authorised for Eightbit Studio." : "No private repositories are authorised yet. Install the GitHub App on a private repository, then reconnect."); });
    return () => { alive.current = false; pending.current?.abort(); };
  }, [client]);
  function choose(name: string) { setRepository(name); setProjects([]); if (name) void request((signal) => client.list(name, signal), (items) => { setProjects(items); setMessage(items.length ? "Choose a project to open a local copy, or save your current project." : "This repository has no browser projects yet. You can save your current project here."); }); }
  function save(copy = false) {
    if (!project || !repository) return;
    const same = !copy && cloud?.repository === repository;
    const id = same ? cloud.projectId : projectId();
    const source = project.source;
    setMessage("Saving this project snapshot to GitHub…");
    void request((signal) => client.save(repository, id, same ? cloud.sha : null, project, signal), (receipt) => { onSaved({ repository, projectId: id, sha: receipt.sha }, source); setMessage("Project saved to GitHub. Later edits stay local until you save again."); });
  }
  const connectUrl = `/studio-api/connect${localProjectId ? `?project=${encodeURIComponent(localProjectId)}` : ""}`;
  return <BrowserDialog title="Continue with GitHub" onClose={onClose}>
    <p>GitHub keeps a copy you can open on another device. Your code is sent only when you choose Save to GitHub; ROMs and game files stay on your device.</p>
    {project && <div className="browser-cloud-preview"><strong>{project.name}</strong><small>{project.kind === "basic" ? "Dragon 32 BASIC" : "Recovered assembly"} · source, settings and project guidance</small></div>}
    {repositories.length > 0 && <label className="browser-field">Private repository<select aria-label="Private repository" disabled={busy} value={repository} onChange={(event) => choose(event.currentTarget.value)}><option value="">Choose a repository…</option>{repositories.map((item) => <option key={item.name} value={item.name}>{item.name}</option>)}</select></label>}
    {nextPage && <button type="button" disabled={busy} onClick={() => { void request((signal) => client.moreRepositories(nextPage, signal), (data) => { setRepositories((previous) => [...previous, ...data.repositories]); setNextPage(data.nextPage); }); }}>More repositories</button>}
    {repository && <button type="button" disabled={busy} onClick={() => choose(repository)}>Refresh saved projects</button>}
    {projects.map((item) => <button className="browser-remote-project" type="button" key={item.id} disabled={busy} onClick={() => { void request((signal) => client.open(repository, item.id, signal), (value) => onOpen(value, repository)); }}>Open {item.name}</button>)}
    {project && repository && <div className="browser-welcome-actions"><button className="primary-button" type="button" disabled={busy} onClick={() => save()}>Save to GitHub</button>{cloud?.repository === repository && <button type="button" disabled={busy} onClick={() => save(true)}>Save a separate copy</button>}</div>}
    <p role="status">{message}</p>
    {busy && <button type="button" onClick={() => { pending.current?.abort(); setBusy(false); setMessage("Stopped waiting. Your local work is safe. A save already received by GitHub may have completed; open the remote copy before retrying."); }}>Cancel request</button>}
    {needsLogin && <a className="browser-connect-link" href={connectUrl}>Connect GitHub</a>}
    {configured && !busy && <p><a href="/studio-api/install" target="_blank" rel="noopener noreferrer">Install the App on a private repository</a></p>}
    {configured && !busy && <a href={connectUrl}>Connect or change authorised repositories</a>}
    {!needsLogin && repositories.length > 0 && <button type="button" disabled={busy} onClick={() => { void request((signal) => client.disconnect(signal), () => { setRepositories([]); setRepository(""); setProjects([]); setNeedsLogin(true); setMessage("Disconnected on this device. Your local projects remain available."); }); }}>Disconnect this device</button>}
    <p className="browser-muted">Use an initialised private repository (create it with a README). GitHub permissions and installation are a one-time setup per account. Local recovery and project downloads work without an account.</p>
  </BrowserDialog>;
}
