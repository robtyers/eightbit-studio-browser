import type { ReactNode } from "react";
import { CloseIcon } from "./Icons";

export interface EditorTab {
  path: string;
  dirty: boolean;
  badge: string;
  badgeClass: string;
}

export function EditorTabs({ tabs, activePath, onSelect, onClose, children }: {
  tabs: readonly EditorTab[];
  activePath: string | null;
  onSelect: (path: string) => void;
  onClose: (path: string) => void;
  children?: ReactNode;
}) {
  return <div className="editor-tabs" role="tablist" aria-label="Open editors">
    {tabs.map((tab) => {
      const saveStatus = tab.dirty ? "Unsaved changes" : "Saved";
      const statusId = `editor-save-status-${encodeURIComponent(tab.path)}`;
      return <div className={tab.path === activePath ? "editor-tab active" : "editor-tab"} key={tab.path}>
        <button className="editor-tab-select" type="button" role="tab" aria-selected={tab.path === activePath}
          aria-label={tab.path.split("/").at(-1)} aria-describedby={statusId} onClick={() => onSelect(tab.path)}>
          <span className={`file-badge ${tab.badgeClass}`} aria-hidden="true">{tab.badge}</span>
          {tab.path.split("/").at(-1)}
          <span className={`tab-save-status ${tab.dirty ? "unsaved" : "saved"}`} aria-hidden="true" title={saveStatus}>{tab.dirty ? "●" : "✓"}</span>
          <span id={statusId} className="visually-hidden">{saveStatus}</span>
        </button>
        <button className="editor-tab-close" type="button" aria-label={`Close ${tab.path}`} onClick={() => onClose(tab.path)}><CloseIcon size={12} /></button>
      </div>;
    })}
    {children}
  </div>;
}
