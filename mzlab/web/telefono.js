// @ts-strict
// @ts-check
// mzLab on a smartphone. Dati and Disegno need a large screen and the Python engine (tens of MB): on a phone the site shows only
// the Teoria, and nothing heavy is downloaded. Loaded first by index.html (before browser.js); the Teoria pages repeat the same rule
// in teoria.js. A phone = a phone browser (user agent) or a touch screen whose short side is under 500 CSS px, in any orientation
// (a tablet is not a phone). ?telefono=1 / ?telefono=0 force the choice for this tab (tests, or a student who wants the full site anyway).
(function () {
  let force = null;
  try {
    const q = /[?&]telefono=([01])/.exec(location.search);
    // kept from the old name: renaming it would lose the users' data
    if (q) sessionStorage.setItem(NOMI.KEY("telefono"), q[1]);
    force = sessionStorage.getItem(NOMI.KEY("telefono"));
  } catch (_) { /* storage blocked: automatic choice */ }
  const ua = navigator.userAgent || "";
  const coarse = !!(/** @type {any} */ (window).matchMedia && matchMedia("(pointer:coarse)").matches);
  const auto = /iPhone|iPod|Android.+Mobile|Windows Phone|Mobile.+Firefox|Opera Mini/i.test(ua) || (coarse && Math.min(screen.width, screen.height) < 500);
  window.MZLAB_PHONE = force === null ? auto : force === "1";
  window.QQQ_PHONE = window.MZLAB_PHONE; // kept from the old name: backward compatibility
  if (window.MZLAB_PHONE) document.documentElement.classList.add("phone");
})();
