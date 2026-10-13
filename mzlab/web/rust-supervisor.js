// @ts-strict
// Helpers of the supervisor of the Rust engine (rust-bridge.js).

// The .wasm of the engine compiled once: `compileOnce(url)()` gives the same promise of a WebAssembly.Module every time (null when it cannot be compiled here:
// the worker then compiles it by itself).
export function compileOnce(/** @type {URL} */ url) {
  /** @type {Promise<WebAssembly.Module | null> | null} */ let compiled = null;
  return () => compiled || (compiled = (async () => {
    try { return await WebAssembly.compileStreaming(fetch(url)); }
    catch (_) { try { return await WebAssembly.compile(await (await fetch(url)).arrayBuffer()); } catch (e) { compiled = null; return null; } }
  })());
}

// A death with the signature of a lack of memory: a trap or `rust_oom` after the memory had grown past 64 MB (`peak` = the largest memory the worker reported):
// the file is too big for the budget, not a fault to retry as it is.
export function oomSign(/** @type {{ panic?: string | null, text?: string, mem?: number } | undefined} */ info, /** @type {number} */ peak) {
  return /unreachable|rust_oom|out of memory|memory access|memoryBudget/i.test(String((info && (info.panic || info.text)) || "")) && Math.max(peak, (info && info.mem) || 0) > 64 * 1048576;
}

// Orders the messages to a worker: the compiled module goes first (null = it compiles by itself), then the budget when the supervisor lowered it, then everything else in the order sent.
export function ordered(/** @type {any} */ w, /** @type {() => Promise<WebAssembly.Module | null>} */ compile, /** @type {() => number} */ budget, /** @type {() => number} */ nextId) {
  let sent = false;
  const first = compile().then(module => { w.postMessage({ op: "module", module }); if (budget()) w.postMessage({ op: "budget", mb: budget(), id: nextId() }); sent = true; });
  return (/** @type {any} */ msg, /** @type {Transferable[] | undefined} */ transfer) => { if (sent) w.postMessage(msg, transfer); else first.then(() => w.postMessage(msg, transfer)); };
}

// What to do after a death of the worker: "restart" (the first one in a minute), "halve" (the second with the signature of a lack of memory: once more with half the
// budget), "oom" (the next one: off with error.memoryBudget) or "fault" (the second of another kind: off, the page asks to reload). `state.restarts` = the times of the deaths.
export function judgeDeath(/** @type {any} */ state, /** @type {any} */ info, /** @type {number} */ peak) {
  const { api, stats } = state, now = Date.now();
  if (info && info.panic) api.lastPanic = String(info.panic);
  state.restarts = state.restarts.filter((/** @type {number} */ t) => now - t < 60000); state.restarts.push(now); stats.restarts++;
  if (state.restarts.length < 2) return "restart";
  if (!oomSign(info, peak)) return "fault";
  if (api.halved) return "oom";
  api.halved = true; api.budgetMb = 96; stats.halved = (stats.halved || 0) + 1;
  return "halve";
}

// Test hook: the count of deaths and the lowered budget start again.
export function forgetFaults(/** @type {any} */ state) {
  const { api, stats } = state; state.restarts = []; stats.restarts = 0; stats.halved = 0; api.halved = false; api.budgetMb = 0; api.disabled = false; api.lastPanic = null;
}
