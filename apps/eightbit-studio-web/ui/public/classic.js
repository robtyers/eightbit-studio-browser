/* Original Eightbit Studio runtime adapter, GPL-3.0-or-later.
 * External cores are pinned, separately noticed build inputs. No ROMs are embedded. */
"use strict";
(() => {
  const configurations = {
    "c64-pal": { core: "c64", size: 20480 },
    "spectrum16": { core: "mame-z80", size: 16384, machine: "spectrum", names: ["spectrum.rom"], options: ["-bios", "en", "-ramsize", "16K"], boot: 'j""{ENTER}', extension: "tap" },
    "spectrum48": { core: "mame-z80", size: 16384, machine: "spectrum", names: ["spectrum.rom"], options: ["-bios", "en", "-ramsize", "48K"], boot: 'j""{ENTER}', extension: "tap" },
    "spectrum128": { core: "mame-z80", size: 32768, machine: "spec128", names: ["zx128_0.rom", "zx128_1.rom"], options: ["-bios", "en", "-ramsize", "128K"], boot: '{ENTER}', extension: "tap" },
    "spectrum-plus2": { core: "mame-z80", size: 32768, machine: "specpls2", names: ["zxp2_0.rom", "zxp2_1.rom"], options: ["-bios", "en", "-ramsize", "128K"], boot: '{ENTER}', extension: "tap" },
    "zx81-1k": { core: "mame-z80", size: 8192, machine: "zx81", names: ["zx81a.rom"], options: ["-bios", "2nd", "-ramsize", "1K"], boot: 'j""{ENTER}', extension: "p" },
    "zx81-16k": { core: "mame-z80", size: 8192, machine: "zx81", names: ["zx81a.rom"], options: ["-bios", "2nd", "-ramsize", "16K"], boot: 'j""{ENTER}', extension: "p" },
  };
  const canvas = document.getElementById("screen"), status = document.getElementById("status"), keys = document.getElementById("keys");
  const held = new Set();
  const physical = new Set();
  const queue = [];
  const transport = new AbortController();
  let pending = [];
  let session, module, config, stopped = false, ready = false, frame, poll, keyboardPoll, audio, audioAt = 0, keyAt = 0, queuedKey;
  const send = (state) => parent.postMessage({ schemaVersion: 1, session, state }, location.origin);
  const assets = new Set(["c64.wasm", "mame-z80.wasm.gz"].map((name) => new URL(`generated/${name}`, location.href).href));
  const fetchAsset = window.fetch.bind(window);
  window.fetch = (input, options) => {
    const url = new URL(input instanceof Request ? input.url : String(input), location.href).href;
    if (!assets.has(url)) return Promise.reject(new Error("Runtime asset is not allowed"));
    if (url.endsWith(".gz")) return fetchAsset(input, { ...options, signal: transport.signal }).then((response) => {
      const decoded = response.headers.get("Content-Encoding")?.toLowerCase() === "gzip";
      if (!response.ok || !response.body || (!decoded && typeof DecompressionStream !== "function")) throw new Error("Compressed runtime is unavailable");
      let total=0;
      const bounded=new TransformStream({ transform(chunk,controller) { total+=chunk.byteLength; if (total > 48*1024*1024) throw new Error("Runtime exceeds its limit"); controller.enqueue(chunk); } });
      return new Response((decoded ? response.body : response.body.pipeThrough(new DecompressionStream("gzip"))).pipeThrough(bounded), { headers:{ "Content-Type":"application/wasm" } });
    });
    return fetchAsset(input, { ...options, signal: transport.signal });
  };
  const open = XMLHttpRequest.prototype.open;
  XMLHttpRequest.prototype.open = function (method, url, ...rest) {
    if (method !== "GET" || !assets.has(new URL(String(url), location.href).href)) throw new Error("Runtime asset is not allowed");
    if (String(url).endsWith(".gz")) throw new Error("Compressed runtime requires streaming fetch");
    return open.call(this, method, url, ...rest);
  };
  const c64Codes = { up:11, down:10, left:8, right:9, space:32, break:3 };
  const keyNames = { up:"ArrowUp", down:"ArrowDown", left:"ArrowLeft", right:"ArrowRight", space:" ", break:"Escape" };
  const key = (name, down) => {
    if (!ready) return;
    if (config.core === "c64") module._studio_key(c64Codes[name], down ? 1 : 0);
    else {
      const code = { up:38, down:40, left:37, right:39, space:32, break:27 }[name];
      canvas.dispatchEvent(new KeyboardEvent(down ? "keydown" : "keyup", { key:keyNames[name], code:name === "space" ? "Space" : name === "break" ? "Escape" : keyNames[name], keyCode:code, which:code, bubbles:true }));
    }
  };
  const release = () => { for (const name of held) key(name, false); held.clear(); if (config?.core === "c64") { for (const code of physical) module?._studio_key(code,0); physical.clear(); } if (queuedKey !== undefined && config?.core === "c64") module?._studio_key(queuedKey, 0); queuedKey = undefined; queue.length = 0; };
  const stop = () => {
    if (stopped) return;
    release(); stopped = true; transport.abort(); for (const bytes of pending) bytes.fill(0); pending = []; ready = false; cancelAnimationFrame(frame); clearInterval(poll); clearInterval(keyboardPoll);
    if (config?.core === "c64") module?._studio_stop();
    else { if (module?._eightbit_ready?.()) module?._ZN15running_machine15emscripten_exitEv?.(); window.__eightbit_mameAudio?.studioStop(); }
    if (audio && audio.state !== "closed") void audio.close().catch(() => {});
  };
  const fail = () => { stop(); status.textContent = "Emulator failed to start."; send("failed"); };
  window.__eightbit_stop = stop;
  window.addEventListener("pagehide", stop); window.addEventListener("blur", release);
  document.addEventListener("visibilitychange", () => { if (document.hidden) stop(); });
  window.addEventListener("error", fail); window.addEventListener("unhandledrejection", fail);
  const started = () => {
    if (stopped) return;
    ready = true; for (const element of document.querySelectorAll("button, input")) element.disabled = false;
    if (config.core !== "c64") keyboardPoll = setInterval(() => { if (queue.length && module._eightbit_key(queue[0])) queue.shift(); }, 100);
    status.textContent = "Running locally"; canvas.focus(); send("ready");
  };
  document.getElementById("audio").addEventListener("click", () => {
    try {
      if (config.core === "c64") { audio ??= new AudioContext({ sampleRate:44100 }); void audio.resume(); }
      else { void window.__eightbit_mameAudio?.studioResume(); }
      status.textContent = "Sound enabled";
    } catch { status.textContent = "Sound is unavailable"; }
  });
  const type = () => {
    if (!ready) return;
    const text = keys.value;
    if (!/^[\x20-\x7e]{0,128}$/.test(text)) { status.textContent = "Use up to 128 ASCII characters."; return; }
    if (queue.length + text.length + 1 > 256) { status.textContent = "Wait for the previous text to finish."; return; }
    queue.push(...Array.from(`${config.core === "c64" ? text.toUpperCase() : text}\r`, (char) => char.charCodeAt(0)));
    keys.value = ""; canvas.focus();
  };
  document.getElementById("type").addEventListener("click", type);
  keys.addEventListener("keydown", (event) => { event.stopPropagation(); if (event.key === "Enter") { event.preventDefault(); type(); } });
  keys.addEventListener("keyup", (event) => event.stopPropagation());
  for (const button of document.querySelectorAll("button[data-key]")) {
    const name = button.dataset.key;
    const down = () => { if (ready && !held.has(name)) { held.add(name); key(name, true); } };
    const up = () => { if (held.delete(name)) key(name, false); };
    button.addEventListener("pointerdown", (event) => { event.preventDefault(); button.setPointerCapture(event.pointerId); down(); });
    for (const event of ["pointerup", "pointercancel", "lostpointercapture", "blur"]) button.addEventListener(event, up);
    button.addEventListener("keydown", (event) => { if (event.key === " " || event.key === "Enter") { event.preventDefault(); event.stopPropagation(); down(); } });
    button.addEventListener("keyup", (event) => { event.stopPropagation(); up(); });
  }
  canvas.addEventListener("keydown", (event) => {
    if (!ready || config.core !== "c64") return;
    event.preventDefault();
    const special = { ArrowUp:11, ArrowDown:10, ArrowLeft:8, ArrowRight:9, Enter:13, Backspace:1, Escape:3 };
    const code = special[event.key] ?? (event.key.length === 1 ? event.key.toUpperCase().charCodeAt(0) : undefined);
    if (code !== undefined && code < 128) { physical.add(code); module._studio_key(code, 1); }
  });
  canvas.addEventListener("keyup", (event) => {
    if (!ready || config.core !== "c64") return;
    const special = { ArrowUp:11, ArrowDown:10, ArrowLeft:8, ArrowRight:9, Enter:13, Backspace:1, Escape:3 };
    const code = special[event.key] ?? (event.key.length === 1 ? event.key.toUpperCase().charCodeAt(0) : undefined);
    if (code !== undefined && code < 128) { physical.delete(code); module._studio_key(code, 0); }
  });
  const startC64 = async (firmware, media) => {
    const imported = await import("./generated/c64.js");
    module = await imported.default({ locateFile:(name) => { if (name !== "c64.wasm") throw new Error("Runtime asset is not allowed"); return new URL(`generated/${name}`, location.href).href; } });
    if (stopped) { firmware.fill(0); media.fill(0); return; }
    const stage = (bytes, call) => { const pointer = module._malloc(bytes.length); if (!pointer) throw new Error("Runtime memory unavailable"); try { module.HEAPU8.set(bytes, pointer); if (!call(pointer, bytes.length)) throw new Error("Runtime rejected input"); } finally { module.HEAPU8.fill(0, pointer, pointer + bytes.length); module._free(pointer); bytes.fill(0); } };
    stage(firmware, module._studio_init); stage(media, module._studio_load);
    canvas.width = module._studio_width(); canvas.height = module._studio_height();
    const context = canvas.getContext("2d"); if (!context) throw new Error("Canvas is unavailable");
    const image = context.createImageData(canvas.width, canvas.height);
    let previous = performance.now(); queue.push(82,85,78,13); started();
    const tick = (now) => {
      if (stopped) return;
      module._studio_step(Math.min(50000, Math.max(0, Math.round((now - previous) * 1000)))); previous = now;
      if (now >= keyAt) { if (queuedKey !== undefined) { module._studio_key(queuedKey, 0); queuedKey = undefined; } else { queuedKey = queue.shift(); if (queuedKey !== undefined) module._studio_key(queuedKey, 1); } keyAt = now + 60; }
      const pixels=module._studio_pixels(); image.data.set(module.HEAPU8.subarray(pixels,pixels + image.data.length)); context.putImageData(image, 0, 0);
      const count = module._studio_audio_count();
      if (audio?.state === "running" && count > 0 && audioAt <= audio.currentTime + 0.25) { const buffer = audio.createBuffer(1,count,44100); buffer.copyToChannel(module.HEAPF32.slice(module._studio_audio()/4,module._studio_audio()/4+count),0); const output = audio.createBufferSource(); output.buffer=buffer; output.connect(audio.destination); audioAt=Math.max(audio.currentTime,audioAt); output.start(audioAt); audioAt+=count/44100; }
      module._studio_audio_clear(); frame=requestAnimationFrame(tick);
    };
    frame=requestAnimationFrame(tick);
  };
  window.addEventListener("message", (event) => {
    if (event.source !== parent || event.origin !== location.origin || !event.data || event.data.schemaVersion !== 1 || session !== undefined) return;
    const data=event.data; const selected=Object.hasOwn(configurations,data.target) ? configurations[data.target] : undefined;
    if (data.operation !== "start" || !selected || !Number.isSafeInteger(data.session) || !(data.firmware instanceof Uint8Array) || data.firmware.length !== selected.size || !(data.media instanceof Uint8Array) || data.media.length < 1 || data.media.length > 65536) return;
    session=data.session; config=selected; const firmware=data.firmware, media=data.media; pending=[firmware,media];
    if (config.core === "c64") { void startC64(firmware,media).catch(fail).finally(() => { firmware.fill(0); media.fill(0); }); return; }
    window.Module={ canvas, noInitialRun:true, arguments:[config.machine,...config.options,"-noreadconfig","-nowriteconfig","-rompath","/roms","-hashpath","/empty","-artpath","/empty","-samplepath","/empty","-cfg_directory","/empty","-nvram_directory","/empty","-nohttp","-skip_gameinfo","-natural","-video","soft","-sound","js","-cassette",`/game.${config.extension}`,"-autoboot_delay","3","-autoboot_command",config.boot],
      locateFile:(name) => { if (name !== "mame-z80.wasm.gz") throw new Error("Runtime asset is not allowed"); return new URL(`generated/${name}`,location.href).href; },
      preRun:[() => { module=window.Module; if (stopped) return; module.FS.mkdir("/roms"); module.FS.mkdir(`/roms/${config.machine}`); module.FS.mkdir("/empty"); config.names.forEach((name,index) => module.FS.writeFile(`/roms/${config.machine}/${name}`,firmware.subarray(index*16384,(index+1)*16384))); module.FS.writeFile(`/game.${config.extension}`,media); firmware.fill(0); media.fill(0); }],
      print:() => {}, printErr:() => {}, onAbort:fail, onRuntimeInitialized:() => { if (stopped) return; poll=setInterval(() => { if (module?._eightbit_ready()) { clearInterval(poll); started(); } },100); module.callMain(module.arguments); },
    };
    const script=document.createElement("script"); script.src="generated/mame-z80.js"; script.onerror=fail; document.head.append(script);
  });
})();
