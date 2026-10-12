// Short-lived worker of the .wiff conversion pool (rust-worker.js, idxpool.js): converts ONE sample of a .wiff + .wiff.scan to mzML with the Rust engine (mzlab-wasm,
// wiff_to_mzml), reading both files by offset (Blob.slice, synchronously), and sends the mzML pieces back moved, not copied. Then the pool closes it: its WASM memory goes back
// to the system. Several of these run at once, one per sample.
import init, { Engine } from "./wasm/mzlab_wasm.js";
import { convert } from "./rust-wiff.js";

// The pool sends one message first, { op: "module", module }: the .wasm compiled once for all the workers, or null (this worker compiles it itself).
let ready = null, startInit = null;
const initWith = module => (ready = (module ? init({ module_or_path: module }) : init()).then(() => postMessage({ op: "ready" })).catch(e => postMessage({ op: "unavailable", text: String(e && e.message || e) })));
const waiting = new Promise(r => { startInit = r; });
const readerFor = blob => { const fr = new FileReaderSync(); return (offset, length) => new Uint8Array(fr.readAsArrayBuffer(blob.slice(offset, offset + length))); };

onmessage = async ev => {
  const m = ev.data;
  if (m.op === "module") { if (!ready) initWith(m.module); startInit(); return; }
  await waiting; await ready;
  try {
    const parts = convert(new Engine(), m, readerFor, p => postMessage({ op: "progress", ...p }));
    postMessage({ op: "done", parts }, parts.map(a => a.buffer));
  } catch (e) {
    const text = typeof e === "string" ? e : String(e && e.message || e);
    // a JSON string {"error_key", "params"} is an error of the reader itself; anything else (RuntimeError: unreachable, out of memory) lost the instance
    postMessage({ op: "error", text, fatal: !(typeof e === "string" && e.startsWith("{")) });
  }
};
