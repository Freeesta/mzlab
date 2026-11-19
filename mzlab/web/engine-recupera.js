// @ts-strict
// Recovery when the engine cannot start (rust-bridge.js lost its worker twice: typically a service-worker cache that holds wasm/JS of two
// deploys): drop the site's app caches and the worker, reload ONCE. A session flag stops a loop; the caller shows the error if it was used.
const FLAG = "mzlab.engineRecover";

/** @returns {Promise<"reloading" | "already">} */
export async function recoverEngine({ log = console.warn, reload = () => location.reload(), prefix = "mzlab-app-", storage = sessionStorage, cacheApi = globalThis.caches, sw = navigator.serviceWorker } = {}) {
  try { if (storage.getItem(FLAG)) return "already"; storage.setItem(FLAG, "1"); } catch (_) { return "already"; }     // no storage: no way to bound the retries
  const dropped = [];
  try { for (const k of cacheApi ? await cacheApi.keys() : []) if (k.startsWith(prefix) && await cacheApi.delete(k)) dropped.push(k); } catch (_) { /* reload anyway */ }
  let unregistered = 0;
  try { for (const r of sw ? await sw.getRegistrations() : []) if (await r.unregister()) unregistered++; } catch (_) { /* reload anyway */ }
  log("[mzLab] engine did not start: dropped caches [" + dropped.join(", ") + "], unregistered " + unregistered + " service worker(s), reloading once");
  reload();
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
