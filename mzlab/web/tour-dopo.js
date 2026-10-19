// @ts-check
"use strict";
// Small helpers of the tour invitation (tour.js): centre it under the side panel and, after «Not now», a bubble on the gear that says where the tour can be taken again.
// The bubble goes away by itself after 10 s, on a click or on Esc. Classic script.
const TOURDOPO = (() => {
  /** Makes the invitation as wide as the side panel, with equal margins at both sides; follows the panel while it opens or is resized. @param {HTMLElement} div */
  function centra(div) {
    const side = document.getElementById("dfiles");
    if (!side) return;
    const apply = () => {
      if (!div.isConnected) { stop(); return; }
      const sr = side.getBoundingClientRect();
      if (sr.width <= 180 || sr.left >= window.innerWidth / 2) return;
      div.style.left = (sr.left + 12) + "px";
      div.style.width = (sr.width - 24) + "px";
    };
    const stop = FRAME.ro(side, apply);
    apply();
  }
  /** Shows the bubble under the gear of the header (once; the invitation itself appears once per browser). */
  function mostra() {
    const gear = document.getElementById("np-set");
    if (!gear || document.getElementById("tour-later")) return;
    const b = document.createElement("div");
    b.id = "tour-later";
    b.setAttribute("role", "status");
    b.setAttribute("aria-live", "polite");
    b.textContent = I18N.t("tour.invito.dopo");
    document.body.appendChild(b);
    const g = gear.getBoundingClientRect();
    const w = b.offsetWidth;
    const left = Math.max(8, Math.min(window.innerWidth - w - 8, g.right - w));
    b.style.left = left + "px";
    b.style.top = (g.bottom + 10) + "px";
    b.style.setProperty("--tip", Math.max(8, Math.min(w - 20, left + w - (g.left + g.width / 2) - 5)) + "px");
    FRAME.next(() => b.classList.add("on"));
    let gone = false;
    const off = () => {
      if (gone) return;
      gone = true;
      clearTimeout(timer);
      document.removeEventListener("pointerdown", off, true);
      document.removeEventListener("keydown", onKey, true);
      b.classList.remove("on");
      const reduced = window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
      setTimeout(() => b.remove(), reduced ? 0 : 320);
    };
    /** @param {KeyboardEvent} e */
    const onKey = e => { if (e.key === "Escape") off(); };
    const timer = setTimeout(off, 10000);
    // the click that closed the invitation is already over: listen from the next turn
    setTimeout(() => { document.addEventListener("pointerdown", off, true); document.addEventListener("keydown", onKey, true); }, 0);
  }
  return { centra, mostra };
})();
