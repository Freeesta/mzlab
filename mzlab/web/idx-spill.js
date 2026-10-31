// @ts-strict
// @ts-check
// Scratch-file accessors of one index build (idx-worker.js): put/get over a sync access handle. get reuses one read buffer, grown on demand:
// the builder copies the bytes out before its next call. A private window (ephemeral storage) can report a full write and then lose the bytes past its real
// limit (about 1 GiB of the origin, whatever `estimate().quota` says): put reads back the last byte of every slot, so that loss ends as the storage-full error.
export function makeSpill(/** @type {any} */ sh, /** @type {(off: number, n: number) => void} */ onFull, /** @type {(h: any, data: Uint8Array, off: number) => number} */ writeFull) {
  let buf = new Uint8Array(0);
  return {
    put: (/** @type {number} */ off, /** @type {Uint8Array} */ data) => {
      if (writeFull(sh, data, off) < data.length) { onFull(off, data.length); throw new Error("storage full"); }
      const last = data.length - 1, back = new Uint8Array(1);
      if (last >= 0 && (sh.read(back, { at: off + last }) < 1 || back[0] !== data[last])) { onFull(off, data.length); throw new Error("storage full"); }
    },
    get: (/** @type {number} */ off, /** @type {number} */ n) => { if (buf.length < n) buf = new Uint8Array(n); const b = buf.subarray(0, n); if (sh.read(b, { at: off }) < n) throw new Error("spill read"); return b; },
  };
}
