// Web Worker of the Rust engine (?motore=rust): mzlab-wasm opens mzML files in blocks and answers TIC, XIC and single scans as
// Float64Arrays that are moved (not copied) to the page. The page gives the arrays back ("recycle") once drawn, so a frame
// allocates nothing. If this worker dies, rust-bridge.js starts a new one (the WASM instance cannot be reused after a trap).
import init, { Engine } from "./wasm/mzlab_wasm.js";

let engine = null;
const pool = [];                       // ArrayBuffers handed back by the page, ready to be written again
const blobs = new Map();               // slot -> Blob (read synchronously, block by block, by the WASM reader)

const take = len => {
  const need = len * 8;
  const i = pool.findIndex(b => b.byteLength >= need);
  return new Float64Array(i >= 0 ? pool.splice(i, 1)[0] : new ArrayBuffer(Math.max(need, 64 << 10)), 0, len);
};
const ready = init().then(() => { engine = new Engine(); }).then(() => postMessage({ op: "ready" }));

function readerFor(blob) {
  const fr = new FileReaderSync();
  return (offset, length) => new Uint8Array(fr.readAsArrayBuffer(blob.slice(offset, offset + length)));
}

onmessage = async ev => {
  const m = ev.data;
  if (m.op === "recycle") { for (const b of m.bufs) if (b.byteLength) pool.push(b); if (pool.length > 8) pool.splice(0, pool.length - 8); return; }
  await ready;
  const { id } = m;
  try {
    let out = null, n = 0, info = null;
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
