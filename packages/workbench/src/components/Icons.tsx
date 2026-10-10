interface IconProps {
  size?: number;
}

const shared = {
  fill: "none",
  stroke: "currentColor",
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
  strokeWidth: 1.8,
};

export function FilesIcon({ size = 20 }: IconProps) {
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" {...shared}><path d="M6 2.8h7l5 5V21H6z"/><path d="M13 2.8V8h5"/></svg>;
}

export function BugIcon({ size = 20 }: IconProps) {
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" {...shared}><path d="M8 9h8v7a4 4 0 0 1-8 0z"/><path d="m9 6 3-3 3 3M5 12h3m8 0h3M5 17h3m8 0h3"/></svg>;
}

export function BuildIcon({ size = 18 }: IconProps) {
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" {...shared}><path d="m14.5 5.5 4-2 2 2-2 4-4 1-5 8-4-4 8-5z"/><path d="m5.5 14.5-2 6 6-2"/></svg>;
}

export function RefreshIcon({ size = 18 }: IconProps) {
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" {...shared}><path d="M20 11a8 8 0 0 0-14.5-4L3 10"/><path d="M3 5v5h5M4 13a8 8 0 0 0 14.5 4L21 14"/><path d="M21 19v-5h-5"/></svg>;
}

export function SaveIcon({ size = 18 }: IconProps) {
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" {...shared}><path d="M4 3h13l3 3v15H4z"/><path d="M8 3v6h8V3M8 21v-7h8v7"/></svg>;
}

export function WarningIcon({ size = 16 }: IconProps) {
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" {...shared}><path d="M12 3 2 21h20z"/><path d="M12 9v5m0 3v.2"/></svg>;
}

export function CloseIcon({ size = 16 }: IconProps) {
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" {...shared}><path d="m6 6 12 12M18 6 6 18"/></svg>;
}

export function PlayIcon({ size = 17 }: IconProps) {
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" {...shared}><path d="m8 5 11 7-11 7z"/></svg>;
}

export function StopIcon({ size = 16 }: IconProps) {
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" {...shared}><path d="M6 6h12v12H6z"/></svg>;
}

export function FocusIcon({ size = 16 }: IconProps) {
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" {...shared}><path d="M8 3H3v5m13-5h5v5M8 21H3v-5m13 5h5v-5"/><circle cx="12" cy="12" r="3"/></svg>;
}

export function AssistantIcon({ size = 20 }: IconProps) {
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" {...shared}><path d="m12 2 1.6 5.2L19 9l-5.4 1.8L12 16l-1.6-5.2L5 9l5.4-1.8z"/><path d="m18.5 15 .8 2.7L22 18.5l-2.7.8-.8 2.7-.8-2.7-2.7-.8 2.7-.8z"/></svg>;
}

export function SourceControlIcon({ size = 20 }: IconProps) {
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" {...shared}><circle cx="6" cy="5" r="2"/><circle cx="6" cy="19" r="2"/><circle cx="18" cy="7" r="2"/><path d="M6 7v10m2-9c5 0 8 1 8 5v2"/></svg>;
}

export function TerminalIcon({ size = 20 }: IconProps) {
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" {...shared}><rect x="3" y="4" width="18" height="16" rx="2"/><path d="m7 9 3 3-3 3m6 0h4"/></svg>;
}

export function SettingsIcon({ size = 20 }: IconProps) {
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" {...shared}><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1-2.8 2.8-.1-.1a1.7 1.7 0 0 0-1.9-.3 1.7 1.7 0 0 0-1 1.6v.2h-4V21a1.7 1.7 0 0 0-1-1.6 1.7 1.7 0 0 0-1.9.3l-.1.1L4.2 17l.1-.1a1.7 1.7 0 0 0 .3-1.9A1.7 1.7 0 0 0 3 14H2.8v-4H3a1.7 1.7 0 0 0 1.6-1 1.7 1.7 0 0 0-.3-1.9L4.2 7 7 4.2l.1.1A1.7 1.7 0 0 0 9 4.6a1.7 1.7 0 0 0 1-1.6v-.2h4V3a1.7 1.7 0 0 0 1 1.6 1.7 1.7 0 0 0 1.9-.3l.1-.1L19.8 7l-.1.1a1.7 1.7 0 0 0-.3 1.9 1.7 1.7 0 0 0 1.6 1h.2v4H21a1.7 1.7 0 0 0-1.6 1z"/></svg>;
}

export function HistoryIcon({ size = 16 }: IconProps) {
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" {...shared}><path d="M4 7v5h5"/><path d="M5.5 12a7 7 0 1 0 2-5L4 10"/><path d="M12 8v4l3 2"/></svg>;
}

export function TestsIcon({ size = 16 }: IconProps) {
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" {...shared}><path d="M9 3h6M10 3v5l-5 9a2.5 2.5 0 0 0 2.2 4h9.6a2.5 2.5 0 0 0 2.2-4l-5-9V3"/><path d="M8 15h8"/></svg>;
}

export function RenumberIcon({ size = 16 }: IconProps) {
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" {...shared}><path d="M9 6h11M9 12h11M9 18h11"/><path d="M4 5h1v3M4 12h2l-2 3h2M4 18h2l-2 3h2"/></svg>;
}

export function ImportIcon({ size = 16 }: IconProps) {
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" {...shared}><path d="M12 3v12m-4-4 4 4 4-4"/><path d="M4 17v4h16v-4"/></svg>;
}

export function ExportIcon({ size = 16 }: IconProps) {
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" {...shared}><path d="M12 15V3m-4 4 4-4 4 4"/><path d="M4 17v4h16v-4"/></svg>;
}

export function RecordIcon({ size = 16 }: IconProps) {
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" {...shared}><circle cx="12" cy="12" r="7"/><circle cx="12" cy="12" r="3" fill="currentColor" stroke="none"/></svg>;
}

export function DiskIcon({ size = 16 }: IconProps) {
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" {...shared}><circle cx="12" cy="12" r="9"/><circle cx="12" cy="12" r="3"/><path d="M12 3v3m0 12v3M3 12h3m12 0h3"/></svg>;
}

export function CassetteIcon({ size = 16 }: IconProps) {
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" {...shared}><rect x="3" y="5" width="18" height="14" rx="2"/><circle cx="8" cy="11" r="2.5"/><circle cx="16" cy="11" r="2.5"/><path d="M8 16h8l2 3H6z"/></svg>;
}

export function CartridgeIcon({ size = 16 }: IconProps) {
  return <svg aria-hidden="true" width={size} height={size} viewBox="0 0 24 24" {...shared}><path d="M6 3h12l3 3v12H3V6z"/><rect x="7" y="6" width="10" height="7" rx="1"/><path d="M6 18v3h12v-3M9 18v3m3-3v3m3-3v3"/></svg>;
}
