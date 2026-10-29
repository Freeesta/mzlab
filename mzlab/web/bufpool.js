// @ts-check
// Pool of ArrayBuffers handed back by the page (op "recycle"), reused by the next answer of rust-worker.js: best fit (the smallest buffer that is big enough,
// the oldest of equals), at most `maxCount` buffers and `maxBytes` bytes kept (the oldest go first; a buffer bigger than the byte cap is not kept).
/** @param {number} [maxCount] @param {number} [maxBytes] */
export function createBufPool(maxCount = 8, maxBytes = 64 << 20) {
  /** @type {ArrayBuffer[]} */ const bufs = [];
  let bytes = 0;
  return {
    /** A Float64Array of `len` doubles over a pooled buffer when one fits, else over a new one (at least 64 KiB, so small answers share a size). @param {number} len */
    take(len) {
      const need = len * 8;
      let best = -1;
      for (let i = 0; i < bufs.length; i++) if (bufs[i].byteLength >= need && (best < 0 || bufs[i].byteLength < bufs[best].byteLength)) best = i;
      if (best < 0) return new Float64Array(new ArrayBuffer(Math.max(need, 64 << 10)), 0, len);
      const b = /** @type {ArrayBuffer} */ (bufs.splice(best, 1)[0]); bytes -= b.byteLength;
      return new Float64Array(b, 0, len);
    },
    /** @param {ArrayBuffer[]} list */
    give(list) {
      for (const b of list) if (b.byteLength && b.byteLength <= maxBytes) { bufs.push(b); bytes += b.byteLength; }
      while (bufs.length > maxCount || bytes > maxBytes) bytes -= /** @type {ArrayBuffer} */ (bufs.shift()).byteLength;
    },
    get size() { return bufs.length; },
    get bytes() { return bytes; }
  };
}
