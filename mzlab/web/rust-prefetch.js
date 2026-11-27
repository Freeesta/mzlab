// @ts-check
// @ts-strict
// Prefetch for the «Choose samples» dialog (wiff-choose.js): a sample ticked there has its byte ranges copied to OPFS while the dialog is open, to the store name its opening computes
// (`storeOf` with the plan of that one sample), so `putHeld` (rust-nativo.js) finds the copy there, or waits for it through `writing`. Unticking a sample or closing the dialog stops the copy
// and removes what it wrote. The sample, the plan and the index stay the engine's and the opening's: this file only moves bytes earlier.
const TICK = 200;                                         // ms between two progress messages

/**
 * @param {{ ARCH: any, jerr: (k: string) => never, held: (key: string, fn: () => Promise<any>) => Promise<any>, copy: (d: any, blob: Blob, store: string, bump: (n: number) => void, ranges?: number[][]) => Promise<void>,
 *   deps: any, storeOf: (nat: any, x: any) => string, partOf: (nat: any, x: any) => number[][] | undefined, workOf: (nat: any, x: any) => number, missing: (nat: any) => Promise<number>,
 *   ensureRoom: (bytes: number, key: string) => Promise<void>, release: (key: string) => void, stubOf: (name: string) => Promise<any>, now?: () => number }} d
 */
export function createPrefetch({ ARCH, jerr, held, copy, deps, storeOf, partOf, workOf, missing, ensureRoom, release, stubOf, now = () => performance.now() }) {
  const holds = /** @type {Map<string, string>} */ (new Map());         // store -> dataset key: kept from the sweep until a stub points to it
  const fresh = /** @type {Map<string, string>} */ (new Map());         // the stores a prefetch wrote (a copy that was already there is never removed)
  const owned = /** @type {Map<string, string[]>} */ (new Map());       // `key#id` -> the ranged stores of that one sample (unticking it removes them)
  const queued = /** @type {Set<string>} */ (new Set());                  // gate names of the requests waiting for a free copy slot (X drops them all)
  const cut = /** @type {Set<string>} */ (new Set()), running = /** @type {Map<string, Promise<void>>} */ (new Map());

  /** @param {{ key: string, sid: number, members: { name: string, blob: Blob }[], ranges: Record<string, number[][]>, tag: string }} m @param {(f: number) => void} tell */
  async function prefetch(m, tell) {
    const nat = { key: m.key, members: m.members, ranges: m.ranges, tag: m.tag }, tag = m.key + "#" + m.sid, room = "pf:" + tag, mine = /** @type {string[]} */ ([]);
    cut.delete(tag);
    const work = nat.members.reduce((a, x) => a + workOf(nat, x), 0);
    let done = 0, last = 0;
    // only the ranged copy of this sample stops when it is unticked: the container is shared by the samples being copied
    const bump = (/** @type {boolean} */ own) => (/** @type {number} */ n) => { if (own && cut.has(tag)) jerr("err.load.cancelled"); done += n; const t = now(); if (t - last > TICK || done >= work) { last = t; tell(work ? Math.min(1, done / work) : 1); } };
    const job = held(m.key, async () => {
      try {
        await ensureRoom(await missing(nat), room);
        for (const x of nat.members) {
          const store = storeOf(nat, x), had = await ARCH.get("files", store), part = !!partOf(nat, x), isNew = !(had && had.size === x.blob.size) && !deps.writing.has(store);
          await copy(deps, x.blob, store, bump(part), partOf(nat, x));
          holds.set(store, m.key);
          if (isNew) { fresh.set(store, m.key); if (part) mine.push(store); }
        }
        owned.set(tag, mine);
      } catch (e) {
        for (const st of mine) { holds.delete(st); fresh.delete(st); await ARCH.remove("files", st).catch(() => {}); }      // a stopped or refused copy leaves nothing of its own
        throw e;
      } finally { release(room); }
    });
    running.set(tag, job.then(() => {}, () => {}));
    try { await job; } finally { running.delete(tag); }
  }

  /** stops and discards the prefetch of samples `sids` of dataset `key` (all of its samples when null); `end`: the dataset is done with it, the holds go and a copy no stub uses is removed */
  async function unprefetch(/** @type {{ key: string, sids?: number[] | null, end?: boolean }} */ m) {
    const mine = (/** @type {string} */ t) => t.startsWith(m.key + "#");
    const tags = m.sids ? m.sids.map(s => m.key + "#" + s) : [...new Set([...running.keys(), ...owned.keys()])].filter(mine);
    for (const t of tags) cut.add(t);
    await Promise.all(tags.map(t => running.get(t)));
    for (const t of tags) { for (const st of owned.get(t) || []) { holds.delete(st); if (fresh.delete(st)) await ARCH.remove("files", st).catch(() => {}); } owned.delete(t); cut.delete(t); }      // a store a stub took over (`landed`) is no longer fresh: it stays
    if (!m.end) return;
    const used = new Set();
    for (const e of await ARCH.list("files")) { if (!/\.mzml$/i.test(e.name)) continue; const st = await stubOf(e.name); if (st) st.members.forEach((/** @type {{ store: string }} */ y) => used.add(y.store)); }
    for (const [st, k] of [...holds]) if (k === m.key) { holds.delete(st); if (fresh.has(st) && !used.has(st)) { fresh.delete(st); await ARCH.remove("files", st).catch(() => {}); } }
  }

  return {
    holds,
    /** the stub of a sample is written (or its opening failed): its stores are no longer the prefetch's, the sweep may free an unused one */
    landed: (/** @type {{ store: string }[]} */ members) => { for (const x of members) { holds.delete(x.store); fresh.delete(x.store); } },
    /**
     * the worker's request: `natprefetch` copies one sample behind the copy gate (`copies.through`, at most 2 at once with the other copies), `natunprefetch` stops and discards
     * @param {any} m @param {{ through: Function, drop: Function }} copies @param {() => string} cancelled @param {(o: any) => void} post
     */
    op: async (m, copies, cancelled, post) => {
      if (m.op === "natunprefetch") {
        const mine = m.sids ? m.sids.map((/** @type {number} */ s) => "pf:" + m.key + "#" + s) : [...queued].filter(n => n.startsWith("pf:" + m.key + "#"));
        for (const n of mine) copies.drop(n, cancelled());
        await unprefetch(m); return { info: "{}" };
      }
      const name = "pf:" + m.key + "#" + m.sid; queued.add(name);
      try {
        return await copies.through(name, 2, -1, () => post({ op: "progress", id: m.id, frac: 0, phase: "read", wait: true }),
          async () => { queued.delete(name); await prefetch(m, frac => post({ op: "progress", id: m.id, frac, phase: "read" })); return { info: "{}" }; });
      } finally { queued.delete(name); }
    },
    prefetch, unprefetch,
  };
}
