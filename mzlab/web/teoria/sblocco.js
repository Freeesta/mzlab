/* mzLab - Teoria: five clicks on the logo unlock the extended Teoria and the chapter editor.
   Each click grows the logo (the image, not the link: the focus ring stays); the fifth one «bursts» it, a status message says what happened and the
   page unlocks the editor and extended mode. The logo stays a link: a click that is not followed by another within 600 ms goes where the link goes.
   No animation under reduced motion (the system setting or data-a11y-reduce). Classic script, no network: it works from file:// too. */
"use strict";
(function () {
  const target = document.getElementById("logo") || document.getElementById("tab-teoria");
  if (!target) return;
  let note = document.getElementById("sblocco");
  if (!note) { note = document.createElement("div"); note.id = "sblocco"; note.setAttribute("role", "status"); document.body.appendChild(note); }
  const still = () => document.documentElement.hasAttribute("data-a11y-reduce") || !target.animate || (matchMedia && matchMedia("(prefers-reduced-motion: reduce)").matches);
  let n = 0, t = 0;
  target.addEventListener("click", e => {
    if (e.button || e.ctrlKey || e.metaKey || e.shiftKey || e.altKey) return;   // «open in a new tab» and the like keep working
    e.preventDefault(); clearTimeout(t); n++;
    const s = 1 + 0.15 * n;
    if (n < 5) {
      if (!still()) target.animate([{ transform: "scale(" + (s - 0.15) + ")" }, { transform: "scale(" + s + ")" }], { duration: 140, fill: "forwards", easing: "ease-out" });
      t = setTimeout(() => { location.href = target.href; }, 600);
      return;
    }
    // 5 clicks reached: unlock editor and extended mode
    n = 0;
    try {
      localStorage.setItem("qqq.teoria.esteso", "1");
      localStorage.setItem("mzlab_teoria_editor", "1");
    } catch (_) {}
    if (!still()) target.animate([{ transform: "scale(1.5)", opacity: 1 }, { transform: "scale(2)", opacity: 0 }], { duration: 320, fill: "forwards", easing: "ease-out" });
    note.textContent = "Editor e modalità autore sbloccati"; note.className = "on";
    if (window.TEORIA_EDITOR) {
      window.TEORIA_EDITOR.attiva?.();
      setTimeout(() => {
        note.classList.remove("on");
        window.TEORIA_EDITOR.toggle();
      }, 800);
    } else {
      setTimeout(() => location.reload(), 900);
    }
  });
})();
