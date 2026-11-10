// @ts-strict
// Loads the .wasm of ONE readers module (wasm-lettori/<family>/, crates/mzlab-lettori-wasm) in the worker that asks, once per worker (afresh after a wasm trap): fetched on the first request that names the family (never the others)
// and checked against its SHA-256 in wasm-lettori/manifest.json. Used by lettori-worker.js (probe, toMzml) and lettore-esterno.js (the engine's workers read a `.raw` through the `thermo` module).

/** @type {Map<string, Promise<any>>} the modules already asked for, by family */
const mods = new Map();
/** fresh glue imports after a trap, by family: the wasm-bindgen glue keeps its instance per URL, so a reload needs a URL it has not seen */
const gens = new Map();

/** @param {string} fam the family (`thermo`, `agilent`, ...) @returns {Promise<any>} the initialised wasm-bindgen module; it rejects (and may be asked for again) when the files are missing or the SHA-256 is wrong */
export function loadModulo(fam) {
  const had = mods.get(fam);
  if (had) return had;
  const p = (async () => {
    const base = new URL("./wasm-lettori/", import.meta.url), man = await (await fetch(new URL("manifest.json", base))).json(), e = man[fam];
    if (!e) throw new Error("no module for " + fam);
    const dir = new URL(fam + "/", base), bytes = await (await fetch(new URL("mzlab_lettori_wasm_bg.wasm", dir))).arrayBuffer();
    const sha = Array.from(new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)), b => b.toString(16).padStart(2, "0")).join("");
    if (sha !== e.sha256) throw new Error("sha");
    const g = gens.get(fam) || 0, m = await import(new URL("mzlab_lettori_wasm.js" + (g ? "?g=" + g : ""), dir).href);
    await m.default({ module_or_path: bytes });
    return m;
  })();
  mods.set(fam, p);
  p.catch(() => { mods.delete(fam); });
  return p;
}

/** after a call that failed: a wasm trap (RuntimeError) leaves the instance broken (memory, allocator), so the family is forgotten and the next request loads a fresh one; other errors keep it. @returns {boolean} whether it was forgotten */
export function forgetIfTrapped(/** @type {string} */ fam, /** @type {any} */ e) {
  if (!(e instanceof WebAssembly.RuntimeError)) return false;
  mods.delete(fam); gens.set(fam, (gens.get(fam) || 0) + 1);
  return true;
}
/** `f` (a call into the module of `fam`) that forgets the family when it traps, so the next `loadModulo` gives a fresh instance (`onTrap` drops the caller's own copy); the error goes on as it was */
export function trapGuard(/** @type {string} */ fam, /** @type {Function} */ f, onTrap = () => {}) {
  return (/** @type {any[]} */ ...a) => { try { return f(...a); } catch (e) { if (forgetIfTrapped(fam, e)) onTrap(); throw e; } };
}
