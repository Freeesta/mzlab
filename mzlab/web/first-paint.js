// @ts-check
// @ts-strict
"use strict";
// The toolbar of the header is part of the first paint: its scripts are read here, in order and before the page is drawn, and barra.js
// draws it as soon as it is loaded (BARRA.primo); carica.js then skips them. Not on a smartphone, which has no toolbar.
const fp = /** @type {any} */ (window);
// Dati is the work area from the first paint, except on the address of another view (#/draw, #/flow, …): applyView takes over with the shell
if (/^#\/(?!data(\/|$))\w/.test(location.hash)) { const vd = document.getElementById("v-data"); if (vd) vd.hidden = true; }
if (!fp.QQQ_PHONE) {
  fp.QQ_FIRST_PAINT = true;
  ["features", "icons-modi", "barra"].forEach(n => document.write('<script src="static/' + n + '.js"><\/script>'));
}

// Disegno: the Ketcher bundle (about 29 MB of script) is what makes the editor late. A direct address (#/draw) starts its frame now, while the
// page scripts are still loading (draw.js adopts it and the "ketcher-ready" message seen here). Any other page has ONE policy: the bundle is
// hinted with low-priority <link rel=prefetch> (never read into memory, so nothing is aborted or copied), once the page has been loaded for
// 3 s and no file is being loaded (event mzlab-index quiet for 3 s), then at an idle moment; skipped with data saving on or when offline.
// The pointer or focus on "Disegno" (shell.js) calls MZLAB_WARM_KETCHER at once. Not on a smartphone, which has no Disegno.
// Opening Disegno then finds the bundle in the HTTP cache.
if (!fp.QQQ_PHONE) {
  const base = "static/vendor/ketcher/";
  const QUIET = 3000;
  const fr = /** @type {HTMLIFrameElement | null} */ (document.getElementById("kframe"));
  let warmed = false;
  const warm = () => {
    if (warmed || navigator.onLine === false) return;
    warmed = true;
    fetch(base + "index.html", { priority: "low" }).then(r => r.text()).then(html => {
      new DOMParser().parseFromString(html, "text/html").querySelectorAll("script[src], link[rel=stylesheet][href]").forEach(el => {
        const link = document.createElement("link");
        link.rel = "prefetch";
        link.setAttribute("fetchpriority", "low");
        const cors = el.getAttribute("crossorigin");
        if (cors !== null) link.crossOrigin = cors;
        link.href = new URL(el.getAttribute("src") || el.getAttribute("href") || "", new URL(base, location.href)).href;
        document.head.appendChild(link);
      });
    }).catch(() => { warmed = false; /* a failed hint changes nothing: the editor loads its own files; the next trigger may retry */ });
  };
  fp.MZLAB_WARM_KETCHER = warm;
  if (/^#\/draw(\/|$)/.test(location.hash) && fr && !fr.getAttribute("src")) {
    warmed = true;
    fr.removeAttribute("loading");
    fr.src = base + "index.html";
    addEventListener("message", e => { if (e.source === fr.contentWindow && e.data && e.data.type === "ketcher-ready") fp.MZLAB_KETCHER_READY = true; });
  } else {
    const nav = /** @type {any} */ (navigator).connection;
    if (!(nav && nav.saveData)) {
      let loadedAt = 0, lastIndex = 0;
      addEventListener("mzlab-index", () => { lastIndex = Date.now(); });
      const idle = (/** @type {() => void} */ f) => (window.requestIdleCallback || setTimeout)(f);
      const when = () => Math.max(loadedAt, lastIndex) + QUIET - Date.now();
      const attempt = () => {
        const wait = when();
        if (wait > 0) setTimeout(attempt, wait);
        else idle(() => (when() > 0 ? attempt() : warm()));
      };
      addEventListener("load", () => { loadedAt = Date.now(); attempt(); }, { once: true });
    }
  }
}
