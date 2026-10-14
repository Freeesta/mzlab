/* mzLab - Teoria: five clicks on the logo unlock the extended Teoria (flag «qqq.teoria.esteso» in localStorage, read by teoria.js).
   Each click grows the logo (the image, not the link: the focus ring stays); the fifth one «bursts» it, a status message says what happened and the
   page reloads with the extended chapters. The logo stays a link: a click that is not followed by another within 600 ms goes where the link goes.
   No animation under reduced motion (the system setting or data-a11y-reduce). Classic script, no network: it works from file:// too. */
"use strict";
(function () {
  const target = document.getElementById("tab-teoria") || document.getElementById("logo");
  if (!target || TEORIA.esteso()) return;
  const note = document.createElement("div"); note.id = "sblocco"; note.setAttribute("role", "status"); document.body.appendChild(note);
  const still = () => document.documentElement.hasAttribute("data-a11y-reduce") || !target.animate || (matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches);
  let n = 0, t = 0;
  target.addEventListener("click", e => {
    if (e.button || e.ctrlKey || e.metaKey || e.shiftKey || e.altKey) return;   // «open in a new tab» and the like keep working
    e.preventDefault(); clearTimeout(t); n++;
    const s = 1 + 0.15 * n;
    if (n < 5) { if (!still()) target.animate([{ transform: "scale(" + (s - 0.15) + ")" }, { transform: "scale(" + s + ")" }], { duration: 140, fill: "forwards", easing: "ease-out" }); t = setTimeout(() => { location.href = target.href; }, 600); return; }
    try { localStorage.setItem("qqq.teoria.esteso", "1"); } catch (_) { /* not stored: the reload shows nothing new */ }
    if (!still()) target.animate([{ transform: "scale(1.5)", opacity: 1 }, { transform: "scale(2)", opacity: 0 }], { duration: 320, fill: "forwards", easing: "ease-out" });
    note.textContent = "Teoria estesa sbloccata"; note.className = "on";
    setTimeout(() => location.reload(), 900);
  });
})();
