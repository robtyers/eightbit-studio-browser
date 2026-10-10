import { useEffect, useRef } from "react";
import type { EmulatorProps } from "@eightbitstudio/browser-workbench";
import { record } from "@eightbitstudio/browser-workbench/contracts";

export function Emulator({ session, firmware, media, onState, machine }: EmulatorProps) {
  const frame = useRef<HTMLIFrameElement>(null);
  const started = useRef(false);
  useEffect(() => {
    const runtimeWindow = frame.current?.contentWindow;
    let complete = false;
    const timer = window.setTimeout(() => { if (!complete) onState("failed"); }, 20_000);
    const receive = (event: MessageEvent<unknown>) => {
      if (event.origin !== window.location.origin || event.source !== frame.current?.contentWindow || !record(event.data) || event.data.schemaVersion !== 1 || event.data.session !== session) return;
      if (event.data.state === "ready" || event.data.state === "failed") {
        complete = true; window.clearTimeout(timer); onState(event.data.state);
      }
    };
    window.addEventListener("message", receive);
    return () => {
      complete = true; window.clearTimeout(timer); window.removeEventListener("message", receive);
      try {
        const stop: unknown = runtimeWindow && Reflect.get(runtimeWindow, "__eightbit_stop");
        if (typeof stop === "function") stop();
      } catch { /* A discarded document may already have released its context. */ }
      firmware.fill(0); media.fill(0);
    };
  }, [session, firmware, media, onState, machine]);
  return <iframe ref={frame} title={`${machine.label} emulator`} className="browser-runtime" src={`${import.meta.env.BASE_URL}${machine.runtime === "xroar" ? "runtime.html" : "classic.html"}`} sandbox="allow-scripts allow-same-origin" onLoad={() => {
    if (started.current) return;
    started.current = true;
    frame.current?.contentWindow?.postMessage({ schemaVersion: 1, operation: "start", session, firmware, media, target: machine.id }, window.location.origin);
  }} />;
}
