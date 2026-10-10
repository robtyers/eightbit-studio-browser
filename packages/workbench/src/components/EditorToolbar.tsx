import { useEffect, useRef, useState, type ReactNode } from "react";
import type { EditorCommandAvailability, EditorCommandId } from "../services/editorCommands";
import { BuildIcon, BugIcon, PlayIcon, SaveIcon, StopIcon } from "./Icons";

interface EditorToolbarProps {
  children: ReactNode;
  enabled: Pick<EditorCommandAvailability, "save" | "build" | "run" | "stop" | "debug">;
  sessionActive: boolean;
  supportsDebug?: boolean;
  labels?: Partial<Record<"save" | "build" | "run", string>>;
  keepSaveVisible?: boolean;
  onCommand: (command: EditorCommandId) => void;
}

export function EditorToolbar({ children, enabled, sessionActive, supportsDebug = true, labels, keepSaveVisible = false, onCommand }: EditorToolbarProps) {
  const saveLabel = labels?.save ?? "Save";
  const buildLabel = labels?.build ?? "Build";
  const runLabel = labels?.run ?? "Run Project";
  const commandKey = /Macintosh|Mac OS X/.test(navigator.userAgent) ? "⌘" : "Ctrl+";
  const barRef = useRef<HTMLDivElement>(null);
  const moreRef = useRef<HTMLButtonElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const [compact, setCompact] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);

  useEffect(() => {
    const bar = barRef.current;
    if (!bar || typeof ResizeObserver === "undefined") return;
    // Observe the editor, not the window: either dock can consume its width.
    const observer = new ResizeObserver(([entry]) => {
      if (!entry) return;
      setCompact(entry.contentRect.width < 440);
      setMenuOpen(false);
    });
    observer.observe(bar);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    if (!menuOpen) return;
    menuRef.current?.querySelector<HTMLButtonElement>("button:not(:disabled)")?.focus();
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

  return <div ref={barRef} className={`command-bar editor-toolbar${compact ? " compact" : ""}`}>
    <div className="breadcrumbs">{children}</div>
    <div id="editor-command-actions" className="command-actions" role="group" aria-label="Editor actions">
      {(!compact || keepSaveVisible) && <button type="button" onClick={() => onCommand("save")} disabled={!enabled.save} title={`${saveLabel} (${commandKey}S)`}><SaveIcon /> {saveLabel}</button>}
      <button className="build-button" type="button" onClick={() => onCommand("build")} disabled={!enabled.build} title={`${buildLabel} (${commandKey}B)`}><BuildIcon /> {buildLabel}</button>
      <button className="run-button" type="button" onClick={() => onCommand("run")} disabled={!enabled.run} title={`${labels?.run ?? "Run"} (${commandKey}R)`}><PlayIcon /> {runLabel}</button>
      {sessionActive && <button className="stop-button" type="button" onClick={() => onCommand("stop")} disabled={!enabled.stop} title="Stop"><StopIcon /> Stop</button>}
      {supportsDebug && !compact && <button className="debug-button" type="button" onClick={() => onCommand("debug")} disabled={!enabled.debug} title="Debug (F5)"><BugIcon /> Debug</button>}
      {compact && (!keepSaveVisible || supportsDebug) && <div className="toolbar-overflow">
        <button ref={moreRef} type="button" aria-label="More editor actions" aria-haspopup="menu" aria-expanded={menuOpen} aria-controls={menuOpen ? "editor-overflow-menu" : undefined} onClick={() => setMenuOpen((open) => !open)} onKeyDown={(event) => {
          if (event.key === "Escape") { event.preventDefault(); setMenuOpen(false); }
          if (event.key === "ArrowDown" || event.key === "ArrowUp") { event.preventDefault(); setMenuOpen(true); }
        }}>•••</button>
        {menuOpen && <div ref={menuRef} id="editor-overflow-menu" className="toolbar-overflow-menu" role="menu" aria-label="More editor actions" onKeyDown={(event) => {
          if (event.key === "Escape" || event.key === "Tab") { closeMenu(); if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); } return; }
          const items = Array.from(menuRef.current?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? []);
          const index = items.findIndex((item) => item === document.activeElement);
          const next = event.key === "Home" ? 0 : event.key === "End" ? items.length - 1 : event.key === "ArrowDown" ? (index + 1) % items.length : event.key === "ArrowUp" ? (index - 1 + items.length) % items.length : undefined;
          if (next !== undefined) { event.preventDefault(); items[next]?.focus(); }
        }}>
          {!keepSaveVisible && <button type="button" role="menuitem" disabled={!enabled.save} onClick={() => { closeMenu(); onCommand("save"); }}><SaveIcon /> {saveLabel} <kbd>{commandKey}S</kbd></button>}
          {supportsDebug && <button type="button" role="menuitem" disabled={!enabled.debug} onClick={() => { closeMenu(); onCommand("debug"); }}><BugIcon /> Debug <kbd>F5</kbd></button>}
        </div>}
      </div>}
    </div>
  </div>;
}
