// @ts-check
// Loading helpers of the engine's worker (rust-worker.js), apart so that node can test them: the copy of a plain upload to OPFS (cancellable, its room released on any failure),
// the streaming gunzip of a .mzML.gz into OPFS (`putGz`) and the gate that lets lrOpen skip re-parsing an index that did not change.

// a .mzML.gz is read in pieces of GZ_STEP and inflated GZ_SLICE at a time (repetitive XML inflates up to ~1000 times: one push makes at most ~64 MB); the mzML is never whole
// in memory. GZ_CAP: the 4 GB of a plain upload (lr.js)
export const GZ_STEP = 1 << 20, GZ_SLICE = 64 << 10, GZ_CAP = 4 * 1024 * 1024 * 1024;
/** the room a .mzML.gz asks before it is inflated: the size the gzip trailer states (ISIZE, the size modulo 4 GiB of the last member), or 5 times the file when it cannot be right
 * @param {Blob} gz */
export async function gzRoom(gz) {
  if (gz.size < 18) return gz.size;
  const t = new DataView(await gz.slice(gz.size - 4).arrayBuffer()).getUint32(0, true);
  return t >= gz.size ? t : gz.size * 5;
}

/** what the inflate of a .gz left when its tab closed: `files/*.gz.part` nobody writes, and the mzXML of a .mzXML.gz in conv/ whose page is gone (gz-flusso.js names it
 * gz~<page>~…, and the page holds the lock mzlab-gzses:<page> while it lives)
 * @param {any} ARCH */
export async function gzClean(ARCH) {
  for (const e of await ARCH.list("files")) if (/\.gz\.part$/.test(e.name)) await ARCH.removeIfFree("files", e.name);
  for (const e of await ARCH.list("conv")) {
    const sid = (/^gz~([a-z0-9]+)~/.exec(e.name) || [])[1];
    if (sid) await navigator.locks.request("mzlab-gzses:" + sid, { ifAvailable: true }, async l => { if (l) await ARCH.remove("conv", e.name); });
  }
}

/** the copy of a plain uploaded file: `copyPrints` (SHA-256 and probe taken in the same pass), `putPlain` (one retry after `free`, room released when it fails); `putGz` and
 * `gunzipTo` inflate a gzip upload with `gunzip(cap)` (the engine's `Gunzip`)
 * @param {{ ARCH: any, impronte: { start: (size: number, noSha1?: boolean) => any }, prints: Map<string, any>, cancelled: () => string, isCancelled: (name: string) => boolean, post: (m: any) => void, free: () => Promise<unknown>, now?: () => number, gunzip?: (cap: number) => any }} h */
export function createPlainPut({ ARCH, impronte, prints, cancelled, isCancelled, post, free, now = () => performance.now(), gunzip }) {
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
  // the gzip File `m.blob` inflated piece by piece through the engine's `Gunzip` (mzlab_core::gunzip, the inflate of the app) into kind/name of OPFS; removed on cancel or error
  async function inflate(/** @type {{ blob: Blob, name: string, id: any }} */ m, /** @type {string} */ kind, /** @type {string} */ name) {
    if (!gunzip) throw JSON.stringify({ error_key: "err.file.gzStream", params: {} });
    const gz = m.blob, g = gunzip(GZ_CAP);
    let o = 0, tp = 0, end = false, piece = new Uint8Array(0), at = 0;
    try {
      return await ARCH.putStream(kind, name, async () => {
        if (end) return null;
        if (at < piece.length) { const s = piece.subarray(at, at + GZ_SLICE); at += s.length; return g.push(s); }
        if (isCancelled(m.name)) throw cancelled();      // «Cancel» of the page: the inflate stops at the next piece
        if (o >= gz.size) { end = true; const last = g.finish(); if (!g.whole() && !g.written()) throw JSON.stringify({ error_key: "err.file.zlib", params: { detail: "truncated" } }); return last; }
        const n = Math.min(GZ_STEP, gz.size - o);
        piece = new Uint8Array(await gz.slice(o, o + n).arrayBuffer()); at = 0; o += n;
        const t = now();      // the bar follows the bytes of the .gz read
        if (t - tp > 100) { tp = t; post({ op: "progress", id: m.id, frac: Math.min(1, o / gz.size), phase: "load", done: o, total: gz.size }); }
        return new Uint8Array(0);
      });
    } catch (e) { await ARCH.remove(kind, name); throw e; }
    finally { if (g.free) g.free(); }
  }
  // a .mzML.gz (`m.name` is the mzML it becomes): into files/<name>.gz.part, then renamed to the name of the mzML, so the plain path indexes and reads it from OPFS. The part file goes
  // on cancel, on error and, left by a closed tab, at the next start (lr_boot); the mzML goes with its sample (lr_remove), as a plain upload's copy.
  async function putGz(/** @type {{ blob: Blob, name: string, id: any }} */ m) {
    prints.delete(m.name);
    try { await inflate(m, "files", m.name + ".gz.part"); return await ARCH.promote("files", m.name + ".gz.part", m.name); }
    catch (e) { ARCH.release(m.name); throw e; }
  }
  // a .mzXML.gz: the inflated mzXML is a temporary file of conv/ (`dest`) that the readers module turns into an mzML; the page removes it then (gz-flusso.js), lr_boot the ones a closed tab left
  async function gunzipTo(/** @type {{ blob: Blob, name: string, id: any, dest: string }} */ m) {
    try { return await inflate(m, "conv", m.dest); } finally { ARCH.release(m.name); }
  }
  return { copyPrints, putPlain, putGz, gunzipTo };
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
