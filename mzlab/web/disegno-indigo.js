// @ts-check
// One queue for every Ketcher conversion (Indigo): its structure service never answers when two calls overlap. Used by draw.js (SMILES of a piece,
// getSmiles, partial export, image export).
let indigoQ = /** @type {Promise<any>} */ (Promise.resolve());
/**
 * Runs `fn` after the calls queued before it. A slot is released after 20 s at most, so a call that never answers cannot block the ones behind it.
 * Never call `indigo()` from inside another `indigo()` callback: it would wait for itself.
 * @template T @param {() => Promise<T>} fn @returns {Promise<T>}
 */
export function indigo(fn) {
  const run = indigoQ.then(fn), slot = Promise.race([run.then(() => {}, () => {}), new Promise(ok => setTimeout(ok, 20000))]);
  indigoQ = slot; return run;
}
