// @ts-check
// The helper workers of one index build (idx-worker.js): they decode the arrays of the spectra (base64 + zlib) while the builder reads the file.
// The builder's courier (crates/mzlab-wasm/src/corriere.rs) calls send(seq, job, bound) and take(seq) synchronously; each job has a slot, a
// SharedArrayBuffer [state, job bytes, answer bytes, -, data...] shared with the helper, which answers in place and wakes the builder with
// Atomics.notify. Only with cross-origin isolation (capacita.js «indiceParallelo»); without it the builder decodes alone, with the same bytes.

const HEAD = 16;
// a helper that does not answer in this time is given up: the builder decodes everything from then on
const WAIT_MS = 20000;
// a helper not ready in this time is not used
const READY_MS = 15000;

/**
 * Starts `n` helpers (n > 0) with the engine's compiled module (or null: each compiles it) and resolves once they are ready.
 * @param {number} n
 * @param {WebAssembly.Module | null} module
 */
export async function creaAiuti(n, module) {
  const url = new URL("./idx-aiuto.js", import.meta.url);
  /** @type {Promise<Worker | null>[]} */ const starting = [];
  for (let i = 0; i < n; i++) {
    try {
      const w = new Worker(url, { type: "module" });
      starting.push(new Promise(res => {
        const t = setTimeout(() => { w.terminate(); res(null); }, READY_MS);
        w.onmessage = e => { clearTimeout(t); if (e.data && e.data.ok) res(w); else { w.terminate(); res(null); } };
        w.onerror = () => { clearTimeout(t); w.terminate(); res(null); };
      }));
      w.postMessage({ op: "module", module });
    } catch (_) { break; }
  }
  const workers = /** @type {Worker[]} */ ((await Promise.all(starting)).filter(Boolean));
  // four jobs out per helper keep them busy while the builder waits for the oldest
  const window = 4 * workers.length;
  /** @type {Array<{ sab: SharedArrayBuffer, i32: Int32Array, u8: Uint8Array } | undefined>} */ const slots = [];
  let dead = workers.length === 0;
  return {
    window,
    /** @type {(seq: number, job: Uint8Array, bound: number) => boolean} */
    send(seq, job, bound) {
      if (dead) return false;
      const i = seq % window, need = HEAD + Math.max(job.length, bound);
      let s = slots[i];
      if (!s || s.sab.byteLength < need) {
        const sab = new SharedArrayBuffer(Math.ceil(need * 1.25 / 4096) * 4096);
        s = slots[i] = { sab, i32: new Int32Array(sab, 0, 4), u8: new Uint8Array(sab) };
      }
      s.u8.set(job, HEAD);
      Atomics.store(s.i32, 1, job.length);
      Atomics.store(s.i32, 0, 0);
      workers[seq % workers.length].postMessage({ op: "job", sab: s.sab });
      return true;
    },
    /** @type {(seq: number) => Uint8Array | undefined} */
    take(seq) {
      const s = slots[seq % window];
      if (!s) return undefined;
      while (Atomics.load(s.i32, 0) === 0) {
        if (dead || Atomics.wait(s.i32, 0, 0, WAIT_MS) === "timed-out") { dead = true; return undefined; }
      }
      return Atomics.load(s.i32, 0) === 1 ? new Uint8Array(s.sab, HEAD, Atomics.load(s.i32, 2)) : undefined;
    },
    chiudi() { dead = true; for (const w of workers) w.terminate(); },
  };
}
