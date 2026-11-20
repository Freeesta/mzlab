// @ts-check
// Drawing, failed editor load: tells at once that the frame will never become Ketcher, so that draw.js need not wait its 60 s.
// Signs: a page that is not Ketcher's (404, offline), a script of it answered with an error, a part it could not import.

/** Looks at the frame now and listens to its window; `fail` may be called more than once.
 * @param {HTMLIFrameElement} fr @param {() => void} fail */
export function watchLoad(fr, fail) {
  try {
    const w = /** @type {any} */ (fr.contentWindow), d = /** @type {Document} */ (fr.contentDocument);
    if (w.location.href === "about:blank" || w.ketcher) return;
    if (d.readyState === "complete" && !d.querySelector("script[src]")) { fail(); return; }
    if (w.performance.getEntriesByType("resource").some((/** @type {any} */ r) => r.initiatorType === "script" && r.responseStatus >= 400)) { fail(); return; }
    w.addEventListener("error", (/** @type {Event} */ e) => { if (/** @type {Element} */ (e.target).tagName === "SCRIPT") fail(); }, true);
    w.addEventListener("unhandledrejection", (/** @type {PromiseRejectionEvent} */ e) => { if (/module|chunk/i.test(String(e.reason && e.reason.message))) fail(); });
  } catch (_) { fail(); }                                  // a frame that cannot be read is not the editor either
}
