import { useWorkbenchCommands } from "@eightbitstudio/workbench/hooks/useWorkbenchCommands";
import { useEffect, useState } from "react";
import { FocusIcon, PlayIcon, StopIcon } from "@eightbitstudio/workbench/components/Icons";
import type { ReactNode } from "react";

export interface MachinePanelStatus {
  processId?: number | null;
  state: string;
  valuesValid: boolean;
  message: string;
  stopId: number | string | null;
  breakpointVerified: boolean;
  breakpointAddress: number;
  registers: Record<string, number> | null;
  memory?: { startAddress: number; bytes: number[] } | null;
  disassembly: { address: number; bytes: number[]; mnemonic: string; operand: string }[];
}

interface MachineBreakpoint {
  sourceUri: string;
  sourceLine: number;
}

export interface MachineDebugPanelProps {
  cpuLabel: string;
  runtimeMissing: string;
  assemblerMissing: string;
  addressPlaceholder: string;
  addressBreakpoints?: boolean;
  byteRegisters: readonly string[];
  sourceInput?: ReactNode;
  showDisassembly?: boolean;
  busy: boolean;
  active?: boolean;
  runtime: string | null;
  assembler: string | null;
  breakpoint: MachineBreakpoint | null;
  addressBreakpoint: number | null;
  status: MachinePanelStatus | null;
  continuing: boolean;
  onAddressBreakpoint: (address: number | null) => void;
  onConfigureRuntime?: () => void;
  onConfigureAssembler?: () => void;
  onStart: () => void;
  onStep: () => void;
  onContinue: () => void;
  onPause: () => void;
  onFocus?: () => void;
  focusLabel?: string;
  onTerminate: () => void;
}

export function MachineDebugPanel({
  busy,
  active: sessionActive,
  runtime,
  assembler,
  cpuLabel,
  runtimeMissing,
  assemblerMissing,
  addressPlaceholder,
  addressBreakpoints = true,
  byteRegisters,
  sourceInput,
  showDisassembly = true,
  breakpoint,
  addressBreakpoint,
  status,
  continuing,
  onAddressBreakpoint,
  onConfigureRuntime,
  onConfigureAssembler,
  onStart,
  onStep,
  onContinue,
  onPause,
  onFocus,
  focusLabel = "Show Emulator Window",
  onTerminate,
}: MachineDebugPanelProps) {
  const active = sessionActive ?? Boolean(status?.processId);
  const stopped = status !== null && status.valuesValid && (status.state === "stopped" || status.state === "cancelled");
  const running = status?.state === "running" || continuing;
  const pauseRequested = status?.message.startsWith("Pause requested") ?? false;
  const [addressText, setAddressText] = useState(
    addressBreakpoint === null ? "" : hex(addressBreakpoint, 4),
  );
  useEffect(() => {
    if (addressBreakpoint === null && parseAddress(addressText) !== null) setAddressText("");
  }, [addressBreakpoint, addressText]);
  useWorkbenchCommands({
    breakpoints: () => document.querySelector<HTMLInputElement>(".machine-debug-panel input")?.focus(),
    registers: () => document.querySelector<HTMLElement>('.machine-debug-panel [aria-label="CPU Registers"]')?.focus(),
    memory: () => document.querySelector<HTMLElement>('.machine-debug-panel [aria-label="Machine Memory"]')?.focus(),
  });
  return (
    <div className="debug-panel machine-debug-panel" aria-live="polite">
      <div className="debug-summary">
        <strong>{status?.message ?? `No ${cpuLabel} debug session`}</strong>
        <span>
          {breakpoint
            ? `Breakpoint ${breakpoint.sourceUri}:${breakpoint.sourceLine}`
            : addressBreakpoint !== null
              ? `Address breakpoint ${hex(addressBreakpoint, 4)}`
              : "Choose an emitting source line or linked address."}
        </span>
        {sourceInput}
        {addressBreakpoints && <label>
          Address breakpoint
          <input
            aria-label={`${cpuLabel} address breakpoint`}
            value={addressText}
            placeholder={addressPlaceholder}
            onChange={(event) => {
              setAddressText(event.target.value);
              onAddressBreakpoint(parseAddress(event.target.value));
            }}
            disabled={busy || active}
          />
        </label>}
        <span>
          {runtime ?? runtimeMissing}
          {assembler ? ` · ${assembler}` : ` · ${assemblerMissing}`}
        </span>
        {status?.breakpointVerified && (
          <em>● Verified breakpoint at {hex(status.breakpointAddress, 4)}</em>
        )}
        <div className="debug-actions">
          {onConfigureRuntime && <button type="button" onClick={onConfigureRuntime} disabled={busy || active}>Emulator Setup…</button>}
          {onConfigureAssembler && <button type="button" onClick={onConfigureAssembler} disabled={busy || active}>Assembler Setup…</button>}
          <button type="button" className="debug-control" onClick={onStart} disabled={busy || active || !runtime || !assembler || (!breakpoint && addressBreakpoint === null)}><PlayIcon /> Debug</button>
          <button type="button" onClick={onStep} disabled={busy || !stopped}>Step</button>
          <button type="button" onClick={onContinue} disabled={busy || !stopped}>Continue</button>
          <button type="button" onClick={onPause} disabled={busy || !running || pauseRequested}>Pause</button>
          {onFocus && <button type="button" onClick={onFocus} disabled={busy || !active}><FocusIcon /> {focusLabel}</button>}
          <button type="button" className="terminate-control" onClick={onTerminate} disabled={busy || !active}><StopIcon /> Stop</button>
        </div>
      </div>
      <section tabIndex={-1} className="debug-inspector" aria-label="CPU Registers">
        <h3>CPU Registers {status?.stopId ? <small>stop {status.stopId}</small> : null}</h3>
        {stopped && status.registers ? (
          <dl className="register-grid">{Object.entries(status.registers).map(([name, value]) => (
            <div key={name}><dt>{name.toUpperCase()}</dt><dd>{hex(value, byteRegisters.includes(name) ? 2 : 4)}</dd></div>
          ))}</dl>
        ) : <p>Registers are available only at a verified stop.</p>}
      </section>
      <section tabIndex={-1} className="debug-inspector machine-memory" aria-label="Machine Memory">
        <h3>Memory {status?.memory ? <small>{hex(status.memory.startAddress, 4)}</small> : null}</h3>
        {stopped && status.memory ? (
          <pre>{formatMemory(status.memory.startAddress, status.memory.bytes)}</pre>
        ) : <p>Memory is available only at a verified stop.</p>}
      </section>
      {showDisassembly && <section tabIndex={-1} className="debug-inspector machine-disassembly" aria-label={`${cpuLabel} Disassembly`}>
        <h3>Disassembly</h3>
        {stopped && status.disassembly.length > 0 ? (
          <table>
            <tbody>{status.disassembly.map((instruction) => (
              <tr key={instruction.address} className={instruction.address === status.registers?.pc ? "current" : undefined}>
                <td>{instruction.address === status.registers?.pc ? "▶" : ""}</td>
                <th scope="row">{hex(instruction.address, 4)}</th>
                <td>{instruction.bytes.map((byte) => byte.toString(16).toUpperCase().padStart(2, "0")).join(" ")}</td>
                <td>{instruction.mnemonic}{instruction.operand ? ` ${instruction.operand}` : ""}</td>
              </tr>
            ))}</tbody>
          </table>
        ) : <p>Disassembly is available only at a verified stop.</p>}
      </section>}
    </div>
  );
}

function formatMemory(start: number, bytes: number[]): string {
  const rows: string[] = [];
  for (let offset = 0; offset < bytes.length; offset += 8) {
    const values = bytes.slice(offset, offset + 8)
      .map((byte) => byte.toString(16).toUpperCase().padStart(2, "0"))
      .join(" ");
    rows.push(`${hex(start + offset, 4)}  ${values}`);
  }
  return rows.join("\n");
}

function hex(value: number, width: number): string {
  return `$${value.toString(16).toUpperCase().padStart(width, "0")}`;
}

function parseAddress(value: string): number | null {
  const match = value.trim().match(/^(?:\$|0x)?([0-9a-f]{1,4})$/i);
  return match ? Number.parseInt(match[1]!, 16) : null;
}
