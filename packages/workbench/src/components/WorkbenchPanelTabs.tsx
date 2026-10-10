import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";

export type WorkbenchPanelId = string;

export interface WorkbenchPanelTab {
  id: WorkbenchPanelId;
  label: string;
  badge?: ReactNode;
  width?: number;
}

interface WorkbenchPanelTabsProps {
  tabs: readonly WorkbenchPanelTab[];
  active: WorkbenchPanelId;
  onSelect: (panel: WorkbenchPanelId) => void;
  onClose: () => void;
}

const TAB_WIDTHS: Record<WorkbenchPanelId, number> = {
  problems: 92,
  output: 68,
  build: 112,
  tests: 78,
  symbols: 93,
  preview: 92,
  debug: 92,
  terminal: 80,
  "source-control": 110,
};
const CLOSE_WIDTH = 38;
const OVERFLOW_WIDTH = 42;
const HORIZONTAL_PADDING = 20;

export function visibleWorkbenchPanelIds(
  tabs: readonly WorkbenchPanelTab[],
  active: WorkbenchPanelId,
  width: number,
): Set<WorkbenchPanelId> {
  const allWidth = tabs.reduce((total, tab) => total + (tab.width ?? TAB_WIDTHS[tab.id] ?? 100), CLOSE_WIDTH + HORIZONTAL_PADDING);
  if (width <= 0 || allWidth <= width) return new Set(tabs.map(({ id }) => id));

  const available = Math.max(0, width - CLOSE_WIDTH - OVERFLOW_WIDTH - HORIZONTAL_PADDING);
  const visible = new Set<WorkbenchPanelId>([active]);
  let used = tabs.find((tab) => tab.id === active)?.width ?? TAB_WIDTHS[active] ?? 100;
  for (const tab of tabs) {
    if (tab.id === active || used + (tab.width ?? TAB_WIDTHS[tab.id] ?? 100) > available) continue;
    visible.add(tab.id);
    used += (tab.width ?? TAB_WIDTHS[tab.id] ?? 100);
  }
  return visible;
}

export function WorkbenchPanelTabs({ tabs, active, onSelect, onClose }: WorkbenchPanelTabsProps) {
  const tabsRef = useRef<HTMLDivElement>(null);
  const moreRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  const [menuOpen, setMenuOpen] = useState(false);
  const visibleIds = useMemo(() => visibleWorkbenchPanelIds(tabs, active, width), [active, tabs, width]);
  const hiddenTabs = tabs.filter(({ id }) => !visibleIds.has(id));

  useEffect(() => {
    const tabsElement = tabsRef.current;
    if (!tabsElement || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(([entry]) => {
      if (!entry) return;
      setWidth(entry.contentRect.width);
      setMenuOpen(false);
    });
    observer.observe(tabsElement);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    if (!menuOpen) return;
    menuRef.current?.querySelector<HTMLButtonElement>("button")?.focus();
    const closeOutside = (event: PointerEvent) => {
      if (event.target instanceof Node && !menuRef.current?.contains(event.target) && !moreRef.current?.contains(event.target)) setMenuOpen(false);
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      setMenuOpen(false);
      moreRef.current?.focus();
    };
    document.addEventListener("pointerdown", closeOutside);
    document.addEventListener("keydown", escape);
    return () => {
      document.removeEventListener("pointerdown", closeOutside);
      document.removeEventListener("keydown", escape);
    };
  }, [menuOpen]);

  function closeMenu() {
    setMenuOpen(false);
    moreRef.current?.focus();
  }

  return (
    <div ref={tabsRef} id="workbench-panel-tabs" className="panel-tabs">
      <div className="panel-tab-list" role="tablist" aria-label="Workbench panels">
        {tabs.filter(({ id }) => visibleIds.has(id)).map((tab) => (
          <button
            key={tab.id}
            type="button"
            role="tab"
            aria-selected={active === tab.id}
            aria-controls="workbench-panel-content"
            className={active === tab.id ? "active" : ""}
            onClick={() => onSelect(tab.id)}
          >
            {tab.label}{tab.badge !== undefined ? <> <span>{tab.badge}</span></> : null}
          </button>
        ))}
      </div>
      {hiddenTabs.length > 0 && (
        <div className="panel-overflow toolbar-overflow">
          <button
            ref={moreRef}
            type="button"
            aria-label="More panels"
            aria-haspopup="menu"
            aria-expanded={menuOpen}
            aria-controls={menuOpen ? "workbench-panel-overflow-menu" : undefined}
            onClick={() => setMenuOpen((open) => !open)}
            onKeyDown={(event) => {
              if (event.key === "Escape") { event.preventDefault(); setMenuOpen(false); }
              if (event.key === "ArrowDown" || event.key === "ArrowUp") { event.preventDefault(); setMenuOpen(true); }
            }}
          >•••</button>
          {menuOpen && (
            <div
              ref={menuRef}
              id="workbench-panel-overflow-menu"
              className="panel-overflow-menu toolbar-overflow-menu"
              role="menu"
              aria-label="Hidden workbench panels"
              onKeyDown={(event) => {
                if (event.key === "Escape" || event.key === "Tab") {
                  closeMenu();
                  if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); }
                  return;
                }
                const items = Array.from(menuRef.current?.querySelectorAll<HTMLButtonElement>("button") ?? []);
                const index = items.findIndex((item) => item === document.activeElement);
                const next = event.key === "Home" ? 0 : event.key === "End" ? items.length - 1 : event.key === "ArrowDown" ? (index + 1) % items.length : event.key === "ArrowUp" ? (index - 1 + items.length) % items.length : undefined;
                if (next !== undefined) { event.preventDefault(); items[next]?.focus(); }
              }}
            >
              {hiddenTabs.map((tab) => (
                <button
                  key={tab.id}
                  type="button"
                  role="menuitemradio"
                  aria-checked={active === tab.id}
                  onClick={() => { closeMenu(); onSelect(tab.id); }}
                >
                  <span>{tab.label}</span>{tab.badge !== undefined ? <> <small>{tab.badge}</small></> : null}
                </button>
              ))}
            </div>
          )}
        </div>
      )}
      <button className="panel-close" type="button" aria-label="Close panel" onClick={onClose}>×</button>
    </div>
  );
}
