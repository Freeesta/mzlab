// @ts-check
// Lab build only (tools/build_site.py with MZLAB_LAB=1; copied to static/lab.js and loaded first in <head>). Two jobs:
//  1. expiry: after the date baked into the build (UTC, whole day included) the page is replaced by a short notice in English and Italian; nothing else
//     is loaded, no data is read. The date cannot be computed (bad date, no clock) = expired, never open. This is a SOFT block, not security: whoever
//     controls their own browser (clock, devtools, offline copy) can get round it. It only keeps the lab page from living on by itself.
//  2. no examples: see below (the build deletes static/esempi and the example drawings; here their buttons go).
//  3. only Data and Draw: window.MZLAB_LAB tells the Tabui loader (tpmine-loader.js, which brings Tabui and mzFlow) and carica.js not to start, so no
//     package is downloaded and no tab is drawn; addresses of any other view go back to #/data. The administrator flag (the Libro editor and the like)
//     reads as «off» and cannot be written. Nothing is deleted from the program: the normal build is untouched.
(function () {
  "use strict";
  var w = /** @type {any} */ (window);
  var EXPIRES = "__LAB_EXPIRES__";
  var DAY = 86400000;

  /** true when the lab page must not run: after the last day, or when the date or the clock cannot be read */
  function expired() {
    try {
      if (!/^\d{4}-\d{2}-\d{2}$/.test(EXPIRES)) return true;
      var end = Date.parse(EXPIRES + "T00:00:00Z") + DAY;
      var now = Date.now();
      if (!isFinite(end) || typeof now !== "number" || !isFinite(now)) return true;
      return now >= end;
    } catch (e) { return true; }
  }

  function showExpired() {
    try { w.stop(); } catch (e) { /* nothing to stop */ }
    var d = document;
    d.documentElement.innerHTML =
      '<head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="robots" content="noindex,nofollow">' +
      "<title>Expired · Scaduta</title><style>" +
      ":root{--bg:#f7f8fa;--fg:#1b1f24;--line:#c9ced6}@media(prefers-color-scheme:dark){:root{--bg:#14171b;--fg:#e8eaed;--line:#3a4048}}" +
      "body{margin:0;min-height:100vh;display:flex;align-items:center;justify-content:center;background:var(--bg);color:var(--fg);font:16px/1.5 system-ui,sans-serif}" +
      "main{max-width:34em;margin:16px;padding:20px 24px;border:1px solid var(--line);border-radius:8px}h1{font-size:20px;margin:0 0 10px}p{margin:0 0 10px}" +
      '</style></head><body><main role="alert"><h1>This lab version has expired</h1>' +
      '<p lang="en">This copy of the program was made available for a limited period, which has ended. Please use the main site.</p>' +
      '<h1 lang="it">Questa versione di laboratorio è scaduta</h1>' +
      '<p lang="it">Questa copia del programma era disponibile per un periodo limitato, che è terminato. Usa il sito principale.</p></main></body>';
  }

  if (expired()) { showExpired(); return; }

  w.MZLAB_LAB = Object.freeze({ expires: EXPIRES });
  // no app download in the lab, whatever ?sito= says or was set before (sito.js keeps a value that is already there)
  w.SITO_PUBBLICO = Object.freeze({ scaricaApp: false });

  // the administrator flag (and its older name) reads as off and cannot be written: one choke point for every current and future reader
  try {
    var HIDDEN = /^(mzlab|qqq)\.admin$/;
    var P = Storage.prototype, get = P.getItem, set = P.setItem;
    P.getItem = function (k) { return HIDDEN.test(String(k)) ? null : get.call(this, k); };
    P.setItem = function (k, v) { if (!HIDDEN.test(String(k))) set.call(this, k, v); };
  } catch (e) { /* no storage: nothing to hide */ }

  // no example files in the lab (the build does not ship them either): the example-set chooser of Data, the example-drawings link of Draw and the
  // tour's example loader are hidden / inert. The elements stay in the DOM (draw.js binds #ex-link when it loads), hidden by CSS and `hidden`.
  try {
    var css = document.createElement("style");
    css.textContent = "#demo,#ex-link,:has(> #ex-link){display:none!important}";
    document.head.appendChild(css);
    var hideExamples = function () {
      var ids = ["demo", "ex-link"];
      for (var i = 0; i < ids.length; i++) {
        var el = document.getElementById(ids[i]);
        if (el) { el.hidden = true; if (ids[i] === "ex-link" && el.parentElement) el.parentElement.hidden = true; }
      }
    };
    document.addEventListener("DOMContentLoaded", hideExamples);
    var tries = 0, wait = setInterval(function () {
      if (w.TOUR) { w.TOUR.caricaEsempio = function () { return Promise.resolve(); }; clearInterval(wait); }
      else if (++tries > 200) clearInterval(wait);
    }, 100);
  } catch (e) { /* the build already ships no examples: nothing to fetch */ }

  // only #/data and #/draw exist; any other address (#/flow, #/tpmine, #/attributio, #/theory, #/practice...) becomes #/data
  function route() {
    try {
      var m = /^#\/(\w+)/.exec(location.hash);
      if (m && m[1] !== "data" && m[1] !== "draw") history.replaceState(null, "", location.pathname + location.search + "#/data");
    } catch (e) { /* old browser */ }
  }
  // the Tabui and Flow tabs written in index.html have nothing behind them here
  document.addEventListener("DOMContentLoaded", function () {
    var late = document.querySelectorAll("#nav [data-late]");
    for (var i = 0; i < late.length; i++) late[i].remove();
  });
  route();
  addEventListener("hashchange", route);
})();
