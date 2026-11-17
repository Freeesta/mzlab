// @ts-strict
// Script loader of the page (replaces document.write): puts the Dati scripts in the page in the order of the list, as classic scripts that
// run in sequence (async=false); then the page shell (shell.js); then the ES modules (the leaves nobody calls by name), as before.
// carica(name) loads a script once and returns a Promise: for the tools that can wait for their first use.
(function () {
  var L = ["m:capacita.js","m:errori.js","stato.js","apertura-area.js","area-lavoro.js","drop-unico.js","tipo-file.js","avvio-progressivo.js","html-sicuro.js","file-sicuri.js","esperimenti.js","azioni-icone.js","azioni.js","azioni-tasti.js","azioni-esc.js","suggerimenti.js","tour-dopo.js","features.js","icons-modi.js","modi.js","ordine.js","campi.js","ruoli-carica.js","ruoli-celle.js","progresso.js","conversione.js","nativo.js","diagnosi.js","m:lettori-famiglie.js","m:lettori-modulo.js","m:valida-mzml.js","cartella-carica.js","cartella-puntata.js","agilent-d.js","waters-raw.js","metodo-m.js","metodo-incl.js","metodo-doc.js","chrom-primo.js","comandi.js","cache-dati.js","nb-pendiente.js","explore.js","lista-file.js","lista-file-menu.js","open-now.js","upload-rows.js","layout.js","fondo.js","fondo-spettro.js","mappa-vicino.js","mappa-margini.js","pannelli.js","xic-bordi.js","pannelli-annulla.js","affianca.js","legende.js","numeri.js","grafici.js","fondo-piatto.js","fondo-chip.js","picchi.js","storia.js","asse.js","eventi.js","tooltip.js","uv-azioni.js","hr.js","composizione.js","elementare.js","visualizza.js","dda.js","cromato.js","calcola.js","calc-addotti-html.js","calc-addotti.js","perdite.js","liste.js","calc-qui.js","calc-qui-html.js","calc-qui-ui.js","scroll.js","spettro-righello.js","spettro.js","mirino.js","mappa-render.js","mappa-dati.js","mappa-caldo.js","mappa-tabella.js","mappa.js","m:sottrazione.js","provenienza.js","elements.js","tabs.js","banco.js","tema-morbido.js","impostazioni-comp.js","settings.js","aggiorna.js","predefinito.js","livelli-1.js","livelli.js","libreria.js","touch.js","qaqc.js","stat.js","campioni-doc.js","campioni.js","campioni-tabella.js","profilo.js","tabella.js","bianco.js","riferimento.js","kmd-calc.js","rapporto.js","m:workflow.js","m:gradiente.js","m:tdati.js","m:tdati-pannelli.js","m:registro-serie.js","m:metodo-registro.js","m:fair-azioni.js","m:qualita.js","m:etichette-picchi.js","m:area-copia.js","tables.js","calc-qui-pannello.js","barra.js","help.js","m:transizioni.js","m:perf.js","m:titoli.js","m:barra-spettro.js","m:spettro-righe.js","m:xic-spettro.js","m:tabella-picchi.js","xic-lente-disegno.js","m:xic-lente.js","m:canali.js","m:sim-ui.js","m:zoom-reset.js","m:menu-sotto.js","molui.js","tpmine-loader.js","logo-piccone.js"];
  var done = {}, booted = false;
  if (window.NOMI) done["nomi.js"] = Promise.resolve("nomi.js");
  if (window.AVVIO) done["avvio-progressivo.js"] = Promise.resolve("avvio-progressivo.js");
  if (window.BARRA) ["features.js", "icons-modi.js", "barra.js"].forEach(function (f) { done[f] = Promise.resolve(f); });   // read by first-paint.js
  function add(n) {
    var mod = n.slice(0, 2) === "m:";
    var f = mod ? n.slice(2) : n;
    if (done[f]) return done[f];
    done[f] = new Promise(function (ok, ko) {
      var s = document.createElement("script");
      if (mod) s.type = "module";
      // after start-up a carica(n) runs as soon as it arrives, ahead of tools waiting idle: in order (async=false) it queued behind every lazy script asked for before it
      s.async = booted;
      if (booted) s.fetchPriority = "high";
      s.src = "static/" + f;
      s.onload = function () { ok(f); };
      // a failed script is forgotten, so a later carica(n) tries again
      s.onerror = function () { delete done[f]; if (s.parentNode) s.parentNode.removeChild(s); ko(new Error("script not loaded: " + f)); };
      document.body.appendChild(s);
    });
    return done[f];
  }
  window.carica = function (n) { return add(n); };
  // «Da dove viene questo ione?» (origine.js) is read on its first use: the module puts the real openOrigin in place of this one
  window.openOrigin = function (a) { return add("m:origine.js").then(function () { return window.openOrigin(a); }); };
  // Tools loaded on first use, not in L (docs/agenti/avvio.md): the download helpers and the command palette (Ctrl/Cmd+K) here, the
  // periodic table in tables.js, the MSn infusion tools in TIPOF.infusione (tipo-file.js). A stub waits for the script, which then replaces it.
  ["dlx", "dlc"].forEach(function (g) { window[g] = function () { var a = arguments; return add("xlsx.js").then(function () { return window[g].apply(null, a); }); }; });
  var opening = false;
  document.addEventListener("keydown", function (e) {
    if (window.PALETTE || opening || !(e.ctrlKey || e.metaKey) || e.altKey || e.shiftKey || (e.key !== "k" && e.key !== "K")) return;
    if (document.documentElement.classList.contains("phone")) return;
    e.preventDefault(); opening = true;
    // keys typed while the script loads are kept and replayed into the box, so the first Ctrl/Cmd+K loses nothing; a key aimed at another field is left alone
    var typed = [], keep = function (k) {
      var t = /** @type {HTMLElement | null} */ (k.target); if (t && t.closest && t.closest("input, textarea, select, [contenteditable]")) return;
      if (k.ctrlKey || k.metaKey || k.altKey) return;
      if (k.key.length === 1 || k.key === "Backspace" || k.key === "Enter") { k.preventDefault(); k.stopPropagation(); typed.push(k.key); }
    };
    var replay = function (/** @type {number} */ left) {
      var found = /** @type {HTMLInputElement | null} */ (document.getElementById("pal-in"));
      if (!found) { if (left > 0) setTimeout(function () { replay(left - 1); }, 50); else console.error("palette: box not found, " + typed.length + " keys dropped"); return; }
      var box = found;
      typed.forEach(function (k) {
        if (k === "Enter") box.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
        else { box.value = k === "Backspace" ? box.value.slice(0, -1) : box.value + k; box.dispatchEvent(new Event("input", { bubbles: true })); }
      });
    };
    document.addEventListener("keydown", keep, true);
    add("paletta.js").then(function () {
      document.removeEventListener("keydown", keep, true); window.PALETTE.open(); replay(1);
    }).catch(function (x) { console.error(x); }).then(function () { document.removeEventListener("keydown", keep, true); opening = false; });
  }, true);
  // a smartphone gets only the card: no Dati, Disegno or shell
  // the rest of the catalog first (i18n.js awaitRest): every script after it finds all its texts; a failed read is tried once more
  var R = window.I18N && window.I18N.rest;
  if (R) add(R).catch(function () { return add(R); }).catch(function (e) { console.error(e); });
  if (window.MZLAB_PHONE) { add("telefono-scheda.js"); return; }
  var classic = L.filter(function (n) { return n.slice(0, 2) !== "m:"; });
  var modules = L.filter(function (n) { return n.slice(0, 2) === "m:"; });
  var rest = classic.concat(["shell.js"], modules);
  // the tags go in by slices of 48 with a yield between them: all ~170 in one task were a 110-180 ms long task at CPU x4 (each insertion starts
  // its fetch); async=false keeps the run order across slices, and no carica(n) call names a script of L, so none can jump the queue
  var i = 0, all = [];
  var go = function () {
    rest.slice(i, i += 48).forEach(function (n) { all.push(add(n).catch(function (e) { console.error(e); })); });
    if (i < rest.length) setTimeout(go, 0);
    else Promise.all(all).then(function () { booted = true; });
  };
  go();
})();
