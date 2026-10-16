// @ts-strict
// Script loader of the page (replaces document.write): puts the Dati scripts in the page in the order of the list, as classic scripts that
// run in sequence (async=false); then the page shell (shell.js); then the ES modules (the leaves nobody calls by name), as before.
// carica(name) loads a script once and returns a Promise: for the tools that can wait for their first use.
(function () {
  var L = ["m:errori.js","stato.js","html-sicuro.js","file-sicuri.js","esperimenti.js","azioni-icone.js","azioni.js","suggerimenti.js","features.js","icons-modi.js","modi.js","ordine.js","ruoli-carica.js","ruoli-celle.js","progresso.js","conversione.js","nativo.js","diagnosi.js","m:lettori-famiglie.js","m:lettori-modulo.js","cartella-carica.js","cartella-puntata.js","cartella-puntata-desktop.js","agilent-d.js","waters-raw.js","metodo-m.js","metodo-incl.js","metodo-doc.js","explore.js","lista-file.js","layout.js","fondo.js","pannelli.js","affianca.js","legende.js","grafici.js","picchi.js","storia.js","asse.js","eventi.js","hr.js","composizione.js","elementare.js","visualizza.js","dda.js","cromato.js","calcola.js","perdite.js","liste.js","scroll.js","spettro.js","mirino.js","mappa-render.js","mappa-tabella.js","mappa.js","m:sottrazione.js","xlsx.js","provenienza.js","m:origine.js","elements.js","tabs.js","infusione.js","infusione-albero.js","infusione-menu.js","banco.js","settings.js","aggiorna.js","livelli-1.js","livelli.js","libreria.js","touch.js","qaqc.js","stat.js","campioni-doc.js","campioni.js","campioni-tabella.js","profilo.js","tabella.js","bianco.js","riferimento.js","pca.js","kmd-calc.js","kmd-vista.js","kmd.js","rapporto.js","m:workflow.js","m:gradiente.js","m:tdati.js","m:tdati-pannelli.js","m:registro-serie.js","m:metodo-registro.js","m:pfas.js","m:fair.js","m:qualita.js","tavola-periodica.js","tables.js","barra.js","paletta.js","help.js","m:transizioni.js","m:perf.js","molui.js","tpmine-loader.js","logo-piccone.js"];
  var done = {};
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
      s.onerror = function () { ko(new Error("script not loaded: " + f)); };
      document.body.appendChild(s);
    });
    return done[f];
  }
  window.carica = function (n) { return add(n); };
  if (window.QQQ_PHONE) { add("telefono-scheda.js"); return; }   // a smartphone gets only the card: no Dati, Disegno or shell
  var classic = L.filter(function (n) { return n.slice(0, 2) !== "m:"; });
  var modules = L.filter(function (n) { return n.slice(0, 2) === "m:"; });
  classic.concat(["shell.js"], modules).forEach(function (n) { add(n).catch(function (e) { console.error(e); }); });
})();
