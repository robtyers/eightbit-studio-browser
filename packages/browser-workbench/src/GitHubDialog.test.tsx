import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { GitHubDialog } from "./GitHubDialog";
import { GitHubError, type GitHubClient } from "./github";
const project = { schemaVersion: 1 as const, name: "Game", kind: "basic" as const, source: "10 END", manifest: "manifest", supportFiles: {} };
function client(): GitHubClient { return { repositories: vi.fn(async () => ({ repositories: [{ name: "me/private" }] })), moreRepositories: vi.fn(async () => ({ repositories: [] })), list: vi.fn(async () => []), save: vi.fn(async () => ({ sha: "a".repeat(40) })), open: vi.fn(), disconnect: vi.fn() }; }
it("shows connection failures without sending source and saves only after an explicit click", async () => {
  const github = client(); const saved = vi.fn();
  render(<GitHubDialog client={github} project={project} onClose={vi.fn()} onOpen={vi.fn()} onSaved={saved} />);
  await screen.findByText("Choose a private repository you authorised for Eightbit Studio.");
  expect(github.save).not.toHaveBeenCalled();
  fireEvent.change(screen.getByLabelText("Private repository"), { target: { value: "me/private" } });
  await screen.findByText(/has no browser projects/);
  fireEvent.click(screen.getByRole("button", { name: "Save to GitHub" }));
  await screen.findByText(/Project saved to GitHub/);
  expect(github.save).toHaveBeenCalledWith("me/private", expect.any(String), null, project, expect.any(AbortSignal));
  expect(saved).toHaveBeenCalledWith(expect.objectContaining({ repository: "me/private", sha: "a".repeat(40) }), project);
});
it("preserves conflicts and ignores late saves after cancellation or unmount", async () => {
  const github = client(); const saved = vi.fn(); let resolve: ((value: { sha: string }) => void) | undefined;
  const cloud = { repository: "me/private", projectId: crypto.randomUUID(), sha: "a".repeat(40) };
  vi.mocked(github.save).mockRejectedValueOnce(new GitHubError("Remote copy changed; your work is preserved", 409)).mockImplementationOnce(() => new Promise((done) => { resolve = done; }));
  const view = render(<GitHubDialog client={github} project={project} cloud={cloud} onClose={vi.fn()} onOpen={vi.fn()} onSaved={saved} />);
  await screen.findByText(/Choose a private repository/);
  fireEvent.click(screen.getByRole("button", { name: "Save to GitHub" })); await screen.findByText(/Remote copy changed/);
  expect(saved).not.toHaveBeenCalled();
  fireEvent.click(screen.getByRole("button", { name: "Save to GitHub" })); await waitFor(() => expect(github.save).toHaveBeenCalledTimes(2));
  fireEvent.click(screen.getByRole("button", { name: "Cancel request" }));
  view.unmount(); await act(async () => resolve?.({ sha: "b".repeat(40) })); expect(saved).not.toHaveBeenCalled();
});

it("reports the exact multi-source snapshot acknowledged by a delayed save", async () => {
  const github = client(), saved = vi.fn();
  let done: ((value: { sha: string }) => void) | undefined;
  vi.mocked(github.save).mockImplementation(() => new Promise(resolve => { done = resolve; }));
  const original = { ...project, kind: "assembly" as const, supportFiles: { "asm/data.asm": "fcb 1" } };
  const props = { client: github, project: original, cloud: { repository: "me/private", projectId: crypto.randomUUID(), sha: "a".repeat(40) }, onClose: vi.fn(), onOpen: vi.fn(), onSaved: saved };
  const view = render(<GitHubDialog {...props} />);
  await screen.findByText(/Choose a private repository/);
  fireEvent.click(screen.getByRole("button", { name: "Save to GitHub" }));
  await waitFor(() => expect(github.save).toHaveBeenCalledOnce());
  view.rerender(<GitHubDialog {...props} project={{ ...original, supportFiles: { "asm/data.asm": "fcb 2" } }} />);
  await act(async () => done?.({ sha: "b".repeat(40) }));
  expect(saved).toHaveBeenCalledWith(expect.anything(), original);
});
