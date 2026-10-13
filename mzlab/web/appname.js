// @ts-check
// @ts-strict
// The visible name of the program: the ONLY place where it is written (chosen 7/10/2026; package and internal name is mzlab).
// Loaded first by index.html and by every page of the Teoria. In HTML a «{APP}» in <title> or in the title/alt/aria-label attributes becomes the name.
window.APP_NAME = "mzLab";
document.title = document.title.split("{APP}").join(APP_NAME);
document.addEventListener("DOMContentLoaded", () => {
  document.title = document.title.split("{APP}").join(APP_NAME);
  document.querySelectorAll("[title*='{APP}'],[alt*='{APP}'],[aria-label*='{APP}']").forEach(e => ["title", "alt", "aria-label"].forEach(a => { const v = e.getAttribute(a); if (v && v.includes("{APP}")) e.setAttribute(a, v.split("{APP}").join(APP_NAME)); }));
});
