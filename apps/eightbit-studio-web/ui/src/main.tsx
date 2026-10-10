import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { BrowserWorkbench } from "@eightbitstudio/browser-workbench";
import { machineCatalog, type BrowserMachine } from "@eightbitstudio/browser-workbench/contracts";
import "@eightbitstudio/workbench/styles.css";
import "@eightbitstudio/workbench/macos.css";
import "@eightbitstudio/browser-workbench/styles.css";
import { WorkerEngine } from "./engine";
import { Emulator } from "./Emulator";
const engine = new WorkerEngine();
function App() {
  const [machines, setMachines] = useState<BrowserMachine[]>();
  const [error, setError] = useState(false);
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    const controller = new AbortController(); setError(false);
    void engine.request({ schemaVersion: 1, operation: "machines" }, new Uint8Array(), controller.signal).then((value) => {
      if (!controller.signal.aborted) setMachines(machineCatalog(value));
    }).catch(() => { if (!controller.signal.aborted) setError(true); });
    return () => controller.abort();
  }, [attempt]);
  return machines ? <BrowserWorkbench machines={machines} engine={engine} Emulator={Emulator} assetBase={import.meta.env.BASE_URL} /> : <main className="browser-output"><h1>Eightbit Studio</h1><p role="status">{error ? "The browser tools could not load. Your saved projects remain on this device." : "Preparing your studio…"}</p>{error && <button type="button" onClick={() => setAttempt((value) => value + 1)}>Try again</button>}</main>;
}
const root = document.getElementById("root");
if (root) createRoot(root).render(<App />);
