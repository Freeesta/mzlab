// @ts-strict
// @ts-noany
// Ketcher's frame gets a localStorage that never throws. Ketcher saves its settings with no try/catch, so a full or blocked storage
// (QuotaExceededError) would stop its start: blank canvas and an English error. The vendor page is not edited: this module wraps the
// frame's Storage methods from the parent as soon as the Ketcher document exists, before its deferred bundle runs. What could not be saved lives in memory.
const FRAME_PAGE = "ketcher/index.html";
const done = new WeakSet();

/** @param {any} w the frame window */
function guard(w) {
  const P = w.Storage && w.Storage.prototype, mem = new WeakMap();
  if (!P) return;
  const bag = (/** @type {object} */ s) => { let m = mem.get(s); if (!m) { m = Object.create(null); mem.set(s, m); } return m; };
  const get = P.getItem, set = P.setItem, del = P.removeItem;
  P.getItem = function (/** @type {string} */ k) { const m = bag(this); if (k in m) return m[k]; try { return get.call(this, k); } catch (_) { return null; } };
  P.setItem = function (/** @type {string} */ k, /** @type {unknown} */ v) { try { set.call(this, k, v); delete bag(this)[k]; } catch (_) { bag(this)[k] = String(v); } };
  P.removeItem = function (/** @type {string} */ k) { delete bag(this)[k]; try { del.call(this, k); } catch (_) { /* nothing stored */ } };
  try { void w.localStorage; } catch (_) {          // the property itself throws (storage blocked): an in-memory stand-in
    const m = bag(w), store = { getItem: (/** @type {string} */ k) => (k in m ? m[k] : null), setItem: (/** @type {string} */ k, /** @type {unknown} */ v) => { m[k] = String(v); }, removeItem: (/** @type {string} */ k) => { delete m[k]; }, clear: () => { for (const k of Object.keys(m)) delete m[k]; }, key: (/** @type {number} */ i) => Object.keys(m)[i] || null, get length() { return Object.keys(m).length; } };
    Object.defineProperty(w, "localStorage", { configurable: true, get: () => store });
  }
}

/** Polls (4 ms) for the Ketcher document of the frame and guards it once; stops when the editor exists or after 90 s. */
export function guardStorage() {
  const fr = /** @type {HTMLIFrameElement | null} */ (document.getElementById("kframe"));
  if (!fr) return;
  const t0 = Date.now(), iv = setInterval(() => {
    try {
      const w = /** @type {any} */ (fr.contentWindow);
      if (w && w.ketcher) { clearInterval(iv); return; }
      if (w && w.location.href.includes(FRAME_PAGE) && w.document && !done.has(w.document)) { done.add(w.document); guard(w); }
    } catch (_) { /* frame being replaced: next tick */ }
    if (Date.now() - t0 > 90000) clearInterval(iv);
  }, 4);
}
guardStorage();
