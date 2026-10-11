// Short-lived worker of the index pool: builds the .mzidx of ONE mzML with the Rust builder (mzlab-wasm, build_mzidx), reading the source file
// in slices (Blob.slice, synchronously) and writing the index to a file of OPFS under a temporary name. Then the pool closes it: the WASM memory
// goes back to the system. Messages only (no SharedArrayBuffer). If it dies, idxpool.js starts another one.
import init, { build_mzidx } from "./wasm/mzlab_wasm.js";

const ready = init().then(() => postMessage({ op: "ready" })).catch(e => postMessage({ op: "unavailable", text: String(e && e.message || e) }));

onmessage = async ev => {
  const m = ev.data;
  await ready;
  let ah = null;
  try {
    const d = await (await (await navigator.storage.getDirectory()).getDirectoryHandle("mzlab", { create: true })).getDirectoryHandle("idx", { create: true });
    const fh = await d.getFileHandle(m.part, { create: true });
    ah = await fh.createSyncAccessHandle();
    ah.truncate(0);
    const fr = new FileReaderSync(), blob = m.blob;
    const read = (off, len) => new Uint8Array(fr.readAsArrayBuffer(blob.slice(off, off + len)));
    const write = (off, data) => { ah.write(data, { at: off }); };
    const [levels, pols, rt, tic, rows, source, run, hr1] = m.inputs;
    const t0 = performance.now();
    const stats = build_mzidx(blob.size, read, write, levels, pols, rt, tic, rows, source, run, hr1, m.budget);
    ah.flush(); ah.close(); ah = null;
    postMessage({ op: "done", stats: JSON.parse(stats), ms: Math.round(performance.now() - t0) });
  } catch (e) {
    if (ah) { try { ah.close(); } catch (_) { /* already closed */ } }
    const text = typeof e === "string" ? e : String(e && e.message || e);
    // a JSON string {"error_key", "params"} is an error of the builder itself; anything else (RuntimeError: unreachable, out of memory) lost the instance
    postMessage({ op: "error", text, fatal: !(typeof e === "string" && e.startsWith("{")) });
  }
};
