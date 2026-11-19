// @ts-strict
// Recovery when the engine cannot start (rust-bridge.js lost its worker twice: a stored copy of the site is a suspect): the reset flow drops it, ONCE.
// A session flag stops a loop; the caller shows the error if it was used.
const FLAG = "mzlab.engineRecover";

/** One recovery per session: the reset flow's "engine" mode (reset.js) drops the app caches and the worker, keeps the files, and reloads with a `ripn` nonce
 * (so the isolating reload that follows an unregister still happens once). @returns {Promise<"reloading" | "already">} */
export async function recoverEngine({ log = console.warn, run = (/** @type {string} */ m) => /** @type {any} */ (window).QQRESET.run(m), storage = sessionStorage } = {}) {
  try { if (storage.getItem(FLAG)) return "already"; storage.setItem(FLAG, "1"); } catch (_) { return "already"; }     // no storage: no way to bound the retries
  log("[mzLab] engine did not start: dropping the stored copy of the site and reloading once");
  try { await run("engine"); } catch (_) { return "already"; }
  return "reloading";
}

/** the page that follows the recovery clears the flag once its engine is up */
export function recoverDone(storage = sessionStorage) { try { storage.removeItem(FLAG); } catch (_) { /* none */ } }

/** A worker that dies while a call is pending resolves it as "engine lost" although the supervisor (rust-bridge.js died) already started a new one:
 * the call is made once more on that one. A second loss, or an engine switched off, is the caller's to handle. */
export async function retryLost(/** @type {{ disabled?: boolean }} */ rb, /** @type {() => Promise<any>} */ call) {
  try { return await call(); }
  catch (e_) { const e = /** @type {any} */ (e_); if (!e || !e.lost || rb.disabled) throw e; console.warn("[mzLab] the engine worker was lost during start: asking its replacement"); return call(); }
}
