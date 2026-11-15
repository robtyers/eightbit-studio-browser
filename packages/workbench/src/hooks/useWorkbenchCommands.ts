import { useEffect, useRef } from "react";
import type { EditorCommandId } from "../services/editorCommands";

// Only app-owned command IDs and callbacks are registered. No event carries code,
// source, terminal input or an arbitrary selector across the native boundary.
const handlers = new Map<EditorCommandId, () => void>();
export function invokeWorkbenchCommand(id: EditorCommandId): boolean {
  const handler = handlers.get(id);
  if (!handler) return false;
  handler();
  return true;
}
export function useWorkbenchCommands(commands: Partial<Record<EditorCommandId, () => void>>) {
  const latest = useRef(commands);
  latest.current = commands;
  const keys = Object.keys(commands).sort().join(",");
  useEffect(() => {
    const owned = new Map<EditorCommandId, () => void>();
    for (const id of Object.keys(latest.current) as EditorCommandId[]) {
      const callback = () => latest.current[id]?.();
      owned.set(id, callback);
      handlers.set(id, callback);
    }
    return () => { for (const [id, callback] of owned) if (handlers.get(id) === callback) handlers.delete(id); };
  }, [keys]);
}
