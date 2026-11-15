import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { BrowserWorkbench } from "@eightbitstudio/browser-workbench";
import { originalRuntimeCapability, type OriginalRuntimeProfile } from "@eightbitstudio/workbench/services/originalRuntime";
import { machineCatalog, result, type BrowserMachine } from "@eightbitstudio/browser-workbench/contracts";
import "@eightbitstudio/workbench/styles.css";
import "@eightbitstudio/workbench/macos.css";
import "@eightbitstudio/browser-workbench/styles.css";
import { WorkerEngine } from "./engine";
import { Emulator } from "./Emulator";
import { Monitor, SpectrumMonitor, C64Monitor, VectrexMonitor, browserDevelopmentDebug, browserOriginalRuntime } from "./Monitor";
const engine = new WorkerEngine();
function App() {
  const [originalProfile, setOriginalProfile] = useState<OriginalRuntimeProfile | null>(null);
  const [originalProfiles, setOriginalProfiles] = useState<Record<string, OriginalRuntimeProfile>>({});
  const [machines, setMachines] = useState<BrowserMachine[]>();
  const [error, setError] = useState(false);
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    const controller = new AbortController(); setError(false);
    void engine.request({ schemaVersion: 1, operation: "machines" }, new Uint8Array(), controller.signal).then((value) => {
      if (!controller.signal.aborted) {
        setMachines(machineCatalog(value));
        try { setOriginalProfile(originalRuntimeCapability(result(value).originalRuntime)); } catch { setOriginalProfile(null); }
        const profiles: Record<string, OriginalRuntimeProfile> = {};
        const available = result(value).originalRuntimes;
        if (available !== null && typeof available === "object") {
          for (const target of ["dragon64", "dragon64-native"] as const) {
            try { const profile = originalRuntimeCapability(Reflect.get(available, target)); if (profile?.target === target) profiles[target] = profile; } catch { /* Invalid profiles remain unavailable. */ }
          }
        }
        setOriginalProfiles(profiles);
      }
    }).catch(() => { if (!controller.signal.aborted) setError(true); });
    return () => controller.abort();
  }, [attempt]);
  return machines ? <BrowserWorkbench machines={machines} engine={engine} Emulator={Emulator} Monitor={Monitor} SpectrumMonitor={SpectrumMonitor} C64Monitor={C64Monitor} VectrexMonitor={VectrexMonitor} DevelopmentDebugger={browserDevelopmentDebug} OriginalRuntime={browserOriginalRuntime} originalProfile={originalProfile} originalProfiles={originalProfiles} assetBase={import.meta.env.BASE_URL} /> : <main className="browser-output"><h1>Eightbit Studio</h1><p role="status">{error ? "The browser tools could not load. Your saved projects remain on this device." : "Preparing your studio…"}</p>{error && <button type="button" onClick={() => setAttempt((value) => value + 1)}>Try again</button>}</main>;
}
const root = document.getElementById("root");
if (root) createRoot(root).render(<App />);
