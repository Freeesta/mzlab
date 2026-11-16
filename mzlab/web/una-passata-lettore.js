// @ts-check
// SPIKE, the builder's side of una-passata.js: inside build_mzidx_streamed (synchronous) the bytes come from the ring of SharedArrayBuffer slots the
// copy fills (`next`, in order, waiting with Atomics.wait); a piece the copy could not put in a full ring, and what is left once the copy is done, is read
// from the dropped File (FileReaderSync, one read per gap). After the last byte of the file the ring carries the scan table the engine's worker made at the
// end of the copy (`table`); `again` (rounds that decode again, fallback) reads the dropped File, since the copy in OPFS cannot be opened inside a synchronous call.

// the ring: SLOTS pieces of at most SLOT bytes, at increasing offsets with gaps; ctrl = [events, pushed, taken, failed (no scan table), stopped, copy done]
export const SLOTS = 4, SLOT = 8 << 20, C = { SEQ: 0, PUSH: 1, TAKE: 2, FAIL: 3, STOP: 4, DONE: 5 }, HEAD = 4 * 9;

const stopped = () => JSON.stringify({ error_key: "err.load.cancelled", params: {} });

/** @param {{ ctrl: Int32Array, meta: Float64Array, data: Uint8Array }} ring @param {Blob} blob @param {(frac: number) => void} tick */
export function createLettore(ring, blob, tick) {
  const { ctrl, meta, data } = ring, size = blob.size;
  // `seq` is read before the condition it waits on: a push between the two wakes the wait at once
  const wait = (/** @type {number} */ seq) => {
    if (Atomics.load(ctrl, C.STOP)) throw stopped();
    if (Atomics.load(ctrl, C.FAIL)) throw new Error("no scan table");
    Atomics.wait(ctrl, C.SEQ, seq, 1000);
  };
  let declared = 0, fromFile = 0;
  const fr = new FileReaderSync(), read = (/** @type {number} */ off, /** @type {number} */ len) => new Uint8Array(fr.readAsArrayBuffer(blob.slice(off, off + len)));
  /** the bytes [off, off + len) of the stream (the file, then the table), in order */
  const next = (/** @type {number} */ off, /** @type {number} */ len) => {
    const out = new Uint8Array(len);
    let got = 0;
    if (off < size) tick(off / size);
    // bytes of the file that will not come through the ring: from the dropped File
    const file = (/** @type {number} */ pos, /** @type {number} */ m) => { out.set(read(pos, m), got); got += m; fromFile += m; };
    while (got < len) {
      const seq = Atomics.load(ctrl, C.SEQ), pos = off + got, take = Atomics.load(ctrl, C.TAKE);
      if (take >= Atomics.load(ctrl, C.PUSH)) {
        if (pos < size && Atomics.load(ctrl, C.DONE)) file(pos, Math.min(len - got, size - pos)); else wait(seq);
        continue;
      }
      const k = take % SLOTS, at = meta[2 * k], n = meta[2 * k + 1];
      if (pos >= at + n) { Atomics.add(ctrl, C.TAKE, 1); Atomics.add(ctrl, C.SEQ, 1); Atomics.notify(ctrl, C.SEQ); continue; }      // slot read: the copy may fill it again
      if (pos < at) {      // a gap: pieces the copy wrote while the ring was full
        if (pos >= size) throw new Error("ring out of order");
        file(pos, Math.min(len - got, at - pos, size - pos)); continue;
      }
      const m = Math.min(len - got, at + n - pos);
      out.set(data.subarray(k * SLOT + pos - at, k * SLOT + pos - at + m), got); got += m;
    }
    return out;
  };
  return {
    next,
    /** the eight inputs of build_mzidx (una-passata.js tableBytes: nine u32, then the parts), from the bytes after the file */
    table() {
      const h = new Uint32Array(next(size, HEAD).buffer), n = h[0] + h[1] + h[2] + h[3] + h[4] + h[5] + h[6], u = next(size + HEAD, n), dec = new TextDecoder();
      let at = 0;
      const part = (/** @type {number} */ i) => { const b = u.slice(at, at + h[i]); at += h[i]; return b; };
      const p = [part(0), part(1), part(2), part(3), part(4), dec.decode(part(5)), dec.decode(part(6)), h[7] === 1];
      declared = h[8];
      return p;
    },
    /** the same file read again (later rounds without a scratch file, the fallback) */
    again: read,
    declared: () => declared,
    /** bytes of the stream read from the File instead of the ring */
    fromFile: () => fromFile,
  };
}

/** idx-worker.js `build` for op "stream": the index of `m` from the ring, with the worker's scratch file, writer and progress; the helpers are closed in any case
 * @param {any} m @param {{ spill: any, write: Function, tick: (stage: string, frac: number, total: number) => void, aiuti: () => Promise<any>, build: Function }} h */
export async function buildStreamed(m, { spill, write, tick, aiuti, build }) {
  const r = createLettore(m.ring, m.blob, f => tick("read", f, 0));
  let put = 0, got = 0, a = null;
  const spillPut = spill && ((/** @type {number} */ off, /** @type {Uint8Array} */ d) => { put += d.length; return spill.put(off, d); });
  const spillGet = spill && ((/** @type {number} */ off, /** @type {number} */ n) => { got += n; tick("index", put ? Math.min(1, got / put) : 0, 0); return spill.get(off, n); });
  try {
    a = await aiuti();
    const stats = build(m.blob.size, r.next, r.again, r.table, write, m.budget, spillPut || undefined, spillGet || undefined, m.cap || 0, a || undefined);
    return { stats: JSON.stringify({ ...JSON.parse(stats), from_file: r.fromFile() }), declared: r.declared() };
  } finally { if (a) a.chiudi(); }
}
