// @ts-strict
"use strict";
// The one place that talks to the browser's frame clock: FRAME.next(fn) = requestAnimationFrame, FRAME.cancel(id), FRAME.ro(el, fn) = one shared ResizeObserver
// (returns the function that stops watching); FRAME.calm() / FRAME.behavior() = the one reduced-motion check and the scroll behavior it picks. tools/controlla.py refuses new direct calls to requestAnimationFrame / ResizeObserver outside this file. Classic script, loaded in the head of index.html (before tour.js and avvio.js, which also use it).
const FRAME = (() => {
  const subs = new Map();                                            // element -> callbacks
  const ro = new ResizeObserver(es => { for (const e of es) for (const f of [...(subs.get(e.target) || [])]) f(e); });
  return {
    next: fn => requestAnimationFrame(fn),
    /** Reduced motion: the system setting or the in-app switch (data-a11y-reduce). */
    calm: () => matchMedia("(prefers-reduced-motion: reduce)").matches || document.documentElement.hasAttribute("data-a11y-reduce"),
    /** The `behavior` of scrollIntoView / scrollTo: instant under reduced motion. */
    behavior() { return this.calm() ? "auto" : "smooth"; },
    cancel: id => { if (id) cancelAnimationFrame(id); },
    ro(el, fn) {
      if (!el) return () => {};
      let s = subs.get(el); if (!s) { s = new Set(); subs.set(el, s); ro.observe(el); }
      s.add(fn);
      return () => { s.delete(fn); if (!s.size) { subs.delete(el); ro.unobserve(el); } };
    }
  };
})();
