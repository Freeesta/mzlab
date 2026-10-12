// Web Worker of the Rust engine (?motore=rust): mzlab-wasm opens mzML files in blocks and answers TIC, XIC and single scans as
// Float64Arrays that are moved (not copied) to the page. The page gives the arrays back ("recycle") once drawn, so a frame
// allocates nothing. If this worker dies, rust-bridge.js starts a new one (the WASM instance cannot be reused after a trap).
import init, { Engine, prepare_upload, formula_mz, composition } from "./wasm/mzlab_wasm.js";
import * as ARCH from "./archivio.js";
import { lab } from "./qqq-db.js";
import { createPool, poolSize } from "./idxpool.js";

let engine = null;
const pool = [];                       // ArrayBuffers handed back by the page, ready to be written again
const blobs = new Map();               // slot -> Blob (read synchronously, block by block, by the WASM reader)

const take = len => {
  const need = len * 8;
  const i = pool.findIndex(b => b.byteLength >= need);
  return new Float64Array(i >= 0 ? pool.splice(i, 1)[0] : new ArrayBuffer(Math.max(need, 64 << 10)), 0, len);
};
const ready = init().then(() => { engine = new Engine(); }).then(() => postMessage({ op: "ready" }));
ready.then(() => fetch(new URL("./idx-worker.js", import.meta.url)).catch(() => {}));      // the index builder script goes into the service worker cache: an upload works offline too

function readerFor(blob) {
  const fr = new FileReaderSync();
  return (offset, length) => new Uint8Array(fr.readAsArrayBuffer(blob.slice(offset, offset + length)));
}


// ---- the flow without Pyodide (?lr=1, lr.js): the mzML files and their indexes live in OPFS (archivio.js), the list of the files, the items of the
// session and the state of the page come from here. The slots of these files are numbered above LR_BASE, apart from the ones of the session (k).
const LR_BASE = 1000000;
const lrSlot = new Map(), lrFile = new Map();
let lrN = 0, lrPool = null;
const slotOf = n => { if (!lrSlot.has(n)) lrSlot.set(n, LR_BASE + ++lrN); return lrSlot.get(n); };
const SMALL = (() => { try { return /iPad|iPhone|Android/i.test(navigator.userAgent) || (navigator.platform === "MacIntel" && navigator.maxTouchPoints > 1); } catch (_) { return false; } })();
const idxPool = () => lrPool || (lrPool = createPool({ needSimd: true, workerUrl: new URL("./idx-worker.js", import.meta.url), size: poolSize(SMALL, navigator.hardwareConcurrency), budget: SMALL ? 32 : 64, notify: key => postMessage({ op: "notice", key }) }));
const jerr = key => { throw JSON.stringify({ error_key: key, params: {} }); };
async function fileOf(name) {
  if (!lrFile.has(name)) { const f = await ARCH.get("files", name); if (!f) jerr("err.session.fileNotFound"); lrFile.set(name, f); }
  return lrFile.get(name);
}
async function lrOpen(name) {                                             // opens the index of a file in its slot; false when there is none
  const f = await fileOf(name), ix = await ARCH.get("idx", name + ".mzidx");
  if (!ix) return false;
  engine.open_index(slotOf(name), ix.size, readerFor(ix));
  return true;
}
// IndexedDB of the page ("qqq_lab", the same one as browser-worker.js): the notebook and, without OPFS, the files
const idb = lab();
const tx = async (store, mode, fn) => {
  const db = await idb();
  return new Promise((res, rej) => { const t = db.transaction(store, mode), s = t.objectStore(store), out = fn(s); t.oncomplete = () => res(out && out.result !== undefined ? out.result : undefined); t.onerror = t.onabort = () => rej(t.error); });
};
const safe = async f => { try { return await f(); } catch (_) { return undefined; } };
async function lrSpace() {
  const files = await safe(() => ARCH.list("files")) || [], idx = await safe(() => ARCH.list("idx")) || [];
  const names = new Set(files.map(f => f.name)), gone = idx.filter(e => !names.has(e.name.replace(/\.mzidx$/, "")));
  const sum = l => l.reduce((a, e) => a + e.size, 0), est = await ARCH.estimate();
  let persisted = false; try { persisted = !!(await navigator.storage.persisted()); } catch (_) { /* optional */ }
  return { opfs: true, files: sum(files), idx: sum(idx), idx_removed: sum(gone), n_removed: gone.length, usage: est.usage, quota: est.quota, persisted };
}
async function lrFree() {                                                 // the indexes of the files that were taken away go first
  const files = new Set((await safe(() => ARCH.list("files")) || []).map(f => f.name));
  for (const e of await safe(() => ARCH.list("idx")) || []) if (!files.has(e.name.replace(/\.mzidx$/, ""))) await ARCH.remove("idx", e.name);
}
async function lrOp(m) {
  switch (m.op) {
    case "lr_boot": {                                                     // every file in OPFS, with its size, whether it has an index, and the File itself for the bridge
      const out = [];
      for (const e of await ARCH.list("files")) {
        if (/\.part$/.test(e.name)) continue;
        const f = await ARCH.get("files", e.name); if (!f) continue;
        lrFile.set(e.name, f);
        const ok = /\.mzml$/i.test(e.name) ? await safe(() => lrOpen(e.name)) : false;
        out.push({ name: e.name, size: f.size, indexed: !!ok, file: f });
      }
      return { info: JSON.stringify(out.map(({ file, ...r }) => r)), files: out.map(o => [o.name, o.file]) };
    }
    case "lr_put": {                                                      // an uploaded file: into OPFS, replacing the one of the same name and its index
      idxPool().cancel(m.name);
      engine.close(slotOf(m.name));
      await ARCH.remove("idx", m.name + ".mzidx");
      let f;
      try { f = await ARCH.put("files", m.name, m.blob); }
      catch (e) { if (/quota/i.test(String(e && (e.name || e.message)))) { await lrFree(); f = await ARCH.put("files", m.name, m.blob); } else throw e; }
      lrFile.set(m.name, f);
      try { if (navigator.storage && navigator.storage.persist && !(await navigator.storage.persisted())) navigator.storage.persist(); } catch (_) { /* optional */ }
      return { info: JSON.stringify({ size: f.size }), files: [[m.name, f]] };
    }
    case "lr_index": {                                                    // the scan table (Rust), the index (a short-lived worker of the pool), then the index is opened
      const f = await fileOf(m.name), t0 = performance.now();
      const a = prepare_upload(f.size, readerFor(f), m.name), part = m.name + ".mzidx.part";
      const res = await idxPool().build({ name: m.name, blob: f, inputs: Array.from(a).slice(0, 8), part });
      if (!res.ok) { await ARCH.remove("idx", part); jerr(res.reason === "memory" ? "error.memoryBudget" : "err.engine.index"); }
      await ARCH.promote("idx", part, m.name + ".mzidx");
      if (!(await lrOpen(m.name))) jerr("err.engine.index");
      return { info: JSON.stringify({ ms: Math.round(performance.now() - t0) }) };
    }
    case "lr_entry": { const f = await fileOf(m.name); if (!(await lrOpen(m.name))) jerr("err.engine.index"); return { info: engine.lr_entry(slotOf(m.name), m.name, f.size, readerFor(f)) }; }
    case "lr_items": { const f = await fileOf(m.name); if (!(await lrOpen(m.name))) jerr("err.engine.index"); return { info: engine.lr_items(slotOf(m.name), m.name, f.size, readerFor(f), JSON.stringify(m.sample || {})) }; }
    case "lr_composition": return { info: composition(m.query || "") };                  // "" = Python's (it words the error)
    case "lr_formula": return { info: formula_mz(m.f || "", m.adduct || "") };
    case "lr_remove": { engine.close(slotOf(m.name)); lrFile.delete(m.name); await ARCH.remove("files", m.name); return { info: "{}" }; }
    case "lr_space": { if (m.free) await lrFree(); return { info: JSON.stringify(await lrSpace()) }; }
    case "lr_kv_get": { const v = await safe(() => tx("kv", "readonly", s => s.get(m.key))); return { info: JSON.stringify({ value: v === undefined ? null : v }) }; }
    case "lr_kv_set": { await safe(() => tx("kv", "readwrite", s => s.put(m.value, m.key))); return { info: "{}" }; }
    case "lr_clear": {                                                    // «New session» with fresh = true: files, indexes, notebook
      for (const n of [...lrFile.keys()]) engine.close(slotOf(n));
      lrFile.clear();
      await safe(async () => { await ARCH.clear("files"); await ARCH.clear("idx"); });
      await safe(() => tx("files", "readwrite", s => s.clear())); await safe(() => tx("kv", "readwrite", s => s.clear()));
      return { info: "{}" };
    }
  }
  return null;
}

onmessage = async ev => {
  const m = ev.data;
  if (m.op === "recycle") { for (const b of m.bufs) if (b.byteLength) pool.push(b); if (pool.length > 8) pool.splice(0, pool.length - 8); return; }
  await ready;
  const { id } = m;
  try {
    let out = null, n = 0, info = null;
    if (m.op.startsWith("lr_")) {
      const r = await lrOp(m);
      postMessage({ id, op: m.op, n: 0, info: r.info, buf: null, len: 0, files: r.files || null });
      return;
    }
    switch (m.op) {
      case "open": blobs.set(m.slot, m.blob); info = engine.open(m.slot, m.blob.size, readerFor(m.blob)); break;
      case "raw2mzml": {                                                              // Thermo .raw -> mzML pieces (the .raw is read block by block, never held)
        const parts = [], total = m.blob.size, rd = readerFor(m.blob);
        let read = 0, written = 0, last = 0;
        const tick = () => { const t = performance.now(); if (t - last > 150) { last = t; postMessage({ op: "progress", id, total, read, written }); } };     // the page shows how far the conversion has got
        engine.raw_to_mzml(m.name, total, (o, l) => { const b = rd(o, l); read = Math.max(read, o + b.length); tick(); return b; }, piece => { parts.push(piece); written += piece.length; tick(); });
        postMessage({ id, op: m.op, n: parts.length, parts }, parts.map(a => a.buffer));
        return;
      }
      case "idxopen": {                                                               // the .mzidx of a file, read from OPFS where the index builder put it (no mzML is read)
        let blob;
        try { blob = await (await (await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab")).getDirectoryHandle("idx")).getFileHandle(m.name + ".mzidx").then(h => h.getFile()); }
        catch (_) { throw '{"error_key":"error.noIndex","params":{}}'; }                  // no OPFS or no index yet: not a fault of the engine, the page uses the mzML
        info = engine.open_index(m.slot, blob.size, readerFor(blob)); break;
      }
      case "route": info = engine.route(m.slot, m.path, m.query || ""); break;       // "" = not answered by the index
      case "scaninfo": info = engine.scaninfo(m.slot, m.sid, m.blob.size, readerFor(m.blob)); break;     // header of one scan: the index row + a slice of the mzML
      case "mrm": info = engine.mrm(m.slot, m.blob.size, readerFor(m.blob)); break;                       // SRM transitions from the chromatograms of the mzML
      case "origin": info = engine.origin(m.items, m.k, m.query); break;                // api/origin over the session: "" = Python's (profile or high resolution)
      case "subxic": info = engine.subxic(m.slot, m.k, m.query); break;                   // api/subxic of a high-resolution survey: "" = Python's
      case "map": info = engine.map_route(m.path, m.items, m.query); break;             // api/map, mapsnap, mappunti, mappunti3d over the session: "" = Python's
      case "idxxic": info = engine.index_xic(m.slot, m.level, m.mz, m.tol); break;
      case "close": engine.close(m.slot); blobs.delete(m.slot); break;
      case "reset": engine.close_all(); blobs.clear(); break;
      case "crash": engine.crash(); break;                                           // test hook: the supervisor must recover
      case "tic": out = take(2 * engine.count(m.slot, m.level)); n = engine.tic(m.slot, m.level, m.px || 0, out); break;
      case "xic": out = take(2 * engine.count(m.slot, m.level)); n = engine.xic(m.slot, m.level, m.mz, m.tol, m.px || 0, out); break;
      case "spectra": {
        const len = engine.spectra_len(m.slot, m.level, m.i0, m.i1);
        if (!len) { n = 0; break; }
        out = take(len); n = engine.spectra(m.slot, m.level, m.i0, m.i1, m.bin, out); break;
      }
      default: throw new Error("unknown operation " + m.op);
    }
    postMessage({ id, op: m.op, n, info, buf: out ? out.buffer : null, len: out ? out.length : 0 }, out ? [out.buffer] : []);
  } catch (e) {
    const text = typeof e === "string" ? e : String(e && e.message || e);
    // a JSON string {"error_key", "params"} is an error of the engine itself (the file is not readable, memory budget); anything else
    // (RuntimeError: unreachable, out of memory) means the instance is lost
    const fatal = !(typeof e === "string" && e.startsWith("{"));
    postMessage({ id, op: m.op, error: text, fatal });
  }
};
