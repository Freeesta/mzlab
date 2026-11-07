// @ts-strict
// The fingerprints of a member (SHA-256, SHA-1, probe: `Passata` of the engine, crates/mzlab-wasm/src/nativi.rs) taken WHILE it is copied to OPFS, so that the file is read once. The default is a dedicated
// worker (impronta-worker.js) that receives each piece as a Transferable after it was written and hashes it on another core while the copy goes on; without a Worker (or `mode: "qui"`) it hashes in the
// worker of the copy at each piece. With four cores or more two hash workers share a member (SHA-256 and probe, SHA-1). At most WINDOW pieces wait for a hash worker (the copy waits for it beyond that: the memory never holds the file).
// The hash worker is started at once, without its .wasm: its script is fetched long before the first file. Until that script has arrived once (`up`) a member
// is hashed in place: on a slow first visit it queued behind the page's own downloads and held the copy for 10-19 s.
const WINDOW = 4;
// the hash worker closes after this long without a member to hash (the memory of its .wasm never shrinks); the next member starts one again
const IDLE_MS = 10000;

/** @typedef {{ add: (b: Uint8Array) => Promise<void> | void, end: () => Promise<string[] | null> }} Presa */

// with cores to spare a member is hashed by two workers: SHA-256 and the probe in the first, SHA-1 in the second (a copy of each piece), about 1.4x the rate of one
const PAIR = () => ((typeof navigator !== "undefined" && navigator.hardwareConcurrency) || 2) >= 4;

/** @param {{ Passata: new (size: number, noSha1?: boolean, onlySha1?: boolean) => { aggiorna: (b: Uint8Array) => void, fine: () => string[] | undefined }, workerUrl: URL, module: () => WebAssembly.Module | null, mode?: string }} h */
export function createImpronte({ Passata, workerUrl, module, mode }) {
  /** @type {(Worker | null)[]} */ const w = [null, null];
  const up = [false, false], armed = [false, false], slots = PAIR() ? [0, 1] : [0];
  let dead = mode === "qui", seq = 0;
  /** @type {ReturnType<typeof setTimeout> | undefined} */ let idle;
  const rest = () => { clearTimeout(idle); idle = setTimeout(() => { if (!jobs.size) for (const k of slots) { w[k]?.terminate(); w[k] = null; } }, IDLE_MS); };
  /** @type {Map<number, { ack: (() => void) | null, out: ((v: string[] | null) => void) | null, open: number[], got: (string[] | null)[], left: number }>} */
  const jobs = new Map();
  const fail = () => { dead = true; for (const k of slots) { w[k]?.terminate(); w[k] = null; } for (const j of jobs.values()) { j.open = [0, 0]; j.ack?.(); j.out?.(null); } jobs.clear(); };
  // the fingerprints of a member from what its workers gave: [sha256, sha1, probe], the SHA-1 from the second when there are two
  const join = (/** @type {(string[] | null)[]} */ g) => g.length < 2 ? g[0] || null : g[0] && g[1] ? [g[0][0], g[1][1], g[0][2]] : null;
  function spawn(/** @type {number} */ k) {
    if (w[k] || dead) return w[k];
    try {
      const x = new Worker(workerUrl, { type: "module" });
      x.onmessage = ev => {
        const m = ev.data, j = jobs.get(m.id);
        if (m.op === "up") { up[k] = true; return; }
        if (!j) return;
        if (m.op === "ack") { j.open[k]--; if (j.open.every(n => n < WINDOW)) { const a = j.ack; j.ack = null; a?.(); } }
        else if (m.op === "fine") { j.got[k] = m.out || null; if (--j.left === 0) { jobs.delete(m.id); j.out?.(join(j.got)); if (!jobs.size) rest(); } }
      };
      x.onerror = fail; x.onmessageerror = fail;
      w[k] = x; armed[k] = false;
    } catch (_) { dead = true; w[k] = null; }
    return w[k];
  }
  // the .wasm goes in at the first member only (its memory never shrinks)
  function worker(/** @type {number} */ k) {
    const x = spawn(k);
    if (x && !armed[k]) { x.postMessage({ op: "module", module: module() }); armed[k] = true; }
    return x;
  }
  /** the worker `k` if its script has arrived, else null (it is started for the next member) */
  const ready = (/** @type {number} */ k) => up[k] ? worker(k) : (spawn(k), null);
  slots.forEach(spawn);
  return {
    /** the fingerprints of a member of `size` bytes: `add(piece)` in file order (the piece goes to the worker: do not use it afterwards), then `end()` gives [sha256, sha1, probe] or null */
    start(/** @type {number} */ size, /** @type {boolean=} */ noSha1) {
      clearTimeout(idle);
      const first = ready(0), second = first && !noSha1 && slots.length > 1 ? ready(1) : null;
      if (first) {
        const wk = second ? [first, second] : [first];
        const id = ++seq, j = { ack: /** @type {(() => void) | null} */ (null), out: /** @type {((v: string[] | null) => void) | null} */ (null), open: [0, 0], got: wk.map(() => null), left: wk.length };
        jobs.set(id, j);
        first.postMessage({ op: "new", id, size, noSha1: noSha1 || !!second });
        second?.postMessage({ op: "new", id, size, onlySha1: true });
        return /** @type {Presa} */ ({
          add(b) {
            if (dead) return;
            if (second) { const c = b.slice(); j.open[1]++; second.postMessage({ op: "upd", id, b: c }, [c.buffer]); }
            j.open[0]++;
            first.postMessage({ op: "upd", id, b }, [b.buffer]);
            if (j.open.some(n => n >= WINDOW)) return new Promise(r => { j.ack = r; });
          },
          end() { return dead ? Promise.resolve(null) : new Promise(r => { j.out = r; for (const x of wk) x.postMessage({ op: "end", id }); }); },
        });
      }
      let p = new Passata(size, noSha1);
      return /** @type {Presa} */ ({
        add(b) { p.aggiorna(b); },
        async end() { const o = p.fine(); return o ? Array.from(o) : null; },
      });
    },
  };
}
