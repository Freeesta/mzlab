/* mzLab - Teoria: «Tavola periodica con isotopi» (chapters 11, 13, 22). The table is the one of the program (tavola-periodica.js,
   data in elements.js); here it only gets the Italian texts. Hover or focus an element to read masses and abundances, click or Enter pins it. */
"use strict";
(function () {
  if (typeof TEORIA === "undefined" || !TEORIA.strumenti) return;
  const T = {
    "tbl.na": "n.d.",
    "tbl.pt.avgWeight": "peso atomico medio <b>{w}</b> <span class=\"muted\">(media pesata sugli isotopi naturali)</span>",
    "tbl.pt.mono": "massa monoisotopica <b>{m}</b> <span class=\"muted\">(<sup>{a}</sup>{s}, l'isotopo più abbondante: è quella che si usa in MS)</span>",
    "tbl.pt.col.iso": "isotopo", "tbl.pt.col.mass": "massa esatta", "tbl.pt.col.abund": "abbondanza %", "tbl.pt.col.delta": "&Delta; dal principale",
    "tbl.pt.peak": "Ogni atomo di {s} nella molecola dà un picco a {off} con circa il {pct}% dell'intensità del picco principale.",
    "tbl.pt.rare": "Elemento raro in LC-MS: abbondanze isotopiche non incluse nel programma.",
    "tbl.pt.gap": "lantanidi e attinidi", "tbl.pt.info": "Isotopi e masse dell'elemento",
  };
  const t = (k, p) => (T[k] || k).replace(/\{(\w+)\}/g, (_, n) => (p && p[n] != null ? p[n] : ""));
  TEORIA.strumenti.registra("tavola", {
    titolo: "Tavola periodica con isotopi",
    needs: [[() => typeof ELEMENTS !== "undefined", "../../elements.js"], [() => typeof TAVOLA !== "undefined", "../../tavola-periodica.js"]],
    monta(body, arg) {
      body.innerHTML = '<p class="note">Passate sopra un elemento (o spostatevi con Tab) per leggere massa esatta e abbondanze dei suoi isotopi; Invio o clic lo blocca. Gli elementi evidenziati sono i più comuni nelle molecole organiche.</p><div class="ptw"></div>';
      const host = body.querySelector(".ptw");
      const P = TAVOLA.create({ elements: ELEMENTS, t, nome: e => e.n });
      host.innerHTML = P.periodic(); P.bindPeriodic(host);
      host.style.overflowX = "auto"; host.querySelector(".pt").style.minWidth = "520px";
      const c = arg && host.querySelector(`.pt-c[data-s="${String(arg).replace(/"/g, "")}"]`); if (c) c.click();
    },
  });
})();
