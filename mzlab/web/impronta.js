// @ts-strict
// The fingerprints of a member (SHA-256, SHA-1, probe: `Passata` of the engine, crates/mzlab-wasm/src/natives.rs) taken WHILE it is copied to OPFS, so that the file is read once. The default is a dedicated
// worker (impronta-worker.js) that receives each piece as a Transferable after it was written and hashes it on another core while the copy goes on; without a Worker (or `mode: "qui"`) it hashes in the
// worker of the copy at each piece. With four cores or more two hash workers share a member (SHA-256 and probe, SHA-1). At most WINDOW pieces wait for a hash worker (the copy waits for it beyond that: the memory never holds the file).
// The hash worker is started at once, without its .wasm: its script is fetched long before the first file. Until that script has arrived once (`up`) a member
// is hashed in place: on a slow first visit it queued behind the page's own downloads and held the copy for 10-19 s. `free`: only the SHA-1 keeps pace with the copy (the reader of a .raw
// needs it to open the file); SHA-256 and probe read the file given at their own rate beside it (two workers) or the copy once it ends (one): the index is built meanwhile.
// A file changed during the upload fails to read (the browser refuses a changed File): no fingerprints, the builder takes them itself.
const WINDOW = 4;
// the hash worker closes after this long without a member to hash (the memory of its .wasm never shrinks); the next member starts one again
const IDLE_MS = 10000;

/** @typedef {{ add: (b: Uint8Array) => Promise<void> | void, end: () => Promise<string[] | null> }} Presa */
/** @typedef {{ add: (b: Uint8Array) => Promise<void> | void, end: (file: Blob) => { sha1: Promise<string | null>, all: Promise<string[] | null> }, drop: () => void }} Libera */

// with cores to spare a member is hashed by two workers: SHA-256 and the probe in the first, SHA-1 in the second (a copy of each piece), about 1.4x the rate of one
const PAIR = () => ((typeof navigator !== "undefined" && navigator.hardwareConcurrency) || 2) >= 4;

/** as `start`, the copy waiting only for the SHA-1 (pieces of the copy, in step, in worker `b`); SHA-256 and probe read `source` (the file given) in worker `a` beside the copy,
 *  or the copy once it ends when `a` is `b`. `end(file)` gives the SHA-1 and the promise of [sha256, sha1, probe] (null if one fails)
 *  @param {{ a: Worker, b: Worker, job: (x: Worker, size: number, how: object) => { id: number, j: any }, ended: (x: Worker, o: { id: number, j: any }, file?: Blob) => Promise<string[] | null>, gone: () => boolean }} h */
function freeOf({ a, b, job, ended, gone }, /** @type {number} */ size, /** @type {Blob} */ source) {
  const s = job(b, size, { onlySha1: true }), k = b === a ? 0 : 1, beside = b !== a ? ended(a, job(a, size, { noSha1: true }), source) : null;
  return /** @type {Libera} */ ({
    add(p) {
      if (gone()) return;
      s.j.open[k]++; b.postMessage({ op: "upd", id: s.id, b: p }, [p.buffer]);
      if (s.j.open[k] >= WINDOW) return new Promise(r => { s.j.ack = r; });
    },
    drop() { if (!gone()) ended(b, s); },
    end(file) {
      if (gone()) return { sha1: Promise.resolve(null), all: Promise.resolve(null) };
      const sha1 = ended(b, s).then(o => (o && o[1]) || null), rest = beside || ended(a, job(a, file.size, { noSha1: true }), file);
      return { sha1, all: Promise.all([rest, sha1]).then(([o, x]) => o && x ? [o[0], x, o[2]] : null) };
    },
  });
}

/** @param {{ Passata: new (size: number, noSha1?: boolean, onlySha1?: boolean) => { aggiorna: (b: Uint8Array) => void, fine: () => string[] | undefined }, workerUrl: URL, module: () => WebAssembly.Module | null, mode?: string }} h */
export function createImpronte({ Passata, workerUrl, module, mode }) {
  /** @type {(Worker | null)[]} */ const w = [null, null];
  const up = [false, false], armed = [false, false], slots = PAIR() ? [0, 1] : [0];
  let dead = mode === "qui", seq = 0;
  /** @type {ReturnType<typeof setTimeout> | undefined} */ let idle;
  const rest = () => { clearTimeout(idle); idle = setTimeout(() => { if (!jobs.size) for (const k of slots) { w[k]?.terminate(); w[k] = null; } }, IDLE_MS); };
  /** @type {Map<number, { ack: (() => void) | null, out: ((v: string[] | null) => void) | null, open: number[], got: (string[] | null)[], left: number, solo?: boolean }>} */
  const jobs = new Map();
  /** a job of one worker: its id and record (`new` sent) */
  const job = (/** @type {Worker} */ x, /** @type {number} */ size, /** @type {object} */ how) => {
    const id = ++seq, j = { ack: /** @type {(() => void) | null} */ (null), out: /** @type {((v: string[] | null) => void) | null} */ (null), open: [0, 0], got: [null, null], left: 1, solo: true };
    jobs.set(id, j); x.postMessage({ op: "new", id, size, ...how }); return { id, j };
  };
  /** the answer of a job (`end`; with `file` the worker reads the member from it) */
  const ended = (/** @type {Worker} */ x, /** @type {{ id: number, j: any }} */ o, /** @type {Blob | undefined} */ file = undefined) =>
    /** @type {Promise<string[] | null>} */ (new Promise(r => { o.j.out = r; x.postMessage(file ? { op: "end", id: o.id, file } : { op: "end", id: o.id }); }));
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
        else if (m.op === "fine") { j.got[k] = m.out || null; if (--j.left === 0) { jobs.delete(m.id); j.out?.(j.solo ? m.out || null : join(j.got)); if (!jobs.size) rest(); } }
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
    /** `freeOf` with the workers of this page; null without a hash worker */
    free(/** @type {number} */ size, /** @type {Blob} */ source) {
      clearTimeout(idle);
      const a = ready(0), b = a && slots.length > 1 ? ready(1) : a;
      return a && b ? freeOf({ a, b, job, ended, gone: () => dead }, size, source) : null;
    },
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
