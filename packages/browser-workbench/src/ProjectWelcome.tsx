import { WelcomeView } from "@eightbitstudio/workbench/components/WelcomeView";
import { ProcessorLogo } from "@eightbitstudio/workbench/components/ProcessorLogo";
import type { StoredProject } from "./projects";
export function ProjectWelcome({ recent, busy, onCreate, onOpen, onRecent, onSettings, onGitHub, message, machines }: {
  machines: import("./contracts").BrowserMachine[]; recent: StoredProject[]; busy: boolean; onCreate: () => void; onOpen: () => void;
  onRecent: (project: StoredProject) => void; onSettings: () => void; onGitHub: () => void; message: string;
}) {
  return <div className="browser-project-welcome">
    <WelcomeView logo={<ProcessorLogo className="browser-brand" wordmark />}
      headline="Get Started" copy="Create a project or pick up where you left off. Build, test and play on your device."
      versionLabel="6809 · 6502 · Z80 · BROWSER EDITION" compact recent={recent.map((project) => ({ name: project.project.name, root: project.id, targetLabel: machines.find((machine) => machine.id === (project.project.target ?? "dragon32"))?.label ?? "Unknown computer" }))}
      recentCaption={(value) => { const item = recent.find((entry) => entry.id === value.root); return `On this device${item?.cloud ? " · GitHub copy available" : ""}`; }}
      actionLabels={{ open: "Load existing project", create: "Create new project", settings: "Computer setup", clearRecent: "About local recovery" }}
      busy={busy} onCreate={onCreate} onOpen={onOpen} onSettings={onSettings}
      onRecent={(value) => { const project = recent.find((entry) => entry.id === value.root); if (project) onRecent(project); }}
      onClearRecent={() => onOpen()} footnote="Projects recover automatically in this browser. Download a backup or save to GitHub to keep another copy." />
    <div className="browser-cloud-card"><div><strong>Continue on another device</strong><p>Save a copy to your private GitHub repository, then open it here next time.</p></div><button type="button" disabled={busy} onClick={onGitHub}>Open from GitHub</button></div>
    {message && <p className="browser-welcome-status" role="status">{message}</p>}
  </div>;
}
