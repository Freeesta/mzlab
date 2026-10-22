// @ts-check
"use strict";
// The toolbar of the header is part of the first paint: its scripts are read here, in order and before the page is drawn, and barra.js
// draws it as soon as it is loaded (BARRA.primo); carica.js then skips them. Not on a smartphone, which has no toolbar.
const fp = /** @type {any} */ (window);
if (!fp.QQQ_PHONE) {
  fp.QQ_FIRST_PAINT = true;
  ["features", "icons-modi", "barra"].forEach(n => document.write('<script src="static/' + n + '.js"><\/script>'));
}

// Disegno: the Ketcher bundle (about 29 MB of script) is what makes the editor late. A direct address (#/draw) starts its frame now, while the
// page scripts are still loading (draw.js adopts it and the "ketcher-ready" message seen here); any other page prefetches the bundle once idle
// (once per session; not on a smartphone, which has no Disegno, nor with data saving on), so opening Disegno finds it in the HTTP cache.
if (!fp.QQQ_PHONE) {
  const base = "static/vendor/ketcher/";
  const fr = /** @type {HTMLIFrameElement | null} */ (document.getElementById("kframe"));
  if (/^#\/draw(\/|$)/.test(location.hash) && fr && !fr.getAttribute("src")) {
    fr.removeAttribute("loading");
    fr.src = base + "index.html";
    addEventListener("message", e => { if (e.source === fr.contentWindow && e.data && e.data.type === "ketcher-ready") fp.MZLAB_KETCHER_READY = true; });
  } else {
    const nav = /** @type {any} */ (navigator).connection;
    const warm = () => fetch(base + "index.html").then(r => r.text()).then(html => {
      new DOMParser().parseFromString(html, "text/html").querySelectorAll("script[src], link[rel=stylesheet][href]").forEach(el => {
        const link = document.createElement("link");
        link.rel = "prefetch";
        const cors = el.getAttribute("crossorigin");
        if (cors !== null) link.crossOrigin = cors;
        link.href = new URL(el.getAttribute("src") || el.getAttribute("href") || "", new URL(base, location.href)).href;
        document.head.appendChild(link);
      });
    }).catch(() => { /* a failed hint changes nothing: the editor loads its own files */ });
    if (!(nav && nav.saveData)) addEventListener("load", () => {
      let seen = false;
      try { seen = sessionStorage.getItem("mzlab.kprefetch") === "1"; sessionStorage.setItem("mzlab.kprefetch", "1"); } catch (_) { /* storage blocked */ }
      if (!seen) setTimeout(() => (window.requestIdleCallback || setTimeout)(warm), 3000);
    }, { once: true });
  }
}
