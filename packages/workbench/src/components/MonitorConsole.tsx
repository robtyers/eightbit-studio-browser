import { originalSpectrumTarget, spectrumModelName } from "../services/monitorFrame";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { MonitorAudio } from "./MonitorAudio";

import { MONITOR_CASSETTE_LIMIT, type MonitorCommand, type MonitorTransport } from "../services/monitorProtocol";
export { MONITOR_CASSETTE_LIMIT, MONITOR_REQUEST_LIMIT, type MonitorCommand, type MonitorTransport } from "../services/monitorProtocol";
import { monitorFrame, recordedDownload, type Cassette, type Recording, type MonitorMachine } from "../services/monitorFrame";
export { monitorFrame } from "../services/monitorFrame";

/** Presentation only: hardware, key mapping and firmware commands remain in Rust. */
const vectrexKeys = new Set(["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "KeyZ", "KeyX", "KeyC", "KeyV", "KeyA", "KeyD", "KeyW", "KeyS", "KeyF", "KeyG", "KeyH", "KeyJ"]);

export function MonitorConsole({ transport, sessionId, onStop, saveRecording, hiddenBehaviour = "stop", unmountBehaviour = "stop", machine = "dragon32" }: { transport: MonitorTransport; sessionId: number; onStop: () => void | Promise<void>; saveRecording?: () => Promise<"saved" | "cancelled">; hiddenBehaviour?: "stop" | "pause"; unmountBehaviour?: "stop" | "release"; machine?: MonitorMachine }) {
  const vectrex = machine === "vectrex";
  const c64 = machine === "c64-pal";
  const spectrum = originalSpectrumTarget(machine);
  const dragon64 = machine === "dragon64" || machine === "dragon64-native";
  const width = c64 ? 384 : spectrum ? 320 : 256;
  const height = vectrex ? 320 : c64 ? 272 : spectrum ? 256 : 192;
  const canvas = useRef<HTMLCanvasElement>(null);
  const active = useRef(false);
  const audio = useRef<MonitorAudio | undefined>(undefined);
  const audioEpoch = useRef(0);
  const [basic, setBasic] = useState(c64);
  const [qualifiedBasic, setQualifiedBasic] = useState(false);
  const [extendedBasic, setExtendedBasic] = useState(false);
  const [cassette, setCassette] = useState<Cassette>();
  const [cassetteMessage, setCassetteMessage] = useState("");
  const [recording, setRecording] = useState<Recording>();
  const [recordingBusy, setRecordingBusy] = useState(false);
  const [recordingMessage, setRecordingMessage] = useState("");
  const recordingEpoch = useRef(0);
  const downloads = useRef(new Map<string, ReturnType<typeof setTimeout>>());
  const [mounting, setMounting] = useState(false);
  const mediaEpoch = useRef(0);
  const [audioMessage, setAudioMessage] = useState("");
  const [message, setMessage] = useState(c64 ? "Starting original C64…" : "Starting original monitor…");
  const [failed, setFailed] = useState(false);
  const [disconnected, setDisconnected] = useState(false);
  const [stopping, setStopping] = useState(false);
  const stopPending = useRef(false);
  const [input, setInput] = useState("");
  const [screenText, setScreenText] = useState("");
  const lifetime = useRef<object | undefined>(undefined);
  const stopCallback = useRef(onStop);
  stopCallback.current = onStop;
  const queue = useMemo(() => ({ sequence: Promise.resolve(), pending: 0, controls: new Map<string, Promise<unknown>>() }), [transport, sessionId]);
  const request = useCallback((command: MonitorCommand) => {
    const control = command.operation === "release" || command.operation === "stop";
    // Reserve one slot per teardown control so input floods cannot retain keys.
    const existing = control ? queue.controls.get(command.operation) : undefined;
    if (existing) return existing;
    if (!control && queue.pending >= 16) return Promise.reject(new Error("Monitor input queue is full. Release the keys and retry."));
    if (!control) queue.pending += 1;
    const result = queue.sequence.then(() => transport(command));
    if (control) queue.controls.set(command.operation, result);
    queue.sequence = result.then(() => undefined, () => undefined).then(() => {
      if (control) queue.controls.delete(command.operation);
      else queue.pending -= 1;
    });
    return result;
  }, [queue, transport]);
  const release = useCallback(() => { if (active.current) void request({ operation: "release" }).catch(() => {}); }, [request]);
  const stop = useCallback(() => {
    if (stopPending.current) return;
    stopPending.current = true;
    active.current = false;
    mediaEpoch.current += 1;
    recordingEpoch.current += 1;
    audio.current?.clear();
    setStopping(true); setFailed(true); setDisconnected(true);
    setMessage("Stopping…");
    const identity = lifetime.current;
    const finish = stopCallback.current;
    // Keep the native window alive until its queued Stop has completed. Closing
    // it also releases the owned session if the transport has already failed.
    void request({ operation: "stop" }).catch(() => {}).then(async () => {
      if (lifetime.current === identity) await finish();
    }).catch(() => {
      if (lifetime.current === identity) setMessage("Cannot close the emulator. Retry Stop or close its window.");
    }).finally(() => {
      if (lifetime.current === identity) { stopPending.current = false; setStopping(false); }
    });
  }, [request]);
  const resumeAudio = () => { const owner = audio.current; if (owner) void owner.resume().then(enabled => { if (active.current && audio.current === owner) setAudioMessage(enabled ? "" : "Audio unavailable."); }); };
  const inputFailure = (error: unknown, identity: object | undefined) => { if (active.current && lifetime.current === identity) { setMessage(error instanceof Error ? error.message : "Keyboard input was rejected."); release(); } };
  useEffect(() => {
    active.current = true;
    const sound = new MonitorAudio(spectrum || c64 || vectrex ? 44100 : 48000);
    audio.current = sound;
    const identity = {};
    lifetime.current = identity;
    stopPending.current = false; setStopping(false); setFailed(false); setDisconnected(false);
    setCassette(undefined); setCassetteMessage(""); setMounting(false); setRecording(undefined); setRecordingBusy(false); setRecordingMessage("");
    let timer: ReturnType<typeof setTimeout> | undefined;
    let generation = true;
    let advancing = false;
    let frameCycles = vectrex ? 30000 : c64 ? 19656 : spectrum ? 69888 : 17920;
    const paused = () => hiddenBehaviour === "pause" && document.hidden;
    const advance = async () => {
      timer = undefined;
      if (!generation || !active.current || paused()) return;
      advancing = true;
      try {
        const epoch = audioEpoch.current;
        const frame = monitorFrame(await request({ operation: "advance", cycles: frameCycles }), sessionId, machine);
        if (!generation || !active.current) return;
        frameCycles = frame.frameCycles ?? frameCycles;
        if (frame.state === "running" && epoch === audioEpoch.current && !paused()) sound.push(frame.audio);
        else sound.clear();
        canvas.current?.getContext("2d")?.putImageData(new ImageData(frame.pixels, width, height), 0, 0);
        setBasic(frame.basic); setQualifiedBasic(frame.qualifiedBasic === true); setExtendedBasic(frame.extendedBasic === true); setMessage(frame.message); setFailed(frame.state !== "running"); setScreenText(frame.text); setCassette(frame.cassette); setRecording(frame.recording);
        if (frame.state === "stopped") { active.current = false; setDisconnected(true); }
      } catch (error) {
        if (generation && active.current) {
          active.current = false; setFailed(true); setDisconnected(true);
          sound.clear();
          setMessage(`${error instanceof Error ? error.message : "Monitor operation failed."} Stop and start the ${c64 ? "computer" : "monitor"} again.`);
          void request({ operation: "stop" }).catch(() => {});
        }
      } finally {
        advancing = false;
      }
      if (generation && active.current && !paused()) timer = setTimeout(() => { void advance(); }, 20);
    };
    const suspend = () => {
      if (document.hidden) {
        clearTimeout(timer); timer = undefined;
        if (hiddenBehaviour === "stop") stop();
        else {
          // Native WebKit can hide an occluded window or its file dialogue.
          // Retain guest/media ownership, but execute no more slices or audio.
          audioEpoch.current += 1; sound.clear(); release();
        }
      } else if (hiddenBehaviour === "pause" && !advancing && timer === undefined) {
        void advance();
      }
    };
    window.addEventListener("blur", release); document.addEventListener("visibilitychange", suspend);
    void advance();
    return () => {
      generation = false; active.current = false; mediaEpoch.current += 1; recordingEpoch.current += 1; clearTimeout(timer);
      for (const [url, timeout] of downloads.current) { clearTimeout(timeout); URL.revokeObjectURL(url); }
      downloads.current.clear();
      sound.dispose(); if (audio.current === sound) audio.current = undefined;
      window.removeEventListener("blur", release); document.removeEventListener("visibilitychange", suspend);
      // Invalidate pending callbacks immediately; a StrictMode replay replaces
      // this teardown marker before its microtask can stop the retained session.
      const ended = {};
      lifetime.current = ended;
      queueMicrotask(() => { if (lifetime.current === ended) void request({ operation: unmountBehaviour }).catch(() => {}); });
    };
  }, [request, release, stop, sessionId, hiddenBehaviour, unmountBehaviour, machine, spectrum, c64, vectrex, width, height]);
  const send = async () => {
    resumeAudio();
    const identity = lifetime.current;
    try { await request({ operation: "type", text: input }); if (active.current && lifetime.current === identity) { setInput(""); canvas.current?.focus(); } }
    catch (error) { inputFailure(error, identity); }
  };
  const reset = () => {
    const owner = audio.current;
    const identity = lifetime.current;
    audioEpoch.current += 1; mediaEpoch.current += 1; setMounting(false);
    recordingEpoch.current += 1; setRecordingBusy(false); setRecordingMessage("");
    owner?.clear(); resumeAudio();
    void request({ operation: "reset" }).then(() => {
      if (active.current && lifetime.current === identity && audio.current === owner) { owner?.clear(); setFailed(false); }
    }).catch(error => { if (lifetime.current === identity) inputFailure(error, identity); });
  };
  const mount = async (file: File) => {
    const identity = lifetime.current;
    const epoch = ++mediaEpoch.current;
    setMounting(true); setCassetteMessage("Checking cassette…");
    try {
      if (!file.size || file.size > MONITOR_CASSETTE_LIMIT) throw new Error(spectrum ? "Select a TAP or WAV file of 1–1048576 bytes." : "Select a CAS or WAV file of 1–1048576 bytes.");
      const bytes = new Uint8Array(await file.arrayBuffer());
      if (!active.current || lifetime.current !== identity || mediaEpoch.current !== epoch) return;
      const frame = monitorFrame(await request({ operation: /\.wav$/i.test(file.name) ? "mount-wav" : "mount-cassette", bytes: Array.from(bytes) }), sessionId, machine);
      if (!active.current || lifetime.current !== identity || mediaEpoch.current !== epoch) return;
      setCassette(frame.cassette); setCassetteMessage(spectrum ? 'Cassette mounted. Enter LOAD "" to load it.' : "Cassette mounted. Enter CLOAD then RUN for BASIC, or CLOADM then EXEC for machine code.");
    } catch (error) {
      if (active.current && lifetime.current === identity && mediaEpoch.current === epoch) setCassetteMessage(error instanceof Error ? error.message : "Cannot mount this cassette.");
    } finally {
      if (lifetime.current === identity && mediaEpoch.current === epoch) setMounting(false);
    }
  };
  const cassetteControl = async (operation: "rewind-cassette" | "eject-cassette") => {
    const identity = lifetime.current;
    const epoch = ++mediaEpoch.current;
    setMounting(true);
    try {
      const frame = monitorFrame(await request({ operation }), sessionId, machine);
      if (!active.current || lifetime.current !== identity || mediaEpoch.current !== epoch) return;
      setCassette(frame.cassette); setCassetteMessage(operation === "rewind-cassette" ? "Cassette rewound. Enter the guest load command again." : "Cassette ejected.");
    } catch (error) {
      if (active.current && lifetime.current === identity && mediaEpoch.current === epoch) setCassetteMessage(error instanceof Error ? error.message : "Cannot update the cassette.");
    } finally {
      if (lifetime.current === identity && mediaEpoch.current === epoch) setMounting(false);
    }
  };
  const recordingAction = async (operation: "start-recording" | "finish-recording" | "discard-recording" | "export-recording") => {
    const identity = lifetime.current;
    const epoch = recordingEpoch.current;
    setRecordingBusy(true); setRecordingMessage("");
    try {
      if (operation === "export-recording" && saveRecording) {
        const saved = await saveRecording();
        if (active.current && lifetime.current === identity && recordingEpoch.current === epoch) setRecordingMessage(saved === "saved" ? "Cassette copy saved." : "Save cancelled. The recording is still available.");
      } else {
        const value = await request({ operation });
        const frame = monitorFrame(value, sessionId, machine);
        if (!active.current || lifetime.current !== identity || recordingEpoch.current !== epoch) return;
        setRecording(frame.recording);
        if (operation === "export-recording") {
          const bytes = recordedDownload(value, machine);
          const url = URL.createObjectURL(new Blob([bytes], { type: "application/octet-stream" }));
          const link = document.createElement("a"); link.href = url; link.download = spectrum ? "spectrum-recording.tap" : "dragon-recording.cas";
          document.body.append(link); link.click(); link.remove();
          const timeout = setTimeout(() => { URL.revokeObjectURL(url); downloads.current.delete(url); }, 60_000);
          downloads.current.set(url, timeout);
          setRecordingMessage("Download requested. Check Files or Downloads before discarding the recording.");
        }
      }
    } catch (error) {
      if (active.current && lifetime.current === identity && recordingEpoch.current === epoch) setRecordingMessage(error instanceof Error ? error.message : "Cassette recording operation failed.");
    } finally {
      if (lifetime.current === identity && recordingEpoch.current === epoch) setRecordingBusy(false);
    }
  };
  return <section className="monitor-console" aria-label={vectrex ? "Original Vectrex development" : c64 ? "Original C64 PAL development" : spectrum ? `Original ${spectrumModelName(machine)} development` : dragon64 ? "Built-in Dragon 64 BASIC development" : extendedBasic ? "Built-in Dragon 32" : basic ? qualifiedBasic ? "Original Dragon 32 BASIC" : "Original Dragon 32 BASIC development" : "Original Dragon 32 monitor"}>
    <p role="status">{message}{audioMessage ? ` · ${audioMessage}` : ""}</p>
    {/* F5 belongs to the workbench Debug action, not the guest keyboard. */}
    <canvas ref={canvas} width={width} height={height} tabIndex={0} aria-label={vectrex ? "Vectrex screen; arrow keys move, Z X C V are buttons 1 to 4" : c64 ? "C64 BASIC screen; type BASIC commands" : spectrum ? "Spectrum BASIC screen; type BASIC commands" : basic ? "Dragon BASIC screen; type BASIC commands" : "Dragon monitor screen; type monitor commands"} style={{ width: "100%", maxHeight: "60vh", aspectRatio: vectrex ? "4 / 5" : "4 / 3", objectFit: "contain", imageRendering: "pixelated", background: "#123019" }} onBlur={release}
      onKeyDown={event => { if (vectrex && !vectrexKeys.has(event.code)) return; if (active.current && (!event.ctrlKey || c64) && !event.metaKey && event.key !== "Tab" && event.key !== "F5" && !event.repeat) { resumeAudio(); event.preventDefault(); const identity = lifetime.current; void request({ operation: "key", code: event.code, pressed: true }).catch(error => inputFailure(error, identity)); } }}
      onKeyUp={event => { if (vectrex && !vectrexKeys.has(event.code)) return; if (active.current && event.key !== "Tab" && event.key !== "F5") { event.preventDefault(); const identity = lifetime.current; void request({ operation: "key", code: event.code, pressed: false }).catch(error => inputFailure(error, identity)); } }} />
    {!vectrex && <form onSubmit={event => { event.preventDefault(); void send(); }}><label>{basic || spectrum ? "BASIC command or input" : "Monitor command"} <input value={input} maxLength={c64 ? 80 : spectrum ? 4095 : qualifiedBasic || extendedBasic ? 249 : basic ? 251 : 96} disabled={failed} onChange={event => setInput(event.currentTarget.value)} autoComplete="off" autoCapitalize={basic || spectrum ? "off" : "characters"} spellCheck={false} /></label><button type="submit" disabled={failed || (!basic && !spectrum && !input)}>Type command</button></form>}
    <div><button type="button" disabled={disconnected} onClick={reset}>{extendedBasic || spectrum || c64 || vectrex ? "Reset computer" : basic ? "Reset BASIC" : "Reset monitor"}</button> <button type="button" disabled={stopping} onClick={stop}>{extendedBasic || spectrum || c64 || vectrex ? "Stop computer" : basic ? "Stop BASIC" : "Stop monitor"}</button></div>
    {basic && !c64 && !vectrex && <details><summary>Cassette</summary>
      <label>{spectrum ? "Mount cassette (.tap or .wav, up to 1 MiB)" : "Mount cassette (.cas or .wav, up to 1 MiB)"} <input type="file" accept={spectrum ? ".tap,.wav" : ".cas,.wav"} disabled={failed || mounting} onChange={event => { const file = event.currentTarget.files?.[0]; event.currentTarget.value = ""; if (file) void mount(file); }} /></label>
      {cassette && <p>{cassette.filename || "Unnamed cassette"} · {cassette.format} · {cassette.bytes} bytes · {cassette.pending ? "Ready" : "End of tape"}</p>}
      <button type="button" disabled={failed || mounting || !cassette} onClick={() => { void cassetteControl("rewind-cassette"); }}>Rewind cassette</button> <button type="button" disabled={failed || mounting || !cassette} onClick={() => { void cassetteControl("eject-cassette"); }}>Eject cassette</button>
      {recording && <div>
        <button type="button" disabled={failed || recordingBusy || recording.state !== "idle"} onClick={() => { void recordingAction("start-recording"); }}>Record cassette output</button> <button type="button" disabled={failed || recordingBusy || recording.state !== "recording"} onClick={() => { void recordingAction("finish-recording"); }}>Finish recording</button> <button type="button" disabled={failed || recordingBusy || recording.state !== "ready"} onClick={() => { void recordingAction("export-recording"); }}>Save recorded cassette</button> <button type="button" disabled={failed || recordingBusy || recording.state === "idle"} onClick={() => { void recordingAction("discard-recording"); }}>Discard recording</button>
        <p role="status">{recordingMessage || recording.message}{recording.samples ? ` · ${(recording.samples / recording.sampleRate).toFixed(1)} s of cassette signal` : ""}</p>
      </div>}
      <p role="status">{cassetteMessage || (spectrum ? 'Mounting is local. Enter LOAD "" to load a cassette. Reset rewinds the cassette.' : "Mounting is local and does not type a load or run command. Reset keeps the current tape position; rewind before loading again.")}</p>
    </details>}
    <details><summary>Screen text and command help</summary><pre style={{ whiteSpace: "pre", overflow: "auto" }}>{screenText}</pre><p>{vectrex ? "Original Vectrex development executive. Focus the screen: arrows move controller 1, Z/X/C/V are its four buttons. Controller 2 uses A/D/W/S and F/G/H/J. Audio starts after a key press. Unsupported executive services fail explicitly; use VecX for compatibility. There is no built-in game or BASIC prompt." : c64 ? "Original C64 PAL development firmware. Enter BASIC commands or replies here. VICE remains the external compatibility option. The original random sequence, transcendental approximations and stack limits differ from the historical firmware. Drives, cartridges and debugging are unsupported." : spectrum ? `Original ${spectrumModelName(machine)} development runtime. Enter BASIC commands or input here. Compatibility qualification is incomplete; MAME remains available with separately supplied firmware.` : dragon64 ? "Built-in Dragon 64 development firmware. Enter BASIC commands or replies here. XRoar remains available in the desktop edition." : extendedBasic ? "Built-in Dragon 32 runs BASIC and machine code with an expanded 32 KiB ROM. Enter BASIC commands or replies here. Cartridges require the desktop edition with XRoar and separately supplied firmware." : qualifiedBasic ? "Original Dragon 32 BASIC. Enter BASIC commands or replies here." : basic ? "Original BASIC development ROM. Language, cassette and legacy ABI qualification are incomplete. Enter BASIC commands or replies here; use XRoar for compatibility-dependent programmes." : "Type H for help. This original ROM is a monitor. BASIC is under development."}</p></details>
  </section>;
}
