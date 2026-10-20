// @ts-check
"use strict";
// Soft theme change: the new colours fade in (about 300 ms) instead of snapping, charts included. Uses the View Transitions API (a snapshot
// cross-fade, so the canvases fade too) and a CSS colour transition (transizioni.css, .tema-tr) where it is missing. Nothing moves with reduced motion.
const TEMASOFT = (() => {
  const R = document.documentElement;
  const FADE_MS = 400;
  const still = () => R.classList.contains("no-tr") || R.hasAttribute("data-a11y-reduce") || !!(window.matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches);
  /** Applies `change` (sets the theme and redraws) with a short fade. @param {() => void} change */
  function run(change) {
    if (still()) { change(); return; }
    if (typeof document.startViewTransition === "function") {
      const t = document.startViewTransition(change);
      t.finished.catch(() => { /* skipped: the change still ran */ });
      return;
    }
    R.classList.add("tema-tr");
    change();
    setTimeout(() => R.classList.remove("tema-tr"), FADE_MS);
  }
  return { run };
})();
