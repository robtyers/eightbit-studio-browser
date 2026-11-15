import { useWorkbenchCommands } from "@eightbitstudio/workbench/hooks/useWorkbenchCommands";
import { FocusIcon, PlayIcon, StopIcon } from "@eightbitstudio/workbench/components/Icons";


export interface BasicDebugPresentation {
  state: string; message: string; stopId?: string | number | null; valuesValid: boolean; breakpointVerified: boolean;
  variables: { runtimeName: string; name: string; displayValue: string }[];
  registers?: { a: number; b: number; cc: number; dp: number; x: number; y: number; u: number; s: number; pc: number } | null;
}
export interface BasicDebugPanelProps {
  busy: boolean;
  active: boolean;
  canStart: boolean;
  description: string;
  breakpointLine: number | null;
  status: BasicDebugPresentation | null;
  onBreakpointLine: (line: number | null) => void;
  onConfigure?: () => void;
  onStart: () => void;
  onContinue: () => void;
  onFocus?: () => void;
  onTerminate: () => void;
}

export function BasicDebugPanel({
  busy,
  active, canStart, description,
  breakpointLine,
  status,
  onBreakpointLine,
  onConfigure,
  onStart,
  onContinue,
  onFocus,
  onTerminate,
}: BasicDebugPanelProps) {
  const stopped = status?.state === "stopped" && status.valuesValid;
  useWorkbenchCommands({
    breakpoints: () => document.querySelector<HTMLInputElement>(".debug-panel input")?.focus(),
    registers: () => document.querySelector<HTMLElement>('.debug-panel [aria-label="CPU Registers"]')?.focus(),
    memory: () => document.querySelector<HTMLElement>('.debug-panel [aria-label="Machine Memory"]')?.focus(),
  });
  return (
    <div className="debug-panel" aria-live="polite">
      <div className="debug-summary">
        <strong>{status?.message ?? "No BASIC debug session"}</strong>
        <label>
          BASIC breakpoint line
          <input
            aria-label="BASIC breakpoint line"
            type="number"
            min="1"
            max="63999"
            value={breakpointLine ?? ""}
            onChange={(event) => {
              const value = Number(event.target.value);
              onBreakpointLine(Number.isInteger(value) && value > 0 && value <= 63999 ? value : null);
            }}
            disabled={busy || active}
          />
        </label>
        <span>
          {description}
        </span>
        {status?.breakpointVerified && <em>● Verified line-level breakpoint</em>}
        <div className="debug-actions">
          {onConfigure && <button type="button" onClick={onConfigure} disabled={busy || active}>Configure…</button>}
          <button type="button" className="debug-control" onClick={onStart} disabled={busy || !canStart || !breakpointLine}><PlayIcon /> Debug</button>
          <button type="button" onClick={onContinue} disabled={busy || !stopped}>Continue</button>
          {onFocus && <button type="button" onClick={onFocus} disabled={busy || !active}><FocusIcon /> Show Emulator Window</button>}
          <button type="button" className="terminate-control" onClick={onTerminate} disabled={busy || !active}><StopIcon /> Stop</button>
        </div>
      </div>
      <section tabIndex={-1} className="debug-inspector" aria-label="BASIC Variables">
        <h3>BASIC Variables {status?.stopId ? <small>stop {status.stopId}</small> : null}</h3>
        {stopped && status.variables.length > 0 ? (
          <dl>{status.variables.map((variable) => (
            <div key={variable.runtimeName}><dt>{variable.name}</dt><dd>{variable.displayValue}</dd></div>
          ))}</dl>
        ) : <p>Values are available only at a verified stop.</p>}
      </section>
      <section tabIndex={-1} className="debug-inspector" aria-label="CPU Registers">
        <h3>CPU Registers</h3>
        {stopped && status.registers ? (
          <dl className="register-grid">{Object.entries(status.registers).map(([name, value]) => (
            <div key={name}><dt>{name.toUpperCase()}</dt><dd>{hex(value, ["cc", "a", "b", "dp"].includes(name) ? 2 : 4)}</dd></div>
          ))}</dl>
        ) : <p>Registers are unavailable while execution is running.</p>}
      </section>
    </div>
  );
}

function hex(value: number, width: number): string {
  return `$${value.toString(16).toUpperCase().padStart(width, "0")}`;
}
