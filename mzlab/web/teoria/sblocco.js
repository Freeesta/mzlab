/* mzLab - Teoria: five clicks on the logo switch the administrator mode on or off (QADMIN of teoria.js, key «mzlab.admin» in localStorage): extended Teoria and chapter editor.
   Each click grows the logo (the image, not the link: the focus ring stays); the fifth one «bursts» it, a status message says what happened and the
   page unlocks the editor and extended mode (or locks them again) after a reload. The logo stays a link: a click that is not followed by another within 600 ms goes where the link goes.
   No animation under reduced motion (the system setting or data-a11y-reduce). Classic script, no network: it works from file:// too. */
"use strict";
(function () {
  const target = document.getElementById("logo") || document.getElementById("tab-teoria");
  if (!target) return;
  const fx = target.querySelector("img") || target;   // an inline link does not scale: animate the image
  let note = document.getElementById("sblocco");
  if (!note) { note = document.createElement("div"); note.id = "sblocco"; note.setAttribute("role", "status"); document.body.appendChild(note); }
  const still = () => document.documentElement.hasAttribute("data-a11y-reduce") || !fx.animate || (matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches);
  let n = 0, t = 0;
  target.addEventListener("click", e => {
    if (e.button || e.ctrlKey || e.metaKey || e.shiftKey || e.altKey) return;   // «open in a new tab» and the like keep working
    e.preventDefault(); clearTimeout(t); n++;
    const s = 1 + 0.15 * n;
    if (n < 5) { if (!still()) fx.animate([{ transform: "scale(" + (s - 0.15) + ")" }, { transform: "scale(" + s + ")" }], { duration: 140, fill: "forwards", easing: "ease-out" }); t = setTimeout(() => { location.href = target.href; }, 600); return; }
    const was = QADMIN.on(); if (was) QADMIN.clear(); else QADMIN.set();
    if (!still()) fx.animate([{ transform: "scale(1.5)", opacity: 1 }, { transform: "scale(2)", opacity: 0 }], { duration: 320, fill: "forwards", easing: "ease-out" });
    note.textContent = was ? "Modalità amministratore disattivata" : "Sbloccato"; note.className = "on";
    setTimeout(() => location.reload(), 900);
  });
})();
