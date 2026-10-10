import { PROJECT_PREFIX, projectId, recentProjects, saveProject, snapshot } from "./projects";
const project = { schemaVersion: 1 as const, name: "Game", kind: "basic" as const, source: "10 END\n", manifest: "schema_version = 1", supportFiles: {} };
beforeEach(() => localStorage.clear());
it("keeps atomic versioned source-only copies, rejects a concurrent overwrite and recovers after a quota failure", async () => {
  const input = { schemaVersion: 1 as const, id: projectId(), project, entryPath: "src/main.bas" };
  const saved = await saveProject(input);
  expect(recentProjects()[0]).toEqual(saved);
  const writes = await Promise.allSettled([saveProject({ ...input, project: { ...project, source: "10 PRINT 2" } }, saved.version), saveProject(input, saved.version)]);
  expect(writes[0]?.status).toBe("fulfilled");
  expect(writes[1]?.status).toBe("rejected");
  const newer = recentProjects()[0];
  expect(newer?.project.source).toBe("10 PRINT 2");
  await expect(saveProject(input, saved.version)).rejects.toThrow("Another tab changed");
  expect(recentProjects()[0]).toEqual(newer);
  const failure = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => { throw new Error("Quota"); });
  await expect(saveProject(input, newer?.version)).rejects.toThrow("Quota"); failure.mockRestore();
  expect(recentProjects()[0]).toEqual(newer);
  expect((await saveProject(input, newer?.version)).project.source).toBe(project.source);
});
it("ignores damaged records and rejects unsupported fields and oversized sources", () => {
  localStorage.setItem(PROJECT_PREFIX + projectId(), "bad");
  expect(recentProjects()).toEqual([]);
  expect(() => snapshot({ ...project, rom: [1] })).toThrow();
  expect(() => snapshot({ ...project, schemaVersion: 2 })).toThrow();
  expect(() => snapshot({ ...project, source: "é".repeat(1024 * 1024) })).toThrow();
});
it("retains an existing copy when safe cross-tab locking is unavailable", async () => {
  const input = { schemaVersion: 1 as const, id: projectId(), project, entryPath: "src/main.bas" };
  const saved = await saveProject(input);
  Object.defineProperty(navigator, "locks", { configurable: true, value: undefined });
  await expect(saveProject({ ...input, project: { ...project, source: "10 PRINT 2" } }, saved.version)).rejects.toThrow("Safe local recovery");
  expect(recentProjects()[0]).toEqual(saved);
});

it("preserves exact schema-2 computer identity and rejects unknown or ambiguous legacy targets", async () => {
  for (const target of ["c64-pal", "spectrum16", "spectrum48", "spectrum128", "spectrum-plus2", "zx81-1k", "zx81-16k"]) {
    const next = snapshot({ ...project, schemaVersion: 2, target });
    const saved = await saveProject({ schemaVersion: 1, id: projectId(), project: next, entryPath: "src/main.bas" });
    expect(recentProjects().find((item) => item.id === saved.id)?.project.target).toBe(target);
  }
  expect(snapshot(project).target).toBeUndefined();
  expect(() => snapshot({ ...project, target: "c64-pal" })).toThrow();
  expect(() => snapshot({ ...project, schemaVersion: 2, target: "unknown" })).toThrow();
});
