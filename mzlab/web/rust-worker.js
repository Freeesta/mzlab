// Web Worker of the Rust engine (?motore=rust): mzlab-wasm opens mzML files in blocks and answers TIC, XIC and single scans as
// Float64Arrays that are moved (not copied) to the page. The page gives the arrays back ("recycle") once drawn, so a frame
// allocates nothing. If this worker dies, rust-bridge.js starts a new one (the WASM instance cannot be reused after a trap).
import init, { Engine, native_samples, native_to_mzml, native_srm, native_external, Passata, version, prepare_upload, formula_mz, provenance, composition, stat_post, campioni, mrm_diff_post } from "./wasm/mzlab_wasm.js";
import * as ARCH from "./archivio.js";
import { lab } from "./qqq-db.js";
import { migra, rowOf } from "./migra.js";
import { limpia } from "./pulizia.js";
import { infoDi, nuovo } from "./manifesto.js";
import { createPool, poolSize, makeGate } from "./idxpool.js";
import { createLabOps } from "./rust-worker-lab.js";
import { createNativo } from "./rust-nativo.js";
import { createImpronte } from "./impronta.js";
import { rawSink } from "./rust-opfs.js";

let engine = null, wasmMem = null;
const pool = [];                       // ArrayBuffers handed back by the page, ready to be written again
const blobs = new Map();               // slot -> Blob (read synchronously, block by block, by the WASM reader)

const take = len => {
  const need = len * 8;
  const i = pool.findIndex(b => b.byteLength >= need);
  return new Float64Array(i >= 0 ? pool.splice(i, 1)[0] : new ArrayBuffer(Math.max(need, 64 << 10)), 0, len);
};
// The page compiles the .wasm once and sends it first ({ op: "module", module }; null = compile it here); any other first message starts it too.
let startInit = /** @type {(m: any) => void} */ (() => {}), wasmModule = null;
const started = new Promise(res => { startInit = res; });
const ready = started.then(module => { wasmModule = module || null; return module ? init({ module_or_path: module }) : init(); }).then(ex => { wasmMem = ex.memory; engine = new Engine(); }).then(() => within(archiveReady, 5000)).then(() => postMessage({ op: "ready" }));
ready.then(() => fetch(new URL("./idx-worker.js", import.meta.url)).catch(() => {}));      // the index builder script goes into the service worker cache: an upload works offline too

// The pyramid of map tiles of a level of a file (`maptiles.rs`) is kept in OPFS (mzlab/tiles/<file>.L<level>.mzt) beside the index: the first
// map of a file builds it from the index, the next visits read it. The engine refuses bytes of another index (its stamp), so a stale file is harmless.
/** @type {Map<number, string>} */ const tileName = new Map();           // slot -> name of the file, set when its index is opened
/** @returns {Promise<any>} */ const tilesDir = async () => (await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab", { create: true })).getDirectoryHandle("tiles", { create: true });
const tilesFile = (/** @type {string} */ name, /** @type {number} */ level) => `${name}.L${level}.mzt`;
async function tilesEnsure(/** @type {string} */ query) {                                  // before api/maptiles: load the saved pyramid; after: save the one just built
  const q = new URLSearchParams(query), slot = Number(q.get("k")), level = Number(q.get("level") || 1), name = tileName.get(slot);
  if (!name || !Number.isInteger(slot) || engine.tiles_held(slot, level)) return null;
  try {
    const f = await (await (await tilesDir()).getFileHandle(tilesFile(name, level))).getFile();
    if (engine.tiles_import(slot, level, new Uint8Array(await f.arrayBuffer()))) return null;
  } catch (_) { /* none saved yet */ }
  return { slot, level, name };
}
async function tilesSave(/** @type {{ slot: number, level: number, name: string }} */ t) {
  try {
    const bytes = engine.tiles_export(t.slot, t.level);
    if (!bytes.length) return;
    const h = await (await tilesDir()).getFileHandle(tilesFile(t.name, t.level), { create: true }), ah = await h.createSyncAccessHandle();
    try { ah.truncate(0); if (ah.write(bytes, { at: 0 }) < bytes.length) { ah.close(); await (await tilesDir()).removeEntry(tilesFile(t.name, t.level)); return; } ah.flush(); } finally { try { ah.close(); } catch (_) { /* closed */ } }
  } catch (_) { /* no OPFS or no room: the pyramid is built again next time */ }
}
async function mapOp(/** @type {any} */ m) {                                                // the map routes of the session; maptiles loads its saved pyramid first and saves a new one after
  const tl = m.path === "/api/maptiles" ? await tilesEnsure(m.query) : null;
  const info = engine.map_route(m.path, m.items, m.query);
  if (tl) await tilesSave(tl);
  return info;
}
async function tilesDrop(/** @type {string | null} */ name, /** @type {Set<string> | null} */ keep) {   // a file's pyramids (name), all of them (null, null) or those of files not in `keep`
  try {
    const d = await tilesDir();
    for await (const [n] of d.entries()) { const own = n.replace(/\.L\d+\.mzt$/, ""); if (name ? own === name : (!keep || !keep.has(own))) await d.removeEntry(n).catch(() => {}); }
  } catch (_) { /* nothing saved */ }
}

// `held` = [kind, name] of a file with a read lease in archivio.js (the index in use): read straight from its SyncAccessHandle into a buffer reused by the next read (the WASM
// copies it at once); no handle, or a short read -> the FileReaderSync of the Blob. `rdStat` counts the time spent inside the readers (sent with every answer: tools/bench_mappa.py)
const rdStat = { ms: 0, n: 0, bytes: 0, hand: 0 };
function readerFor(blob, held) {
  const fr = new FileReaderSync();
  let buf = held ? new Uint8Array(1 << 20) : null;
  return (offset, length) => {
    const t = performance.now();
    let out = null;
    if (held) {
      if (buf.length < length) buf = new Uint8Array(length);
      const o = buf.subarray(0, length);
      if (ARCH.leaseRead(held[0], held[1], o, offset) === length) { out = o; rdStat.hand += length; }
    }
    if (!out) out = new Uint8Array(fr.readAsArrayBuffer(blob.slice(offset, offset + length)));
    rdStat.ms += performance.now() - t; rdStat.n++; rdStat.bytes += length;
    return out;
  };
}


// ---- the flow of the page (lr.js): the mzML files and their indexes live in OPFS (archivio.js), the list of the files, the items of the
// session and the state of the page come from here. The slots of these files are numbered above LR_BASE, apart from the ones of the session (k).
const LR_BASE = 1000000;
const lrSlot = new Map(), lrFile = new Map();
let lrN = 0, lrPool = null;
const slotOf = n => { if (!lrSlot.has(n)) lrSlot.set(n, LR_BASE + ++lrN); return lrSlot.get(n); };
const SMALL = (() => { try { return /iPad|iPhone|Android/i.test(navigator.userAgent) || (navigator.platform === "MacIntel" && navigator.maxTouchPoints > 1); } catch (_) { return false; } })();
const gate = makeGate(poolSize(SMALL, navigator.hardwareConcurrency));                // the workers of the pool count against it (2 on iPad/phone)
const idxPool = () => lrPool || (lrPool = createPool({ workerUrl: new URL("./idx-worker.js", import.meta.url), moduleUrl: new URL("./wasm/mzlab_wasm_bg.wasm", import.meta.url), module: wasmModule, gate, size: poolSize(SMALL, navigator.hardwareConcurrency), budget: SMALL ? 32 : 64, notify: key => postMessage({ op: "notice", key }) }));
const jerr = (key, params = {}) => { throw JSON.stringify({ error_key: key, params }); };
async function fileOf(name) {
  if (!lrFile.has(name)) { const f = await ARCH.get("files", name); if (!f) jerr("err.session.fileNotFound"); lrFile.set(name, f); }
  return lrFile.get(name);
}
const nativo = createNativo({ ARCH, readerFor, fileOf: async n => { try { return await fileOf(n); } catch (_) { return null; } }, jerr, native_samples, native_to_mzml, native_srm, native_external, impronte: createImpronte({ Passata, workerUrl: new URL("./impronta-worker.js", import.meta.url), module: () => wasmModule }) });
async function lrOpen(name) {                                             // opens the index of a file in its slot; false when there is none
  const f = await fileOf(name), ix = await ARCH.get("idx", name + ".mzidx");
  if (!ix) return false;
  await ARCH.lease("idx", name + ".mzidx");
  engine.open_index(slotOf(name), ix.size, readerFor(ix, ["idx", name + ".mzidx"]));
  await nativo.attach(engine, name, slotOf(name));                         // a sample of a maker's file (rust-nativo.js): the routes read its reader
  return true;
}
// IndexedDB of the page ("qqq_lab"): the notebook and, without OPFS, the files
const idb = lab(() => postMessage({ op: "blocked" }));      // another tab holds the database: the page says so
const tx = async (store, mode, fn) => {
  const db = await idb();
  return new Promise((res, rej) => { const t = db.transaction(store, mode), s = t.objectStore(store), out = fn(s); t.oncomplete = () => res(out && out.result !== undefined ? out.result : undefined); t.onerror = t.onabort = () => rej(t.error); });
};
const safe = async f => { try { return await f(); } catch (_) { return undefined; } };
const kvGet = key => safe(() => tx("kv", "readonly", s => s.get(key)));
// the archive: its schema is brought up to date before any file is opened, and the manifest follows every change of the folders of OPFS
const infoOf = async (kind, name, fresh) => {
  const i = kind === "files" ? null : await safe(async () => infoDi(kind, await ARCH.get(kind, name)));
  return i && fresh && !i.crate ? { ...i, crate: version() } : i;                  // an index just written by this build: the meta of the .mzidx has no crate
};
const archiveReady = (async () => {
  try {
    ARCH.watch(async (ev, kind, name, size) => { if (ev === "del") await tx("manifest", "readwrite", s => s.delete(kind + "/" + name)); else { const info = await infoOf(kind, name, true); await tx("manifest", "readwrite", s => s.put(rowOf(kind, name, size, undefined, info))); } });
    await migra({ getSchema: () => kvGet("schema"), setSchema: n => tx("kv", "readwrite", s => s.put(n, "schema")), list: kind => ARCH.list(kind), putManifest: row => tx("manifest", "readwrite", s => s.put(row)) });
  } catch (_) { /* the archive works without the manifest; the next start tries again */ }
})();
// when idle, a little after the start: what is left over is cleaned (pulizia.js), then the other tabs are told
setTimeout(() => safe(async () => {
  await archiveReady;
  const rows = await safe(() => tx("manifest", "readonly", s => s.getAll())) || [];
  await limpia({ list: k => ARCH.list(k), remove: (k, n) => ARCH.remove(k, n), manifest: async () => rows, info: infoOf,
    annota: (k, e, info) => tx("manifest", "readwrite", s => s.put(rowOf(k, e.name, e.size, e.lastModified, info))) });
}), 8000);
const within = (p, ms) => Promise.race([p, new Promise(r => setTimeout(r, ms))]);              // a database held by another tab never answers: «start from scratch» does not wait for it
async function lrSpace() {
  const files = await safe(() => ARCH.list("files")) || [], idx = await safe(() => ARCH.list("idx")) || [];
  const names = new Set(files.map(f => f.name)), gone = idx.filter(e => !names.has(e.name.replace(/(\.cen)?\.mzidx$/, "")));
  const sum = l => l.reduce((a, e) => a + e.size, 0), est = await ARCH.estimate();
  let persisted = false; try { persisted = !!(await navigator.storage.persisted()); } catch (_) { /* optional */ }
  const feat = await safe(() => ARCH.list("feat")) || [];
  return { opfs: true, files: sum(files), idx: sum(idx), feat: sum(feat), idx_removed: sum(gone), n_removed: gone.length, usage: est.usage, quota: est.quota, persisted };
}
async function lrFree() {                                                 // the indexes of the files that were taken away go first
  const files = new Set((await safe(() => ARCH.list("files")) || []).map(f => f.name));
  for (const e of await safe(() => ARCH.list("idx")) || []) if (!files.has(e.name.replace(/(\.cen)?\.mzidx$/, ""))) await ARCH.remove("idx", e.name);
  await tilesDrop(null, files);
}
async function lrFreeDerived() {                                         // every index and pipeline file can be made again from the sources: all of them go
  for (const kind of ["idx", "feat"]) for (const e of await safe(() => ARCH.list(kind)) || []) await safe(() => ARCH.remove(kind, e.name));
  await tilesDrop(null, null);
}
// Operations on single scans (`scans.rs` of the engine): the scans of a centroid file come from the mzML by offset, a background comes from the indexes.
function scanOp({ op, id, slot, path, query, blob }) {
  const q = query || "";
  if (op === "scanbin") {
    const b = engine.scanbin(slot, q, readerFor(blob));
    postMessage({ id, op, n: b.length, info: null, buf: b.length ? b.buffer : null, len: b.length }, b.length ? [b.buffer] : []);
    return;
  }
  const info = op === "specbg" ? engine.spectrum_bg(slot, q) : engine.mzml_route(slot, path, q, readerFor(blob));
  postMessage({ id, op, n: 0, info, buf: null, len: 0 });
}

const labOp = createLabOps({ engine: () => engine, ARCH, fileOf, lrOpen, slotOf, readerFor, jerr });      // methods (.dam) and the routes of one item about its acquisition
async function lrMigrate() {                                              // what an earlier visit left in IndexedDB passes to OPFS
  for (const n of (await safe(() => tx("files", "readonly", s => s.getAllKeys()))) || []) {
    const buf = await safe(() => tx("files", "readonly", s => s.get(n)));
    if (!buf) continue;
    try { await ARCH.put("files", n, buf instanceof Blob ? buf : new Uint8Array(buf)); await safe(() => tx("files", "readwrite", s => s.delete(n))); } catch (e) { /* it stays where it is */ }
  }
}
async function lrPut(m) {                                              // an uploaded file: into OPFS, replacing the one of the same name and its index
  if (!m.nat) await ARCH.ensureRoom(m.blob.size, m.name);                 // room first (error.quota with the MB needed and free), never write hoping; held for the file and its index until lr_index ends
  idxPool().cancel(m.name);
  engine.close(slotOf(m.name));
  for (const ix of [m.name + ".mzidx", m.name + ".cen.mzidx"]) {         // an index of a newer program stays where it is (lr_index then answers error.newerFormat)
    const cur = await ARCH.get("idx", ix);
    if (!(cur && nuovo("idx", await safe(() => infoDi("idx", cur))))) await ARCH.remove("idx", ix);
  }
  await tilesDrop(m.name, null);
  let f;
  if (m.nat) {                                                        // a sample of a maker's file: its members go to OPFS (the progress is the copy), the file of the program is the stub
    f = await nativo.put(m, frac => postMessage({ op: "progress", id: m.id, frac, phase: "read" }));
    lrFile.set(m.name, f);
    await ARCH.persistOnce();
    return { info: JSON.stringify({ size: f.size }), files: [[m.name, f]] };
  }
  try { f = await ARCH.put("files", m.name, m.blob); }
  catch (e) { if (/quota/i.test(String(e && (e.name || e.message)))) { await lrFree(); f = await ARCH.put("files", m.name, m.blob); } else { ARCH.release(m.name); throw e; } }
  lrFile.set(m.name, f);
  if (/\.dam$/i.test(m.name)) ARCH.release(m.name);                    // a method has no index
  await ARCH.persistOnce();
  return { info: JSON.stringify({ size: f.size }), files: [[m.name, f]] };
}
async function lrOp(m) {
  switch (m.op) {
    case "lr_boot": {                                                     // every file in OPFS, with its size, whether it has an index, and the File itself for the bridge
      if (!m.fresh) await lrMigrate(); const out = [];
      for (const e of await ARCH.list("files")) {
        if (/\.part$/.test(e.name)) continue;
        const f = await ARCH.get("files", e.name); if (!f) continue;
        lrFile.set(e.name, f);
        const ok = /\.mzml$/i.test(e.name) ? await safe(() => lrOpen(e.name)) : false;
        const st = /\.mzml$/i.test(e.name) ? await nativo.stubOf(e.name) : null;
        out.push({ name: e.name, size: st ? st.size : f.size, indexed: !!ok, nat: !!st, file: f });
      }
      return { info: JSON.stringify(out.map(({ file, ...r }) => r)), files: out.map(o => [o.name, o.file]) };
    }
    case "lr_put": return lrPut(m);
    case "lr_index": try {                                                    // the scan table (Rust), the index (a short-lived worker of the pool), then the index is opened
      const kept = await ARCH.get("idx", m.name + ".mzidx"), keptInfo = kept && await safe(() => infoDi("idx", kept));
      if (nuovo("idx", keptInfo)) jerr("error.newerFormat", { file: m.name, version: keptInfo.versione_formato });      // an index of a newer program stays as it is: no new one is built over it
      const f = await fileOf(m.name), t0 = performance.now(), job = await nativo.indexJob(m.name), part = m.name + ".mzidx.part", partCen = job && m.cen ? m.name + ".cen.mzidx.part" : undefined;
      const onProgress = p => postMessage({ op: "progress", id: m.id, frac: p.frac, phase: p.phase });
      const res = await idxPool().build(job ? { name: m.name, nat: job, part, partCen, onProgress }
        : { name: m.name, blob: f, inputs: Array.from(prepare_upload(f.size, readerFor(f), m.name)).slice(0, 8), part, onProgress });      // frac: how far the builder has read
      if (partCen) {                                                                // the centroid index of a profile sample, made in the same decoding; for a sample of centroids the builder wrote nothing
        if (res.ok && res.stats && res.stats.cen) await ARCH.promote("idx", partCen, m.name + ".cen.mzidx"); else await ARCH.remove("idx", partCen);
      }
      if (!res.ok) {                                                                // the true cause reaches the page: the error of the builder (a key), else its text
        await ARCH.remove("idx", part);
        if (res.reason === "memory") jerr("error.memoryBudget");
        let key = ""; try { key = JSON.parse(res.text).error_key || ""; } catch (_) { /* not JSON */ }
        if (key) throw res.text;
        throw JSON.stringify({ error_key: res.reason === "crash" ? "err.engine.indexCrash" : "err.engine.indexWhy", params: { detail: String(res.text || res.reason).slice(0, 200) } });
      }
      await ARCH.promote("idx", part, m.name + ".mzidx");
      let opened = false;
      try { opened = await lrOpen(m.name); } catch (_) { await ARCH.remove("idx", m.name + ".mzidx"); jerr("error.badIndex"); }      // an index that does not open (incomplete, damaged) is not kept
      if (!opened) jerr("err.engine.index");
      return { info: JSON.stringify({ ms: Math.round(performance.now() - t0) }) };
    } finally { ARCH.release(m.name); }
    case "lr_index_cen": try {                                                // mzFinder works on centroids: the index of a profile sample with its scans as centroids, apart from the one of the Data tab
      const job = await nativo.indexJob(m.name), part = m.name + ".cen.mzidx.part";
      if (!job) jerr("err.session.fileNotFound");
      const res = await idxPool().build({ name: m.name, nat: { ...job, st: { ...job.st, centroid: true } }, part });
      if (!res.ok) { await ARCH.remove("idx", part); throw JSON.stringify({ error_key: "err.engine.indexWhy", params: { detail: String(res.text || res.reason).slice(0, 200) } }); }
      await ARCH.promote("idx", part, m.name + ".cen.mzidx");
      return { info: "{}" };
    } finally { ARCH.release(m.name); }
    case "lr_entry": {
      const f = await fileOf(m.name); if (!(await lrOpen(m.name))) jerr("err.engine.index");
      const info = engine.lr_entry(slotOf(m.name), m.name, f.size, readerFor(f)), st = await nativo.stubOf(m.name);
      return { info: st ? JSON.stringify({ ...JSON.parse(info), size: st.size, nat: true }) : info };      // the size of a sample of a maker's file is the one of its members
    }
    case "lr_items": { const f = await fileOf(m.name); if (!(await lrOpen(m.name))) jerr("err.engine.index"); return { info: engine.lr_items(slotOf(m.name), m.name, f.size, readerFor(f), JSON.stringify(m.sample || {})) }; }
    case "lr_composition": return { info: composition(m.query || "") };                  // "" = Python's (it words the error)
    case "lr_stat": return { info: stat_post(m.path || "", m.body || "") };               // the statistics routes; "E" + JSON = an error, "" = not one of them
    case "lr_provenance": return { info: provenance(m.ctx || "") };
    case "lr_campioni": return { info: campioni(m.cop || "", m.body || "") };                       // campioni/2 (proponi, valida, ...): JSON in, JSON out
    case "lr_formula": return { info: formula_mz(m.f || "", m.adduct || "") };
    case "lr_remove": {
      engine.close(slotOf(m.name)); lrFile.delete(m.name); nativo.forget(m.name); await ARCH.remove("files", m.name); await tilesDrop(m.name, null); await safe(lrFree);      // the indexes of the file go with it
      await safe(async () => nativo.sweep((await ARCH.list("files")).filter(e => /\.mzml$/i.test(e.name)).map(e => e.name)));      // the members of a maker's file that no sample uses any more go
      return { info: "{}" };
    }
    case "lr_space": { if (m.free) await (m.free === "derived" ? lrFreeDerived() : lrFree()); return { info: JSON.stringify(await lrSpace()) }; }
    case "lr_kv_get": { const v = await safe(() => tx("kv", "readonly", s => s.get(m.key))); return { info: JSON.stringify({ value: v === undefined ? null : v }) }; }
    case "lr_kv_set": { await safe(() => tx("kv", "readwrite", s => s.put(m.value, m.key))); return { info: "{}" }; }
    case "lr_clear": {                                                    // «New session» with fresh = true: files, indexes, notebook
      for (const n of [...lrFile.keys()]) engine.close(slotOf(n));
      lrFile.clear();
      await safe(async () => { await ARCH.clear("files"); await ARCH.clear("idx"); await tilesDrop(null, null); });
      await within(safe(() => tx("files", "readwrite", s => s.clear())), 5000); await within(safe(() => tx("kv", "readwrite", s => s.clear())), 5000);
      return { info: "{}" };
    }
  }
  return null;
}

// linear memory of the WASM (it only grows): the page recycles the worker when it is large and idle (rust-bridge.js). The cache is trimmed to its budget first (one LRU).
const memNow = () => { try { if (engine) engine.trim_cache(); } catch (_) { /* the answer goes out anyway */ } return wasmMem ? wasmMem.buffer.byteLength : 0; };
let active = 0;                                                                   // requests in flight: the read handles are exclusive, so they are released when the worker is idle
onmessage = async ev => { active++; try { await handle(ev); } finally { if (--active === 0) await ARCH.parkAll(); } };
async function handle(ev) {
  const m = ev.data;
  if (m.op === "module") { startInit(m.module); return; } startInit(null);
  if (m.op === "recycle") { for (const b of m.bufs) if (b.byteLength) pool.push(b); if (pool.length > 8) pool.splice(0, pool.length - 8); return; }
  await ready;
  await ARCH.wake();                                                              // the read handles parked while idle are reopened before the request reads
  const { id } = m;
  try {
    let out = null, n = 0, info = null;
    if (m.op.startsWith("lr_")) {
      const r = await labOp(m) || await lrOp(m);
      postMessage({ id, op: m.op, n: 0, info: r.info, buf: null, len: 0, files: r.files || null, mem: memNow(), rd: rdStat });
      return;
    }
    switch (m.op) {
      case "open": blobs.set(m.slot, m.blob); info = engine.open(m.slot, m.blob.size, readerFor(m.blob)); break;
      case "mzml2cen": {                                                              // a profile mzML of the archive -> a copy with one point per peak on OPFS (centroidi/convert.rs; read twice, never held)
        const f = await fileOf(m.name), rd = readerFor(f), sink = await rawSink(m.dest);
        let written = 0, last = 0, scans = 0, scansTotal = null, did = false;
        const tick = force => { const t = performance.now(); if (force || t - last > 150) { last = t; postMessage({ op: "progress", id, total: f.size, written, scans, scansTotal }); } };
        try { did = engine.mzml_to_centroids(m.name, f.size, rd, piece => { sink.write(piece); written += piece.length; tick(false); }, (d, t) => { scans = d; scansTotal = t; tick(d === 0 || d === t); }); }
        catch (e) { const q = await sink.error(); await sink.abort(); throw q || e; }
        if (!did) { await sink.abort(); postMessage({ id, op: m.op, none: true }); return; }     // no profile spectrum: nothing to reduce
        const r = await sink.finish();
        postMessage({ id, op: m.op, n: r.parts ? r.parts.length : 1, parts: r.parts || null, file: r.file || null }, r.parts ? r.parts.map(a => a.buffer) : []);
        return;
      }
      case "natsamples": info = await nativo.samples(m.members); break;                      // the samples of a maker's dataset (a .wiff holds several), from its members
      case "nat2mzml": {                                                              // «Download .mzML»: one sample of a maker's file as mzML on OPFS, written piece by piece
        const sink = await nativo.toMzml(m, p => postMessage({ op: "progress", id, ...p }));
        postMessage({ id, op: m.op, n: sink.parts ? sink.parts.length : 1, parts: sink.parts || null, file: sink.file || null }, sink.parts ? sink.parts.map(a => a.buffer) : []);
        return;
      }
      case "natregistro": info = await nativo.registro(String(m.name).split("#")[0], m.id); break;       // one series of the instrument log of a .raw (status log, scan trailer), on demand
      case "natsrm": {                                                                // the SRM traces of a sample of a maker's file, for mzFinder (no mzML)
        const b = await nativo.srm(m.name);
        postMessage({ id, op: m.op, srm: b }, b ? [b.buffer] : []);
        return;
      }
      case "idxopen": {                                                               // the .mzidx of a file, read from OPFS where the index builder put it (no mzML is read)
        let blob;
        try { blob = await (await (await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab")).getDirectoryHandle("idx")).getFileHandle(m.name + ".mzidx").then(h => h.getFile()); }
        catch (_) { throw '{"error_key":"error.noIndex","params":{}}'; }                  // no OPFS or no index yet: not a fault of the engine, the page uses the mzML
        await ARCH.lease("idx", m.name + ".mzidx");
        info = engine.open_index(m.slot, blob.size, readerFor(blob, ["idx", m.name + ".mzidx"])); engine.set_polarity(m.slot, m.pol | 0); tileName.set(m.slot, m.name); await nativo.attach(engine, m.name, m.slot); break;
      }
      case "route": info = engine.route(m.slot, m.path, m.query || ""); break;       // "" = not answered by the index
      case "scaninfo": info = engine.scaninfo(m.slot, m.sid, m.blob.size, readerFor(m.blob)); break;     // header of one scan: the index row + a slice of the mzML
      case "mzmlroute": case "specbg": case "scanbin": return scanOp(m, id);                            // api/spectra, scan, scanavg, scanbin, spectrum with a background: see scanOp
      case "specexact": info = engine.spectrum_exact(m.slot, m.query || "", readerFor(m.blob)); break;   // api/spectrum of ONE scan of a centroid HR file: exact peaks from the mzML
      case "mrm": info = engine.mrm(m.slot, m.blob.size, readerFor(m.blob)); break;                       // SRM transitions from the chromatograms of the mzML
      case "mrmchrom": info = engine.mrm_chrom(m.slot, m.blob.size, readerFor(m.blob), m.kind); break;   // api/chrom of an MRM file: a chromatogram of the mzML
      case "fileinfo": info = engine.fileinfo(m.slot, m.name, m.blob.size, readerFor(m.blob)); break;      // api/fileinfo: the head of the mzML + the index
      case "origin": info = engine.origin(m.items, m.k, m.query); break;                // api/origin over the session: "" = Python's (a file without index)
      case "subxic": info = engine.subxic(m.slot, m.k, m.query); break;                   // api/subxic of a high-resolution survey: "" = Python's
      case "map": info = await mapOp(m); break;                                          // api/map, mapsnap, mappunti, mappunti3d, maptiles over the session: "" = Python's             // api/map, mapsnap, mappunti, mappunti3d over the session: "" = Python's
      case "diff": info = engine.diff_traces(m.path, m.query || ""); break;               // api/chrom, api/xic with ref: "" = Python's, "E" + JSON = the user's error
      case "qaqc": info = engine.qaqc_ion(m.query || ""); break;                          // api/qaqc over the files at the slots of the query: "" = Python's, "E" + JSON = the user's error
      case "mrmdiff": info = mrm_diff_post(m.files, m.ref, m.query || ""); break;        // api/mrm with ref, from the transitions of the files
      case "idxxic": info = engine.index_xic(m.slot, m.level, m.mz, m.tol); break;
      case "budget": engine.set_budget_mb(m.mb); break;                                  // the supervisor lowers it after a death by lack of memory
      case "close": engine.close(m.slot); blobs.delete(m.slot); tileName.delete(m.slot); break;   case "reset": engine.close_all(); blobs.clear(); tileName.clear(); break;
      case "crash": if (m.grow) wasmMem.grow(m.grow * 16); engine.crash(); break;        // test hook: the supervisor must recover (grow = MB of memory taken first, the signature of a lack of memory)
      case "tic": out = take(2 * engine.count(m.slot, m.level)); n = engine.tic(m.slot, m.level, m.px || 0, out); break;
      case "xic": out = take(2 * engine.count(m.slot, m.level)); n = engine.xic(m.slot, m.level, m.mz, m.tol, m.px || 0, out); break;
      case "spectra": {
        const len = engine.spectra_len(m.slot, m.level, m.i0, m.i1);
        if (!len) { n = 0; break; }
        out = take(len); n = engine.spectra(m.slot, m.level, m.i0, m.i1, m.bin, out); break;
      }
      default: throw new Error("unknown operation " + m.op);
    }
    postMessage({ id, op: m.op, n, info, buf: out ? out.buffer : null, len: out ? out.length : 0, mem: memNow(), rd: rdStat }, out ? [out.buffer] : []);
  } catch (e) {
    const text = typeof e === "string" ? e : String(e && e.message || e);
    // a JSON string {"error_key", "params"} is an error of the engine itself (the file is not readable, memory budget); anything else
    const fatal = !(typeof e === "string" && e.startsWith("{"));
    postMessage({ id, op: m.op, error: text, fatal, mem: wasmMem ? wasmMem.buffer.byteLength : 0, panic: fatal ? (globalThis.__mzlabPanic || null) : null });      // the panic message (set by the engine before the abort) and the memory it had: the supervisor tells a lack of memory from another fault
  }
}
