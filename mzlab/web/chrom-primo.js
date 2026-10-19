// @ts-check
// Order of the first requests to the engine after «Carica dati»: it answers one at a time, so the first chromatogram is asked for before what only completes the graphs
// (the series list of an instrument log waits for `ready`), and a provisional TIC is replaced by ONE real request however many redraws ask meanwhile. Used by getChrom (explore.js) and registro-serie.js.
const CHROM1 = (() => {
  /** @type {() => void} */ let open = () => {};
  const ready = new Promise(r => { open = () => r(undefined); });
  // a session that never asks for a chromatogram must not hold the others back
  setTimeout(open, 3000);
  /** @type {Map<string, Promise<any>>} */ const asked = new Map();
  return {
    ready,
    /** the request of a chromatogram has just been sent: the others may follow. @template T @param {Promise<T>} r @returns {Promise<T>} */
    sent(r) { open(); return r; },
    /** `make()` sends the real request that replaces a provisional TIC, once per key until it ends */
    once(/** @type {string} */ key, /** @type {() => Promise<any>} */ make) { if (!asked.has(key)) asked.set(key, make().finally(() => asked.delete(key))); open(); },
    clear() { asked.clear(); },
  };
})();
