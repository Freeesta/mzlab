// @ts-strict
// @ts-check
// Scratch-file accessors of one index build (idx-worker.js): put/get over a sync access handle. get reuses one read buffer, grown on demand:
// the builder copies the bytes out before its next call.
export function makeSpill(/** @type {any} */ sh, /** @type {(off: number, n: number) => void} */ onFull, /** @type {(h: any, data: Uint8Array, off: number) => number} */ writeFull) {
  let buf = new Uint8Array(0);
  return {
    put: (/** @type {number} */ off, /** @type {Uint8Array} */ data) => { if (writeFull(sh, data, off) < data.length) { onFull(off, data.length); throw new Error("storage full"); } },
    get: (/** @type {number} */ off, /** @type {number} */ n) => { if (buf.length < n) buf = new Uint8Array(n); const b = buf.subarray(0, n); if (sh.read(b, { at: off }) < n) throw new Error("spill read"); return b; },
  };
}
