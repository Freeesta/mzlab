// @ts-check
// What this browser can do, in one place: feature detection only (never the user agent). A function that needs something the browser lacks is turned off
// by itself with the common notice (avviso-capacita.js) and comes back by itself when the browser has it: nothing is saved, the probes run at every load.
// Used by the engine bridge, the folder pickers, the 3D map; classic scripts read it as window.CAPACITA.
import { segnala } from "./avviso-capacita.js";

/** @typedef {"simd128" | "opfs" | "opfsSync" | "cartellaLocale" | "webgl2" | "isolamento" | "offscreen2d" | "locks"} Nome */

// function -> the capacities it needs (it is off only when one of ITS capacities is missing)
export const REQUISITI = /** @type {Record<string, Nome[]>} */ ({
  motore: ["simd128", "opfs", "opfsSync"],
  cartella: ["cartellaLocale"],
  mappa3d: ["webgl2"],
  indiceParallelo: ["isolamento"],
});

// WebAssembly.validate of a module with one v128.const (SIMD128): every .wasm of the site is built with it
const SIMD_MODULE = new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0, 1, 4, 1, 96, 0, 0, 3, 2, 1, 0, 10, 23, 1, 21, 0, 253, 12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 26, 11]);
const g = /** @type {any} */ (globalThis);

// one state per page, even if the module is loaded under two URLs
/** @type {{ found: Partial<Record<Nome, boolean>>, done: Promise<Record<string, boolean>> | null }} */
const state = g.__capacita || (g.__capacita = { found: {}, done: null });

// test switch: ?cap-off=a,b, MZLAB_CAP_OFF (string or array) and the older MZLAB_NO_SIMD
function forcedOff() {
  /** @type {Set<string>} */ const off = new Set();
  try { for (const n of String(new URLSearchParams(g.location && g.location.search).get("cap-off") || "").split(",")) if (n) off.add(n.trim()); } catch (_) { /* no location */ }
  const w = g.MZLAB_CAP_OFF; for (const n of Array.isArray(w) ? w : String(w || "").split(",")) if (n) off.add(String(n).trim());
  if (g.MZLAB_NO_SIMD) off.add("simd128");
  return off;
}

// synchronous probes of the page (about 1 ms in all); what lives only in workers (opfsSync, offscreen2d) is told by the engine worker with nota()
function probes() {
  /** @type {Partial<Record<Nome, boolean>>} */ const c = {};
  try { c.simd128 = WebAssembly.validate(SIMD_MODULE); } catch (_) { c.simd128 = false; }
  c.opfs = !!(g.navigator && g.navigator.storage && typeof g.navigator.storage.getDirectory === "function");
  c.cartellaLocale = typeof g.showDirectoryPicker === "function";
  c.webgl2 = typeof g.WebGL2RenderingContext === "function";
  c.isolamento = !!g.crossOriginIsolated && typeof g.SharedArrayBuffer === "function";
  c.locks = !!(g.navigator && g.navigator.locks);
  return c;
}

/** Runs the probes of the page once; the answer is the same promise afterwards (`rifai` runs them again: the cost measured by the e2e). */
export function rileva(rifai = false) {
  if (rifai) state.done = null;
  if (!state.done) { Object.assign(state.found, probes()); state.done = Promise.resolve(snapshot()); }
  return /** @type {Promise<Record<string, boolean>>} */ (state.done);
}

/** The answers known now, as {name: boolean} (a capacity still to be told by a worker is missing). */
export function snapshot() {
  /** @type {Record<string, boolean>} */ const out = {};
  for (const n of /** @type {Nome[]} */ (Object.keys(state.found))) out[n] = forcedOff().has(n) ? false : !!state.found[n];
  return out;
}

/** A worker tells what only it can see (e.g. {opfsSync: true, offscreen2d: true}). */
export function nota(/** @type {Partial<Record<Nome, boolean>>} */ cap) {
  Object.assign(state.found, cap); state.done = null; rileva();
  for (const f of Object.keys(REQUISITI)) if (stato(f) === "no") segnala(f);
}

/** "si" | "no" | "verifica" (a worker has not told yet: the function stays on). The app (desktop) has the native engine: everything is "si". */
export function statoCap(/** @type {Nome} */ nome) {
  if (g.MZLAB_DESKTOP) return "si";
  rileva();
  if (forcedOff().has(nome)) return "no";
  const v = state.found[nome];
  return v === undefined ? "verifica" : v ? "si" : "no";
}

/** True unless the browser lacks the capacity (a capacity still in "verifica" counts as present). */
export const ha = (/** @type {Nome} */ nome) => statoCap(nome) !== "no";

/** "si" | "no" | "verifica" of a function: "no" as soon as one of its capacities is missing. */
export function stato(/** @type {string} */ funzione) {
  const st = (REQUISITI[funzione] || []).map(statoCap);
  return st.includes("no") ? "no" : st.includes("verifica") ? "verifica" : "si";
}

/** True when the function can run; otherwise false, and the notice is shown (on `comando`, which gets aria-disabled, and once in the summary line). */
export function serve(/** @type {string} */ funzione, /** @type {HTMLElement | null} */ comando = null) {
  if (stato(funzione) !== "no") return true;
  segnala(funzione, comando);
  return false;
}

/** The capacities a function lacks (for the technical line of a notice). */
export const mancano = (/** @type {string} */ funzione) => (REQUISITI[funzione] || []).filter(n => statoCap(n) === "no");

rileva();
window.CAPACITA = { rileva, snapshot, nota, ha, stato, statoCap, serve, mancano, REQUISITI };
