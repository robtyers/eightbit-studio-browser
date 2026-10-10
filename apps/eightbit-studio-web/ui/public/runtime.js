/* Eightbit Studio browser integration, GPL-3.0-or-later.
 * XRoar remains a separate upstream source tree with a local-files-only patch.
 * This disposable document owns the emulator, keyboard state and audio context.
 */
"use strict";
(() => {
  let session;
  let ready = false;
  let started = false;
  const held = new Set();
  const screen = document.getElementById("screen");
  const status = document.getElementById("status");
  const keys = document.getElementById("keys");
  const send = (state) => parent.postMessage({ schemaVersion: 1, session, state }, location.origin);
  const release = () => {
    if (ready) for (const scan of held) window.Module.ccall("wasm_scan_release", null, ["number"], [scan]);
    held.clear();
  };
  const closeAudio = () => {
    release(); ready = false;
    const context = window.Module?.SDL2?.audioContext;
    if (context && context.state !== "closed") void context.close().catch(() => {});
  };
  // The owning adapter calls this synchronously before discarding the iframe.
  window.__eightbit_stop = closeAudio;
  window.addEventListener("pagehide", closeAudio);
  window.addEventListener("blur", release);
  document.addEventListener("visibilitychange", () => { if (document.hidden) closeAudio(); });
  const fail = () => { closeAudio(); status.textContent = "Emulator failed to start."; send("failed"); };
  window.addEventListener("error", (event) => {
    if (event.filename === new URL("generated/xroar.js", location.href).href || event.filename === new URL("runtime.js", location.href).href) fail();
  });

  // Both the C patch and this transport allow-list prevent firmware downloads.
  const wasmURL = new URL("generated/xroar.wasm", location.href).href;
  const originalFetch = window.fetch.bind(window);
  window.fetch = (input, options) => {
    const url = new URL(input instanceof Request ? input.url : String(input), location.href).href;
    if (url !== wasmURL) return Promise.reject(new Error("Runtime asset is not allowed"));
    return originalFetch(input, options);
  };
  const originalOpen = XMLHttpRequest.prototype.open;
  XMLHttpRequest.prototype.open = function (method, url, ...rest) {
    if (method !== "GET" || new URL(String(url), location.href).href !== wasmURL) throw new Error("Runtime asset is not allowed");
    return originalOpen.call(this, method, url, ...rest);
  };
  // XRoar calls these upstream UI hooks. This edition does not expose machine,
  // cartridge, disk or arbitrary download controls, nor interpret HTML output.
  for (const name of ["ui_menu_select", "ui_update_tape_input_filename", "ui_update_tape_playing", "ui_update_disk_info", "ui_set_html", "ui_update_hd_filename", "ui_set_fullscreen", "ui_set_checkbox", "ui_set_value", "ui_menu_clear", "ui_menu_add"]) window[name] = () => {};
  window.ui_done_initialising = () => {
    ready = true;
    for (const input of document.querySelectorAll("button, input")) input.disabled = false;
    status.textContent = "Running locally";
    screen.focus(); send("ready");
  };
  document.getElementById("audio").addEventListener("click", () => {
    const context = window.Module?.SDL2?.audioContext;
    if (context) void context.resume().then(() => { status.textContent = "Sound enabled"; }).catch(() => { status.textContent = "Sound is unavailable"; });
  });
  const type = () => {
    if (!ready) return;
    const text = keys.value;
    if (!/^[\x20-\x7e]{0,128}$/.test(text)) { status.textContent = "Use up to 128 ASCII characters."; return; }
    window.Module.ccall("wasm_queue_basic", null, ["string"], [`${text}\r`]);
    keys.value = ""; screen.focus();
  };
  document.getElementById("type").addEventListener("click", type);
  keys.addEventListener("keydown", (event) => { event.stopPropagation(); if (event.key === "Enter") { event.preventDefault(); type(); } });
  keys.addEventListener("keyup", (event) => event.stopPropagation());
  for (const button of document.querySelectorAll("button[data-scan]")) {
    const scan = Number(button.dataset.scan);
    const down = () => { if (ready && !held.has(scan)) { held.add(scan); window.Module.ccall("wasm_scan_press", null, ["number"], [scan]); } };
    const up = () => { if (ready && held.delete(scan)) window.Module.ccall("wasm_scan_release", null, ["number"], [scan]); };
    button.addEventListener("pointerdown", (event) => { event.preventDefault(); button.setPointerCapture(event.pointerId); down(); });
    for (const name of ["pointerup", "pointercancel", "lostpointercapture", "blur"]) button.addEventListener(name, up);
    button.addEventListener("keydown", (event) => { if (event.key === " " || event.key === "Enter") { event.preventDefault(); event.stopPropagation(); down(); } });
    button.addEventListener("keyup", (event) => { event.stopPropagation(); up(); });
  }
  window.addEventListener("message", (event) => {
    if (event.source !== parent || event.origin !== location.origin || !event.data || event.data.schemaVersion !== 1) return;
    if (event.data.operation === "stop" && event.data.session === session) { closeAudio(); return; }
    if (event.data.operation !== "start" || started || !Number.isSafeInteger(event.data.session) || !(event.data.firmware instanceof Uint8Array) || event.data.firmware.length !== 16384 || !(event.data.media instanceof Uint8Array) || event.data.media.length > 8 * 1024 * 1024) return;
    started = true; session = event.data.session;
    const firmware = event.data.firmware;
    const media = event.data.media;
    window.Module = {
      canvas: screen,
      arguments: ["-no-c", "-machine", "dragon32", "-extbas", "/dragon32.rom", "-no-cart", "-tape-fast", "-run", "/game.cas", "-ao-fragments", "3"],
      locateFile: (name) => { if (name !== "xroar.wasm") throw new Error("Runtime asset is not allowed"); return wasmURL; },
      preRun: [() => { window.Module.FS.writeFile("/dragon32.rom", firmware); window.Module.FS.writeFile("/game.cas", media); firmware.fill(0); media.fill(0); }],
      print: () => {}, printErr: () => {}, onAbort: fail,
    };
    const script = document.createElement("script"); script.src = "generated/xroar.js"; script.onerror = fail; document.head.append(script);
  });
})();
