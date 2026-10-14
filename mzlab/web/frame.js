"use strict";
// The one place that talks to the browser's frame clock: FRAME.next(fn) = requestAnimationFrame, FRAME.cancel(id), FRAME.ro(el, fn) = one shared ResizeObserver
// (returns the function that stops watching). tools/controlla.py refuses new direct calls to requestAnimationFrame / ResizeObserver outside this file. Classic script, loaded in the head of index.html (before tour.js and avvio.js, which also use it).
const FRAME = (() => {
  const subs = new Map();                                            // element -> callbacks
  const ro = typeof ResizeObserver === "function" ? new ResizeObserver(es => { for (const e of es) for (const f of [...(subs.get(e.target) || [])]) f(e); }) : null;
  return {
    next: fn => requestAnimationFrame(fn),
    cancel: id => { if (id) cancelAnimationFrame(id); },
    ro(el, fn) {
      if (!ro || !el) return () => {};
      let s = subs.get(el); if (!s) { s = new Set(); subs.set(el, s); ro.observe(el); }
      s.add(fn);
      return () => { s.delete(fn); if (!s.size) { subs.delete(el); ro.unobserve(el); } };
    }
  };
})();
