// Bootstraps the simulation worker. The page sends one `init` message carrying
// the URL of the app's wasm-bindgen glue; importing that module loads the wasm,
// because `dx` appends its own initialization call to the end of the glue and
// assigns `__dx_mainWasm` when it resolves.

const INIT_TIMEOUT_MS = 30000;
const POLL_INTERVAL_MS = 4;

// dx's initialization call is not awaited by the glue module, so a failure to
// load the wasm surfaces here rather than from the import.
let initFailure = null;
self.addEventListener("unhandledrejection", (event) => {
  initFailure ??= event.reason;
});

self.onmessage = async (event) => {
  const data = event.data;
  if (!data || data.type !== "init" || !data.js_url) {
    return;
  }
  // start_simulation_worker() installs the handler that serves jobs.
  self.onmessage = null;

  try {
    const mod = await import(new URL(data.js_url, self.location.href).href);
    await waitForWasm();
    mod.start_simulation_worker();
    self.postMessage({ type: "ready" });
  } catch (error) {
    self.postMessage({ type: "init_failed", error: String(error) });
  }
};

async function waitForWasm() {
  const deadline = Date.now() + INIT_TIMEOUT_MS;
  while (globalThis.__dx_mainWasm === undefined) {
    if (initFailure !== null) {
      throw initFailure;
    }
    if (Date.now() > deadline) {
      throw new Error("timed out waiting for wasm initialization");
    }
    await new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS));
  }
}
