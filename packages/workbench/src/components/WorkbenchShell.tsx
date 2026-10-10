import { useEffect, useState, type CSSProperties, type ReactNode } from "react";
import { WorkbenchResizeHandle } from "./WorkbenchResizeHandle";
import { WorkbenchPanelTabs, type WorkbenchPanelTab } from "./WorkbenchPanelTabs";
import { constrainWorkbenchLayout, DEFAULT_WORKBENCH_LAYOUT, loadWorkbenchLayout, saveWorkbenchLayout, workbenchLayoutLimits, WORKBENCH_LAYOUT_MIN, type WorkbenchLayout } from "../services/workbenchLayout";

export interface WorkbenchShellProps {
  sidebarOpen: boolean;
  activity: ReactNode;
  sidebar: ReactNode;
  toolbar: ReactNode;
  editor: ReactNode;
  panel: {
    open: boolean;
    tabs: readonly WorkbenchPanelTab[];
    active: string;
    onSelect: (id: string) => void;
    onClose: () => void;
    label: string;
    content: ReactNode;
  };
  assistant?: ReactNode;
  status: ReactNode;
  children?: ReactNode;
}

/** The established application layout is invariant across targets. Families supply content, not a replacement shell. */
export function WorkbenchShell({ sidebarOpen, activity, sidebar, toolbar, editor, panel, assistant, status, children }: WorkbenchShellProps) {
  const [layout, setLayout] = useState(loadWorkbenchLayout);
  const [viewport, setViewport] = useState(() => ({ width: window.innerWidth, height: window.innerHeight }));
  const assistantOpen = Boolean(assistant);
  const visible = constrainWorkbenchLayout(layout, viewport, assistantOpen);
  const limits = workbenchLayoutLimits(visible, viewport, assistantOpen);
  useEffect(() => { saveWorkbenchLayout(layout); }, [layout]);
  useEffect(() => {
    const resize = () => setViewport({ width: window.innerWidth, height: window.innerHeight });
    window.addEventListener("resize", resize);
    return () => window.removeEventListener("resize", resize);
  }, []);
  const style = {
    "--primary-sidebar-width": `${visible.primarySidebarWidth}px`,
    "--assistant-width": `${visible.assistantWidth}px`,
    "--bottom-panel-height": `${visible.bottomPanelHeight}px`,
  } as CSSProperties;
  function resizer(key: keyof WorkbenchLayout, label: string, controls: string, className: string, maximum: number, pointerFactor: 1 | -1) {
    return <WorkbenchResizeHandle className={className} label={label} controls={controls}
      orientation={key === "bottomPanelHeight" ? "horizontal" : "vertical"}
      value={visible[key]} minimum={WORKBENCH_LAYOUT_MIN[key]} maximum={maximum} pointerFactor={pointerFactor}
      onChange={(value) => setLayout((current) => ({ ...current, [key]: value }))}
      onReset={() => setLayout((current) => ({ ...current, [key]: DEFAULT_WORKBENCH_LAYOUT[key] }))} />;
  }
  return <div className={`app workbench ${assistantOpen ? "assistant-visible" : ""} ${sidebarOpen ? "" : "sidebar-hidden"}`} style={style}>
    <nav className="activity-bar" aria-label="Workbench views">{activity}</nav>
    {sidebar}
    {resizer("primarySidebarWidth", "Resize primary side bar", "primary-sidebar", "primary-sidebar-resizer", limits.primarySidebarMax, 1)}
    <main className="editor-area" aria-label="Project editor">
      {toolbar}
      {editor}
      <section id="workbench-bottom-panel" className={panel.open ? "bottom-panel" : "bottom-panel closed"} aria-label="Workbench panel">
        {resizer("bottomPanelHeight", "Resize bottom panel", "workbench-bottom-panel", "bottom-panel-resizer", limits.bottomPanelMax, -1)}
        <WorkbenchPanelTabs tabs={panel.tabs} active={panel.active} onSelect={panel.onSelect} onClose={panel.onClose} />
        <div id="workbench-panel-content" className="panel-content" role="tabpanel" aria-label={panel.label}>{panel.content}</div>
      </section>
    </main>
    {assistantOpen && <>{resizer("assistantWidth", "Resize AI Agent", "assistant-sidebar", "assistant-resizer", limits.assistantMax, -1)}{assistant}</>}
    <footer className="status-bar">{status}</footer>
    {children}
  </div>;
}
