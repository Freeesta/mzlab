// @ts-strict
// Script loader of the page (replaces document.write): puts the Dati scripts in the page in the order of the list, as classic scripts that
// run in sequence (async=false); then the page shell (shell.js); then the ES modules (the leaves nobody calls by name), as before.
// carica(name) loads a script once and returns a Promise: for the tools that can wait for their first use.
(function () {
  var L = ["m:capacita.js","m:errori.js","stato.js","apertura-area.js","area-lavoro.js","tipo-file.js","avvio-progressivo.js","html-sicuro.js","file-sicuri.js","esperimenti.js","azioni-icone.js","azioni.js","azioni-tasti.js","azioni-esc.js","suggerimenti.js","tour-dopo.js","features.js","icons-modi.js","modi.js","ordine.js","ruoli-carica.js","ruoli-celle.js","progresso.js","conversione.js","nativo.js","diagnosi.js","m:lettori-famiglie.js","m:lettori-modulo.js","m:valida-mzml.js","cartella-carica.js","cartella-puntata.js","cartella-puntata-desktop.js","agilent-d.js","waters-raw.js","metodo-m.js","metodo-incl.js","metodo-doc.js","chrom-primo.js","comandi.js","cache-dati.js","explore.js","lista-file.js","lista-file-menu.js","layout.js","fondo.js","fondo-spettro.js","mappa-vicino.js","mappa-margini.js","pannelli.js","pannelli-annulla.js","affianca.js","legende.js","numeri.js","grafici.js","fondo-piatto.js","picchi.js","storia.js","asse.js","eventi.js","uv-azioni.js","hr.js","composizione.js","elementare.js","m:elementare-pannello.js","visualizza.js","dda.js","cromato.js","calcola.js","calc-addotti-html.js","calc-addotti.js","perdite.js","liste.js","calc-qui.js","calc-qui-html.js","calc-qui-ui.js","scroll.js","spettro-righello.js","spettro.js","mirino.js","mappa-render.js","mappa-dati.js","mappa-caldo.js","mappa-tabella.js","mappa.js","m:sottrazione.js","xlsx.js","provenienza.js","m:origine.js","elements.js","tabs.js","infusione.js","infusione-albero.js","infusione-menu.js","banco.js","tema-morbido.js","teoria/impostazioni-comp.js","settings.js","aggiorna.js","predefinito.js","livelli-1.js","livelli.js","libreria.js","touch.js","qaqc.js","stat.js","campioni-doc.js","campioni.js","campioni-tabella.js","profilo.js","tabella.js","bianco.js","riferimento.js","kmd-calc.js","kmd-vista.js","kmd.js","rapporto.js","m:workflow.js","m:gradiente.js","m:tdati.js","m:tdati-pannelli.js","m:registro-serie.js","m:metodo-registro.js","m:pfas.js","m:fair.js","m:qualita.js","m:etichette-picchi.js","tavola-periodica.js","tables.js","calc-qui-pannello.js","barra.js","ricerca.js","paletta.js","help.js","m:transizioni.js","m:perf.js","m:barra-spettro.js","m:xic-spettro.js","xic-lente-disegno.js","m:xic-lente.js","m:canali.js","molui.js","tpmine-loader.js","logo-piccone.js"];
  var done = {};
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
      s.async = false;
      s.src = "static/" + f;
      s.onload = function () { ok(f); };
      // a failed script is forgotten, so a later carica(n) tries again
      s.onerror = function () { delete done[f]; if (s.parentNode) s.parentNode.removeChild(s); ko(new Error("script not loaded: " + f)); };
      document.body.appendChild(s);
    });
    return done[f];
  }
  window.carica = function (n) { return add(n); };
  // a smartphone gets only the card: no Dati, Disegno or shell
  if (window.QQQ_PHONE) { add("telefono-scheda.js"); return; }
  var classic = L.filter(function (n) { return n.slice(0, 2) !== "m:"; });
  var modules = L.filter(function (n) { return n.slice(0, 2) === "m:"; });
  var rest = classic.concat(["shell.js"], modules);
  var go = function () { rest.forEach(function (n) { add(n).catch(function (e) { console.error(e); }); }); };
  go();
})();
