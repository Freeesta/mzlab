// @ts-check
// The readers modules in the page (docs/agenti/lettori-modulo.md): `LETTORI_MOD` opens, in a worker of its own (lettori-worker.js, started on the first request; each family's .wasm loaded when a file of it arrives), the files of a family that
// the engine does not read, and gives their mzML as a File of OPFS. `shell.js moduloToMzml` (rule R of upload0) decides which files go here. Module loaded by carica.js.

/** @typedef {{ name: string, blob: Blob }} Member */
/** @typedef {{ formato: string, campioni: { id: number, name: string, scans: number }[], motivi: string[] }} Probe */

/** @type {Record<string, { ext: string[], membri: string[], errori: string[], soloModulo: string[] }>} */
const FAM = /** @type {any} */ (window).LETTORI_FAM;

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
  const e = Object.assign(new Error("err.file.wiffLoad"), { json: { error: "err.file.wiffLoad", error_key: "err.file.wiffLoad", params: { family: "", detail: "worker" } } });
  for (const p of wait.values()) p.no(e);
  wait.clear();
}
function ask(/** @type {any} */ msg, /** @type {((p: any) => void) | undefined} */ tell = undefined) {
  const w = start(), id = ++seq;
  return new Promise((ok, no) => { wait.set(id, { ok, no, tell }); w.postMessage({ ...msg, id }); });
}
const cancelled = () => Object.assign(new Error("cancelled"), { cancelled: true });

const LETTORI_MOD = {
  /** true once the worker has been started (a probe is asked only then, or when it is needed) */
  get loaded() { return !!worker; },
  /** the extensions only the modules read (every family's `soloModulo`) */
  extensions: () => Object.values(FAM).flatMap(f => f.soloModulo),
  /** the family whose module takes an error of the engine (`err.file.agilent` -> "agilent"), or "" */
  retriable: (/** @type {string | undefined} */ key) => Object.keys(FAM).find(k => FAM[k].errori.includes(String(key))) || "",
  /** the family of a dataset from the names of its members (extension, or a part of the path), or "" */
  famigliaDi(/** @type {string[]} */ names) {
    const n = names.map(x => x.toLowerCase());
    return Object.keys(FAM).find(k => n.some(x => FAM[k].ext.some(e => x.endsWith(e)) || FAM[k].membri.some(m => x.includes(m)))) || "";
  },
  /** the name of the mzML made from `name`: "x.ext" -> "x.mzML", or "x.ext.mzML" when that name is taken (names in any case) */
  mzmlName(/** @type {string} */ name, /** @type {string[]} */ taken) {
    const base = name.replace(/\.[^./]+$/, ""), used = new Set(taken.map(n => n.toLowerCase()));
    return used.has((base + ".mzML").toLowerCase()) ? name + ".mzML" : base + ".mzML";
  },
  /** the File that stands for a sample the engine refused, in the list of the files until `shell.js moduloToMzml` converts it (members: [{ name, blob }]) */
  stand(/** @type {string} */ name, /** @type {Member[]} */ members, /** @type {number} */ c, /** @type {string} */ fam) {
    const f = new File([JSON.stringify({ mzlab: "modulo1" })], name);
    Object.defineProperty(f, "mzlabModulo", { value: { members, c, fam, size: members.reduce((a, m) => a + m.blob.size, 0) } });
    return f;
  },
  /** what the module sees in a dataset */
  async probe(/** @type {string} */ fam, /** @type {Member[]} */ members) { return /** @type {Probe} */ ((await ask({ op: "probe", fam, members })).info); },
  /** the mzML of sample `c` of a dataset as a File of OPFS (a Blob without OPFS); `signal` ends the worker, so the conversion stops at once */
  async toMzml(/** @type {string} */ fam, /** @type {Member[]} */ members, /** @type {number} */ c, /** @type {string} */ dest, /** @type {((p: any) => void) | undefined} */ onProgress, /** @type {AbortSignal | undefined} */ signal) {
    if (signal && signal.aborted) throw cancelled();
    const w = start(), drop = () => navigator.storage.getDirectory().then(d => d.getDirectoryHandle("mzlab")).then(d => d.getDirectoryHandle("conv")).then(d => d.removeEntry(dest)).catch(() => {});
    const onAbort = () => { w.postMessage({ op: "cancel" }, []); w.terminate(); if (w === worker) worker = null; const e = cancelled(); for (const p of wait.values()) p.no(e); wait.clear(); drop(); };
    if (signal) signal.addEventListener("abort", onAbort, { once: true });
    try {
      const r = await ask({ op: "toMzml", fam, members, c, dest }, onProgress);
      return r.file || new Blob(r.parts, { type: "application/xml" });
    } finally { if (signal) signal.removeEventListener("abort", onAbort); }
  },
};
/** @type {any} */ (window).LETTORI_MOD = LETTORI_MOD;
export default LETTORI_MOD;
