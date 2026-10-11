/* mzLab - Teoria: «Frammenta» (chapter 14). The two calculators of the chapter in a side panel: competition between a rearrangement and a
   simple cleavage (RRK model) and the neutral-loss calculator with the nitrogen-rule parity. Only the arithmetic: no verdict.
   The example comes from data-esempio on <html> (or from the argument of TEORIA.strumenti.frammenta). */
"use strict";
(function () {
  if (typeof TEORIA === "undefined" || !TEORIA.strumenti) return;
  // examples: precursor m/z and fragments (nominal). Add one by key and put data-esempio="key" on the <html> of the chapter.
  const ESEMPI = { atrazina: { p: "216", f: "174, 146, 132, 104", nome: "atrazina, [M+H]⁺ (esempio svolto del capitolo 14)" } };
  TEORIA.strumenti.registra("frammenta", {
    titolo: "Frammenta",
    needs: [[() => typeof SIMFRAG !== "undefined", "../sim-frag.js"]],
    monta(body, arg) {
      const k = typeof arg === "string" ? arg : document.documentElement.dataset.esempio;
      const es = arg && typeof arg === "object" ? { p: String(arg.p), f: String(arg.f), nome: arg.nome } : ESEMPI[k];
      body.innerHTML = `<p class="note">Due calcolatori del capitolo 14. ${es && es.nome ? "Esempio precaricato: " + es.nome + "." : ""} Cambiate i valori: il programma fa solo i conti, la scelta delle ipotesi resta vostra.</p>
<div class="sim" id="fr-loss"><h4>Perdite neutre e parità dell'azoto</h4></div><div class="sim" id="fr-rrk"><h4>Riarrangiamento o scissione diretta?</h4></div>`;
      SIMFRAG.perdite(body.querySelector("#fr-loss"), es || undefined);
      SIMFRAG.rrk(body.querySelector("#fr-rrk"));
    },
  });
})();
