
const MAX_RECENT_PROJECTS = 5;

export interface RecentProject {
  name: string;
  root: string;
  targetLabel?: string;
}


/** Bounded recent-project persistence with an application-owned namespace. */
export function createRecentProjectsStore(STORAGE_KEY: string) {
function loadRecentProjects(): RecentProject[] {
  try {
    const parsed: unknown = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "[]");
    if (!Array.isArray(parsed)) {
      return [];
    }
    return parsed
      .filter(isRecentProject)
      .slice(0, MAX_RECENT_PROJECTS);
  } catch {
    return [];
  }
}

function rememberProject(project: RecentProject): RecentProject[] {
  const recent = [
    project,
    ...loadRecentProjects().filter((item) => item.root !== project.root),
  ].slice(0, MAX_RECENT_PROJECTS);
  localStorage.setItem(STORAGE_KEY, JSON.stringify(recent));
  return recent;
}

function clearRecentProjects(): RecentProject[] {
  try {
    localStorage.removeItem(STORAGE_KEY);
  } catch {
    // Recent-project persistence is a convenience and must not block the application.
  }
  return [];
}

function isRecentProject(value: unknown): value is RecentProject {
  if (!value || typeof value !== "object") {
    return false;
  }
  const candidate = value as Partial<RecentProject>;
  return (
    typeof candidate.name === "string" &&
    candidate.name.length > 0 &&
    candidate.name.length <= 64 &&
    !hasControlCharacters(candidate.name) &&
    typeof candidate.root === "string" &&
    candidate.root.startsWith("/") &&
    candidate.root.length <= 4096 &&
    !hasControlCharacters(candidate.root) &&
    (candidate.targetLabel === undefined || (typeof candidate.targetLabel === "string" &&
      candidate.targetLabel.length > 0 && candidate.targetLabel.length <= 160 && !hasControlCharacters(candidate.targetLabel)))
  );
}

function hasControlCharacters(value: string): boolean {
  return Array.from(value).some((character) => {
    const codePoint = character.codePointAt(0) ?? 0;
    return codePoint <= 0x1f || (codePoint >= 0x7f && codePoint <= 0x9f);
  });
}

return { loadRecentProjects, rememberProject, clearRecentProjects };
}
