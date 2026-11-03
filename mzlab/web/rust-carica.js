// @ts-check
// Loading helpers of the engine's worker (rust-worker.js), apart so that node can test them: the copy of a plain upload to OPFS (cancellable, its room released on any failure)
// and the gate that lets lrOpen skip re-parsing an index that did not change.

/** the copy of a plain uploaded file: `copyPrints` (SHA-256 and probe taken in the same pass), `putPlain` (one retry after `free`, room released when it fails)
 * @param {{ ARCH: any, impronte: { start: (size: number, noSha1?: boolean) => any }, prints: Map<string, any>, cancelled: () => string, isCancelled: (name: string) => boolean, post: (m: any) => void, free: () => Promise<unknown>, now?: () => number }} h */
export function createPlainPut({ ARCH, impronte, prints, cancelled, isCancelled, post, free, now = () => performance.now() }) {
  async function copyPrints(/** @type {{ blob: Blob, name: string, id: any }} */ m) {
    const blob = m.blob, pass = /\.mzml$/i.test(m.name) ? impronte.start(blob.size, true) : null;
    let tp = 0;      // the page shows the copy (bytes done / total, at most 10 times a second)
    const put = ARCH.putChunks("files", m.name, blob.size, async (/** @type {number} */ o, /** @type {number} */ n) => {
      if (isCancelled(m.name)) throw cancelled();      // «Cancel» of the page: the copy stops at the next piece
      const t = now();
      if (t - tp > 100) { tp = t; post({ op: "progress", id: m.id, frac: blob.size ? Math.min(1, o / blob.size) : 0, phase: "load", done: o, total: blob.size }); }
      return new Uint8Array(await blob.slice(o, o + n).arrayBuffer());
    }, pass ? (/** @type {Uint8Array} */ b) => pass.add(b) : undefined);
    let f;
    try { f = await put; } catch (e) { if (pass) pass.end().catch(() => null); throw e; }
    const o = pass && await pass.end().catch(() => null);
    if (o) prints.set(m.name, { size: blob.size, sha256: o[0], probe: o[2] });
    return f;
  }
  async function putPlain(/** @type {{ blob: Blob, name: string, id: any }} */ m) {
    prints.delete(m.name);
    try {
      try { return await copyPrints(m); }
      catch (e_) { const e = /** @type {any} */ (e_); if (!/quota/i.test(String(e && (e.name || e.message)))) throw e; await free(); return await copyPrints(m); }
    } catch (e) { ARCH.release(m.name); throw e; }      // the room promised to this file is not kept for a copy that failed or was cancelled
  }
  return { copyPrints, putPlain };
}

/** which slots hold the index they are asked to open: `same(slot, ix)` is true when the index (size and lastModified) is the one `mark`ed and the slot was not dropped since */
export function createOpenGate() {
  /** @type {Map<number, string>} */ const seen = new Map();
  const stamp = (/** @type {{ size: number, lastModified?: number }} */ ix) => ix.size + ":" + (ix.lastModified || 0);
  return {
    same: (/** @type {number} */ slot, /** @type {{ size: number, lastModified?: number }} */ ix) => seen.get(slot) === stamp(ix),
    mark: (/** @type {number} */ slot, /** @type {{ size: number, lastModified?: number }} */ ix) => { seen.set(slot, stamp(ix)); },
    drop: (/** @type {number} */ slot) => { seen.delete(slot); },
    clear: () => { seen.clear(); },
  };
}
