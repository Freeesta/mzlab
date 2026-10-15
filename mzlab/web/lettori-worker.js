// @ts-check
// The worker of the readers modules (crates/mzlab-lettori-wasm, wasm-lettori/<family>/): opens the files of a family that the engine does not read and writes their mzML into OPFS (rust-opfs.js rawSink).
// The .wasm of a family is fetched on the first message that names it (never the others) and checked against its SHA-256 in wasm-lettori/manifest.json (carica-modulo.js). Messages: probe, toMzml, cancel (the page also ends the worker, a running conversion cannot listen).
import { rawSink } from "./rust-opfs.js";
import { loadModulo as load } from "./carica-modulo.js";

/** `read(member, offset, length)` over some Blobs, as the bridge wants it */
function reader(/** @type {Blob[]} */ blobs) {
  const fr = new FileReaderSync();
  return (/** @type {number} */ i, /** @type {number} */ o, /** @type {number} */ n) => new Uint8Array(fr.readAsArrayBuffer(blobs[i].slice(o, o + n)));
}

/** the error as the page reads it: the JSON of the bridge, or the key of a module that did not load */
function errOf(/** @type {any} */ e, /** @type {string} */ fam) {
  try { const j = JSON.parse(String(e)); if (j && j.error_key) return j; } catch (_) { /* not JSON */ }
  return { error_key: "err.file.wiffLoad", params: { family: fam, detail: String(e && e.message || e) } };
}

/** @param {{ op: string, id: number, fam: string, members: { name: string, blob: Blob }[], c?: number, dest?: string }} m */
async function run(m) {
  const post = (/** @type {any} */ x, /** @type {Transferable[]} */ t = []) => /** @type {any} */ (self).postMessage({ id: m.id, ...x }, t);
  let sink = null;
  try {
    const w = await load(m.fam), names = m.members.map(x => x.name), sizes = Float64Array.from(m.members.map(x => x.blob.size)), read = reader(m.members.map(x => x.blob));
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
    post({ error: errOf(e, m.fam) });
  }
}

self.onmessage = e => { if (e.data && e.data.op !== "cancel") run(e.data); };
