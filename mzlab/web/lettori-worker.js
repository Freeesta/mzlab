// @ts-check
// The worker of the readers module (crates/mzlab-lettori-wasm, wasm-lettori/): opens the files of a maker that the engine does not read and writes their mzML into OPFS (rust-opfs.js rawSink).
// The .wasm is fetched on the first message and checked against the SHA-256 of wasm-lettori/manifest.json. Messages: probe, toMzml, cancel (the page also ends the worker, a running conversion cannot listen).
import { rawSink } from "./rust-opfs.js";

/** @type {Promise<any> | null} */
let mod = null;
function load() {
  if (mod) return mod;
  mod = (async () => {
    const base = new URL("./wasm-lettori/", import.meta.url), man = await (await fetch(new URL("manifest.json", base))).json(), e = man.lettori;
    if (!e) throw new Error("lettori");
    const bytes = await (await fetch(new URL(e.wasm, base))).arrayBuffer();
    const sha = Array.from(new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)), b => b.toString(16).padStart(2, "0")).join("");
    if (sha !== e.sha256) throw new Error("sha");
    const m = await import(new URL(e.js, base).href);
    await m.default({ module_or_path: bytes });
    return m;
  })();
  mod.catch(() => { mod = null; });
  return mod;
}

/** `read(member, offset, length)` over some Blobs, as the bridge wants it */
function reader(/** @type {Blob[]} */ blobs) {
  const fr = new FileReaderSync();
  return (/** @type {number} */ i, /** @type {number} */ o, /** @type {number} */ n) => new Uint8Array(fr.readAsArrayBuffer(blobs[i].slice(o, o + n)));
}

/** the error as the page reads it: the JSON of the bridge, or the key of a module that did not load */
function errOf(/** @type {any} */ e) {
  try { const j = JSON.parse(String(e)); if (j && j.error_key) return j; } catch (_) { /* not JSON */ }
  return { error_key: "err.file.lettoriLoad", params: { detail: String(e && e.message || e) } };
}

/** @param {{ op: string, id: number, members: { name: string, blob: Blob }[], c?: number, dest?: string }} m */
async function run(m) {
  const post = (/** @type {any} */ x, /** @type {Transferable[]} */ t = []) => /** @type {any} */ (self).postMessage({ id: m.id, ...x }, t);
  let sink = null;
  try {
    const w = await load(), names = m.members.map(x => x.name), sizes = Float64Array.from(m.members.map(x => x.blob.size)), read = reader(m.members.map(x => x.blob));
    if (m.op === "probe") return post({ info: JSON.parse(w.probe(names, sizes, read)) });
    sink = await rawSink(/** @type {string} */ (m.dest));
    const s = sink;
    let written = 0, last = 0;
    w.to_mzml(names, sizes, read, m.c || 0, (/** @type {Uint8Array} */ p) => { written += p.length; s.write(p); }, (/** @type {number} */ scans, /** @type {number} */ scansTotal) => {
      const t = performance.now(); if (t - last < 100 && scans < scansTotal) return; last = t;
      post({ progress: { scans, scansTotal, written } });
    });
    const q = s.error(); if (q) throw q;
    const r = await s.finish(); sink = null;
    post(r.file ? { file: r.file } : { parts: r.parts });
  } catch (e) {
    if (sink) await sink.abort().catch(() => {});
    post({ error: errOf(e) });
  }
}

self.onmessage = e => { if (e.data && e.data.op !== "cancel") run(e.data); };
