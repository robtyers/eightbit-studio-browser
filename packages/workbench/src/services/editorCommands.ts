import catalogue from "./commands.json";

export type EditorCommandId = keyof typeof catalogue;
export type EditorCommandAvailability = Record<EditorCommandId, boolean>;
export const EDITOR_COMMANDS = Object.entries(catalogue).filter(([, command]) => !command.hidden).map(([key, command]) => ({
  id: key as EditorCommandId,
  label: [...command.path, command.label].join(": "),
  shortcut: command.accelerator.replaceAll("CmdOrCtrl", "⌘").replaceAll("Alt", "⌥").replaceAll("Shift", "⇧").replaceAll("+", ""),
  aliases: command.aliases,
}));
export function isEditorCommandId(value: unknown): value is EditorCommandId {
  return typeof value === "string" && Object.hasOwn(catalogue, value);
}
