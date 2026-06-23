// assets/worker.js

let ready = false;
let wasmHandler = null;
let queue = [];
let initError = null;

function enqueue(event) {
  queue.push(event);
}

function flushQueue() {
  if (!wasmHandler) return;
  for (const ev of queue) wasmHandler(ev);
  queue = [];
}

function requestIdFromEvent(event) {
  const id = event && event.data && event.data.id;
  return Number.isFinite(id) ? id : 0;
}

function postInitErrorForId(id, error) {
  self.postMessage({
    id,
    output: {
      success: false,
      stats: null,
      error,
    },
  });
}

function failQueuedRequests(error) {
  const queued = queue;
  queue = [];

  const requestIds = queued
    .map(requestIdFromEvent)
    .filter((id) => id !== 0);

  if (requestIds.length === 0) {
    postInitErrorForId(0, error);
    return;
  }

  for (const id of requestIds) {
    postInitErrorForId(id, error);
  }
}

// Normal runtime handler (queue until wasm installs its own handler)
self.onmessage = (event) => {
  if (initError) {
    postInitErrorForId(requestIdFromEvent(event), initError);
  } else if (ready && wasmHandler) {
    wasmHandler(event);
  } else {
    enqueue(event);
  }
};

// Heuristically find the best wasm filename mentioned anywhere in the JS bundle.
// We strongly prefer ones containing "os-rs-gui_bg" and a hash.
function pickWasmFromJsText(text) {
  // Collect *all* .wasm strings (handles minified variants)
  const matches = [];
  const re = /['"]([^'"]+?\.wasm)['"]/g;
  for (let m; (m = re.exec(text)); ) {
    matches.push(m[1]);
  }
  if (matches.length === 0) return null;

  // Prefer os-rs-gui_bg hashed names first
  const preferred = matches
    .filter((s) => s.includes("os-rs-gui_bg") && s.includes("dxh"))
    .sort((a, b) => b.length - a.length)[0];
  if (preferred) return preferred;

  // Next prefer any os-rs-gui_bg
  const bg = matches
    .filter((s) => s.includes("os-rs-gui_bg"))
    .sort((a, b) => b.length - a.length)[0];
  if (bg) return bg;

  // Otherwise pick the longest .wasm string
  return matches.sort((a, b) => b.length - a.length)[0];
}

async function initWorker(js_url_from_main) {
  // Make JS url absolute (so URL() bases are always valid)
  const js_abs_url = new URL(js_url_from_main, self.location.href).href;

  // Fetch bundle text so we can find the hashed wasm name
  const res = await fetch(js_abs_url, { cache: "no-store" });
  if (!res.ok) throw new Error(`Failed to fetch JS bundle: ${res.status} ${res.statusText}`);
  const text = await res.text();

  const wasm_rel_or_abs = pickWasmFromJsText(text);
  if (!wasm_rel_or_abs) throw new Error("Could not locate any .wasm reference inside JS bundle");

  // Resolve wasm url relative to the JS module URL unless it's already absolute
  const wasm_abs_url = wasm_rel_or_abs.startsWith("http://") ||
    wasm_rel_or_abs.startsWith("https://") ||
    wasm_rel_or_abs.startsWith("/")
    ? wasm_rel_or_abs
    : new URL(wasm_rel_or_abs, js_abs_url).href;

  // Now import the module and init wasm explicitly with the resolved URL
  const mod = await import(js_abs_url);
  await mod.default(wasm_abs_url);

  mod.start_simulation_worker();

  // wasm-bindgen sets self.onmessage to its handler once initialized
  wasmHandler = self.onmessage;
  ready = true;
  flushQueue();
}

// Intercept first init message
const originalOnMessage = self.onmessage;
self.onmessage = async (event) => {
  try {
    if (initError) {
      postInitErrorForId(requestIdFromEvent(event), initError);
      return;
    }

    const data = event.data;

    if (!ready && data && data.type === "init" && data.js_url) {
      self.onmessage = originalOnMessage;
      await initWorker(data.js_url);
      return;
    }

    enqueue(event);
  } catch (e) {
    initError = "Worker init failed: " + e.toString();
    console.error("Worker init failed:", e);
    failQueuedRequests(initError);
  }
};
