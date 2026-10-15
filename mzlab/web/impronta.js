// @ts-check
// The fingerprints of a member (SHA-256, SHA-1, probe: `Passata` of the engine, crates/mzlab-wasm/src/nativi.rs) taken WHILE it is copied to OPFS, so that the file is read once. The default is a dedicated
// worker (impronta-worker.js) that receives each piece as a Transferable after it was written and hashes it on another core while the copy goes on; without a Worker (or `mode: "qui"`) it hashes in the
// worker of the copy at each piece. At most WINDOW pieces wait for the hash worker (the copy waits for it beyond that: the memory never holds the file).
const WINDOW = 4;

/** @typedef {{ add: (b: Uint8Array) => Promise<void> | void, end: () => Promise<string[] | null> }} Presa */

/** @param {{ Passata: new (size: number) => { aggiorna: (b: Uint8Array) => void, fine: () => string[] | undefined, free: () => void }, workerUrl: URL, module: () => WebAssembly.Module | null, mode?: string }} h */
export function createImpronte({ Passata, workerUrl, module, mode }) {
  /** @type {Worker | null} */ let w = null;
  let dead = mode === "qui", seq = 0;
  /** @type {Map<number, { ack: (() => void) | null, out: ((v: string[] | null) => void) | null, open: number }>} */
  const jobs = new Map();
  const fail = () => { dead = true; w = null; for (const j of jobs.values()) { j.open = 0; j.ack?.(); j.out?.(null); } jobs.clear(); };
  function worker() {
    if (w || dead) return w;
    try {
      const x = new Worker(workerUrl, { type: "module" });
      x.onmessage = ev => {
        const m = ev.data, j = jobs.get(m.id);
        if (!j) return;
        if (m.op === "ack") { j.open--; if (j.open < WINDOW) { const a = j.ack; j.ack = null; a?.(); } } else if (m.op === "fine") { jobs.delete(m.id); j.out?.(m.out || null); }
      };
      x.onerror = fail; x.onmessageerror = fail;
      x.postMessage({ op: "module", module: module() });
      w = x;
    } catch (_) { dead = true; w = null; }
    return w;
  }
  return {
    /** the fingerprints of a member of `size` bytes: `add(piece)` in file order (the piece goes to the worker: do not use it afterwards), then `end()` gives [sha256, sha1, probe] or null */
    start(/** @type {number} */ size) {
      const wk = worker();
      if (wk) {
        const id = ++seq, j = { ack: /** @type {(() => void) | null} */ (null), out: /** @type {((v: string[] | null) => void) | null} */ (null), open: 0 };
        jobs.set(id, j);
        wk.postMessage({ op: "new", id, size });
        return /** @type {Presa} */ ({
          add(b) {
            if (dead) return;
            j.open++;
            wk.postMessage({ op: "upd", id, b }, [b.buffer]);
            if (j.open >= WINDOW) return new Promise(r => { j.ack = r; });
          },
          end() { return dead ? Promise.resolve(null) : new Promise(r => { j.out = r; wk.postMessage({ op: "end", id }); }); },
        });
      }
      let p = new Passata(size);
      return /** @type {Presa} */ ({
        add(b) { p.aggiorna(b); },
        async end() { const o = p.fine(); p.free(); return o ? Array.from(o) : null; },
      });
    },
  };
}
