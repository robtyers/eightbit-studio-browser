import type { RecentProject } from "../services/recentProjects";
import type { ReactNode } from "react";

export interface WelcomeViewProps {
  logo: ReactNode;
  headline: string;
  copy: string;
  footnote?: string;
  versionLabel?: string | null;
  compact?: boolean;
  recent: RecentProject[];
  busy: boolean;
  onOpen: () => void;
  onCreate: () => void;
  onClone?: () => void;
  onSettings: () => void;
  onRecent: (project: RecentProject) => void;
  onClearRecent: () => void;
  actionLabels?: Partial<Record<"open" | "create" | "settings" | "clearRecent", string>>;
  recentCaption?: (project: RecentProject) => string;
}

export function WelcomeView({ logo, headline, copy, footnote, versionLabel = "VERSION 0.5 · PREVIEW", compact = false, recent, busy, onOpen, onCreate, onClone, onSettings, onRecent, onClearRecent, actionLabels, recentCaption }: WelcomeViewProps) {
  return (
    <main className={`welcome-view${compact ? " welcome-view-compact" : ""}`}>
      {logo}
      {versionLabel && <p className="eyebrow">{versionLabel}</p>}
      <h1>{headline}</h1>
      <p className="welcome-copy">
        {copy}
      </p>
      <div className="welcome-actions">
        <button className="primary-button" type="button" onClick={onOpen} disabled={busy}>
          {actionLabels?.open ?? "Open Project…"}
        </button>
        <button className="secondary-button" type="button" onClick={onCreate} disabled={busy}>
          {actionLabels?.create ?? "Create Project…"}
        </button>
        <button className="secondary-button" type="button" onClick={onSettings} disabled={busy}>
          {actionLabels?.settings ?? "Settings…"}
        </button>
        {onClone && <button className="secondary-button" type="button" onClick={onClone} disabled={busy}>Clone Repository…</button>}
      </div>
      {recent.length > 0 && (
        <section className="recent-projects" aria-labelledby="recent-heading">
          <div className="recent-projects-heading">
            <h2 id="recent-heading">Recent projects</h2>
            <button className="recent-projects-clear" type="button" onClick={onClearRecent} disabled={busy}>
              {actionLabels?.clearRecent ?? "Clear recent projects"}
            </button>
          </div>
          {recent.map((project) => (
            <button className="recent-project-button" key={project.root} type="button" onClick={() => onRecent(project)} disabled={busy}>
              <span>{project.name}{project.targetLabel && <small className="recent-project-target">{project.targetLabel}</small>}</span>
              <small>{recentCaption ? recentCaption(project) : project.root}</small>
            </button>
          ))}
        </section>
      )}
      {footnote && <p className="welcome-footnote">{footnote}</p>}
    </main>
  );
}
