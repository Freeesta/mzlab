// @ts-check
// The readers module in the page (docs/agenti/lettori-modulo.md): `LETTORI_MOD` opens, in a worker of its own (lettori-worker.js, started on the first request), the files of a maker that
// the engine does not read, and gives their mzML as a File of OPFS. `shell.js moduloToMzml` (rule R of upload0) decides which files go here. Module loaded by carica.js.

/** @typedef {{ name: string, blob: Blob }} Member */
/** @typedef {{ formato: string, campioni: { id: number, name: string, scans: number }[], motivi: string[] }} Probe */

/** extensions of formats that only the module reads (lower case, with the dot); each package that adds a format adds its own here */
const EXT = /** @type {string[]} */ ([]);
/** the keys of the errors of the engine for which the module tries again */
const RETRY = ["err.file.agilent", "err.file.wiff", "err.file.numpress"];

/** @type {Worker | null} */ let worker = null;
let seq = 0;
/** @type {Map<number, { ok: (r: any) => void, no: (e: any) => void, tell?: (p: any) => void }>} */
const wait = new Map();

function start() {
  if (worker) return worker;
  const w = new Worker(new URL("lettori-worker.js", import.meta.url), { type: "module" });
  w.onmessage = e => {
    const { id, progress, ...r } = e.data, p = wait.get(id);
    if (!p) return;
    if (progress) { if (p.tell) p.tell(progress); return; }
    wait.delete(id);
    if (r.error) p.no(Object.assign(new Error(r.error.error_key), { json: { error: r.error.error_key, ...r.error } }));
    else p.ok(r);
  };
  w.onerror = () => fail(w);
  return (worker = w);
}
function fail(/** @type {Worker} */ w) {
  if (w !== worker) return;
  worker = null;
  const e = Object.assign(new Error("err.file.lettoriLoad"), { json: { error: "err.file.lettoriLoad", error_key: "err.file.lettoriLoad", params: {} } });
  for (const p of wait.values()) p.no(e);
  wait.clear();
}
function ask(/** @type {any} */ msg, /** @type {((p: any) => void) | undefined} */ tell = undefined) {
  const w = start(), id = ++seq;
  return new Promise((ok, no) => { wait.set(id, { ok, no, tell }); w.postMessage({ ...msg, id }); });
}
const cancelled = () => Object.assign(new Error("cancelled"), { cancelled: true });

const LETTORI_MOD = {
  /** true once the worker has been started (a probe for the sets of .raw/.wiff/.D is asked only then, or when it is needed) */
  get loaded() { return !!worker; },
  /** the extensions only the module reads */
  extensions: () => EXT.slice(),
  /** true when `key` is an error of the engine that the module may not have */
  retriable: (/** @type {string | undefined} */ key) => RETRY.includes(String(key)),
  /** the name of the mzML made from `name`: "x.ext" -> "x.mzML", or "x.ext.mzML" when that name is taken (names in any case) */
  mzmlName(/** @type {string} */ name, /** @type {string[]} */ taken) {
    const base = name.replace(/\.[^./]+$/, ""), used = new Set(taken.map(n => n.toLowerCase()));
    return used.has((base + ".mzML").toLowerCase()) ? name + ".mzML" : base + ".mzML";
  },
  /** the File that stands for a sample the engine refused, in the list of the files until `shell.js moduloToMzml` converts it (members: [{ name, blob }]) */
  stand(/** @type {string} */ name, /** @type {Member[]} */ members, /** @type {number} */ c) {
    const f = new File([JSON.stringify({ mzlab: "modulo1" })], name);
    Object.defineProperty(f, "mzlabModulo", { value: { members, c, size: members.reduce((a, m) => a + m.blob.size, 0) } });
    return f;
  },
  /** what the module sees in a dataset */
  async probe(/** @type {Member[]} */ members) { return /** @type {Probe} */ ((await ask({ op: "probe", members })).info); },
  /** the mzML of sample `c` of a dataset as a File of OPFS (a Blob without OPFS); `signal` ends the worker, so the conversion stops at once */
  async toMzml(/** @type {Member[]} */ members, /** @type {number} */ c, /** @type {string} */ dest, /** @type {((p: any) => void) | undefined} */ onProgress, /** @type {AbortSignal | undefined} */ signal) {
    if (signal && signal.aborted) throw cancelled();
    const w = start(), drop = () => navigator.storage.getDirectory().then(d => d.getDirectoryHandle("mzlab")).then(d => d.getDirectoryHandle("conv")).then(d => d.removeEntry(dest)).catch(() => {});
    const onAbort = () => { w.postMessage({ op: "cancel" }, []); w.terminate(); if (w === worker) worker = null; const e = cancelled(); for (const p of wait.values()) p.no(e); wait.clear(); drop(); };
    if (signal) signal.addEventListener("abort", onAbort, { once: true });
    try {
      const r = await ask({ op: "toMzml", members, c, dest }, onProgress);
      return r.file || new Blob(r.parts, { type: "application/xml" });
    } finally { if (signal) signal.removeEventListener("abort", onAbort); }
  },
};
/** @type {any} */ (window).LETTORI_MOD = LETTORI_MOD;
export default LETTORI_MOD;
