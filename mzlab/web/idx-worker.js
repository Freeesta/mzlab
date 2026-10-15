// Short-lived worker of the index pool: builds the .mzidx of ONE mzML with the Rust builder (mzlab-wasm, build_mzidx), reading the source file
// in slices (Blob.slice, synchronously) and writing the index to a file of OPFS under a temporary name. Then the pool closes it: the WASM memory
// goes back to the system. Messages only (no SharedArrayBuffer). If it dies, idxpool.js starts another one.
import init, { build_mzidx, build_mzidx_native, native_external } from "./wasm/mzlab_wasm.js";
import { writeFull, quotaError } from "./archivio.js";
import { buildNative } from "./rust-nativo.js";
import { createEsterno } from "./lettore-esterno.js";

const ext = createEsterno(native_external);                         // a Thermo .raw is read through the `thermo` module, loaded here on the first one (lettore-esterno.js)

// The pool sends one message first, { op: "module", module }: the .wasm compiled once for all the workers, or null (this worker compiles it itself).
let ready = null, startInit = null;
const initWith = module => (ready = (module ? init({ module_or_path: module }) : init()).then(() => postMessage({ op: "ready" })).catch(e => postMessage({ op: "unavailable", text: String(e && e.message || e) })));
const waiting = new Promise(r => { startInit = r; });

onmessage = async ev => {
  const m = ev.data;
  if (m.op === "module") { if (!ready) initWith(m.module); startInit(); return; }
  await waiting; await ready;
  const dropSpool = async () => { if (sh) { try { sh.close(); } catch (_) { /* already closed */ } sh = null; } if (d) { try { await d.removeEntry(m.part + ".spool"); } catch (_) { /* none */ } } };
  let ah = null, sh = null, d = null, fullAt = 0, endAt = 0;
  try {
    d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab", { create: true })).getDirectoryHandle("idx", { create: true });
    const fh = await d.getFileHandle(m.part, { create: true });
    ah = await fh.createSyncAccessHandle();
    ah.truncate(0);
    const write = (off, data) => { if (writeFull(ah, data, off) < data.length) { fullAt = off + data.length; throw new Error("storage full"); } endAt = Math.max(endAt, off + data.length); };       // a full disk: not a silently short index
    // scratch file of the builder: the peaks of pass 1 that do not fit in the budget wait here, so that the file is decoded once (without it the rounds decode it again)
    let spool = [undefined, undefined];
    try {
      sh = await (await d.getFileHandle(m.part + ".spool", { create: true })).createSyncAccessHandle();
      sh.truncate(0);
      const swrite = (off, data) => { if (writeFull(sh, data, off) < data.length) { fullAt = off + data.length; throw new Error("storage full"); } };
      const sread = (off, len) => { const b = new Uint8Array(len); if (sh.read(b, { at: off }) < len) throw new Error("spool short"); return b; };
      spool = [swrite, sread];
    } catch (_) { if (sh) { try { sh.close(); } catch (__) { /* gone */ } sh = null; } }
    const t0 = performance.now();
    let stats;
    if (m.nat) {                                                    // a dataset of a maker (rust-nativo.js): the index comes straight from its reader; the progress says the phase
      stats = await buildNative(build_mzidx_native, ext, m.nat, m.name, write, p => postMessage({ op: "progress", frac: p.frac, phase: p.phase }), m.budget, spool);
    } else {
      const fr = new FileReaderSync(), blob = m.blob;
      let tp = 0;
      const read = (off, len) => {                                  // the builder reads the file front to back: the offset is the progress (the page shows it, at most 5 times a second)
        const now = performance.now();
        if (now - tp > 200) { tp = now; postMessage({ op: "progress", frac: blob.size ? Math.min(1, off / blob.size) : 0 }); }
        return new Uint8Array(fr.readAsArrayBuffer(blob.slice(off, off + len)));
      };
      const [levels, pols, rt, tic, rows, source, run, hr1] = m.inputs;
      stats = build_mzidx(blob.size, read, write, levels, pols, rt, tic, rows, source, run, hr1, m.budget, spool[0], spool[1]);
    }
    ah.flush();
    if (ah.getSize() < endAt) { fullAt = endAt; throw new Error("storage full"); }           // the browser took the writes but the file is shorter (quota reached): never an index that looks complete
    ah.close(); ah = null;
    await dropSpool();
    postMessage({ op: "done", stats: JSON.parse(stats), ms: Math.round(performance.now() - t0) });
  } catch (e) {
    if (ah) { try { ah.close(); } catch (_) { /* already closed */ } }
    await dropSpool();
    let text = typeof e === "string" ? e : String(e && e.message || e);
    if (fullAt) text = await quotaError(fullAt).catch(() => text);
    // a JSON string {"error_key", "params"} is an error of the builder itself; anything else (RuntimeError: unreachable, out of memory) lost the instance
    postMessage({ op: "error", text, fatal: !fullAt && !(typeof e === "string" && e.startsWith("{")) });
  }
};
