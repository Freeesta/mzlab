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

// True when this browser runs SIMD128 (every .wasm is built with it, one binary and no fallback): a tiny module with a v128.const is validated, no library.
// `globalThis.MZLAB_NO_SIMD` is a test switch that makes the check fail.
export function simdOk() {
  if (/** @type {any} */ (globalThis).MZLAB_NO_SIMD) return false;
  try {
    return WebAssembly.validate(new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0, 1, 4, 1, 96, 0, 0, 3, 2, 1, 0, 10, 23, 1, 21, 0, 253, 12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 26, 11]));
  } catch (_) { return false; }
}

// A death with the signature of a lack of memory: a trap or `rust_oom` after the memory had grown past 64 MB (`peak` = the largest memory the worker reported):
// the file is too big for the budget, not a fault to retry as it is.
export function oomSign(/** @type {{ panic?: string | null, text?: string, mem?: number } | undefined} */ info, /** @type {number} */ peak) {
  return /unreachable|rust_oom|out of memory|memory access|memoryBudget/i.test(String((info && (info.panic || info.text)) || "")) && Math.max(peak, (info && info.mem) || 0) > 64 * 1048576;
}

// Orders the messages to a worker: the compiled module goes first (null = it compiles by itself), then the policy of the memory budget (what the engine decides it from),
// then the budget when the supervisor lowered it (the level of the session), then the step down after a death by lack of memory (`down()`: {current, strikes} or null,
// asked once from Rust's `budget_down`), then everything else in the order sent. `ready` resolves when the growth probe is known, so the policy carries it.
export function ordered(/** @type {any} */ w, /** @type {() => Promise<WebAssembly.Module | null>} */ compile, /** @type {() => number} */ budget, /** @type {() => number} */ nextId, /** @type {() => object | null} */ policy = () => null, /** @type {() => Promise<unknown>} */ ready = () => Promise.resolve(), /** @type {() => object | null} */ down = () => null) {
  let sent = false;
  const first = Promise.all([compile(), ready()]).then(([module]) => {
    w.postMessage({ op: "module", module }); const p = policy(); if (p) w.postMessage({ op: "policy", ...p });
    if (budget()) w.postMessage({ op: "budget", mb: budget(), id: nextId() });
    const d = down(); if (d) w.postMessage({ op: "down", ...d });
    sent = true;
  });
  return (/** @type {any} */ msg, /** @type {Transferable[] | undefined} */ transfer) => { if (sent) w.postMessage(msg, transfer); else first.then(() => w.postMessage(msg, transfer)); };
}

// What to do after a death of the worker. With the signature of a lack of memory: "down" (the first and the second within a minute: the engine restarts one level lower, the
// level comes from Rust's `budget_down`) or "oom" (the third: off with error.memoryBudget, no loop of restarts). Another kind of death: "restart" (the first in a minute) or
// "fault" (the second: off, the page asks to reload). `state.restarts` and `state.ooms` = the times of the deaths; `api.strikes` = the deaths for memory in the last minute.
export function judgeDeath(/** @type {any} */ state, /** @type {any} */ info, /** @type {number} */ peak) {
  const { api, stats } = state, now = Date.now(), recent = (/** @type {number[]} */ l) => l.filter(t => now - t < 60000);
  if (info && info.panic) api.lastPanic = String(info.panic);
  state.restarts = recent(state.restarts); state.restarts.push(now); stats.restarts++;
  if (!oomSign(info, peak)) return state.restarts.length < 2 ? "restart" : "fault";
  state.ooms = recent(state.ooms || []); state.ooms.push(now); api.strikes = state.ooms.length;
  return api.strikes >= 3 ? "oom" : "down";
}

// The step down after a death for lack of memory: the new worker is asked once for the level below (`budget_down` in Rust) before it opens the files.
export function stepDown(/** @type {any} */ api) { api.pendingDown = { current: api.budgetMb || (api.choice && api.choice.mb) || 0, strikes: api.strikes }; }
export function takeDown(/** @type {any} */ api) { const d = api.pendingDown; api.pendingDown = null; return d; }

// Rust's answer to the step down (`m.mb`, 0 = there is no level below): the engine works at that level (true), or there is none (false: the engine is off).
export function lowerTo(/** @type {any} */ api, /** @type {any} */ stats, /** @type {{ mb: number }} */ m, /** @type {(api: any) => void} */ onLowered) {
  if (!(m.mb > 0)) return false;
  api.budgetMb = m.mb; stats.lowered = (stats.lowered || 0) + 1; onLowered(api);
  return true;
}

// Test hook: the count of deaths and the lowered budget start again.
export function forgetFaults(/** @type {any} */ state) {
  const { api, stats } = state; state.restarts = []; state.ooms = []; stats.restarts = 0; stats.lowered = 0; api.strikes = 0; api.pendingDown = null; api.budgetMb = 0; api.disabled = false; api.lastPanic = null;
}

// The engine stops for good: every file is marked bad and the page is told why (`key`).
export function off(/** @type {any} */ api, /** @type {any[]} */ slots, /** @type {(key: string) => void} */ notify, /** @type {string} */ key) {
  api.disabled = true; slots.forEach(s => { if (s) s.bad = true; }); notify(key);
}
