/* mzLab - Teoria: «Massa e addotti» (chapters 6 and 13). From a formula: nominal and monoisotopic mass atom by atom, then [M+H]+, [M+Na]+, [M−H]−.
   Only the arithmetic, shown step by step: no identification. Formula parser and masses are PAL.parse / PAL.exact (pratica/motore.js, elements.js). */
"use strict";
(function () {
  const ELECTRON = 0.00054858;                       // mass of the electron (u): an ion is the atom mass minus (or plus) the electrons
  /** Rounds half up (0.5 goes up, as in the course), tolerant of binary representation errors. */
  const halfUp = (x, d) => { const k = Math.pow(10, d), s = x < 0 ? -1 : 1; return s * Math.floor(Math.abs(x) * k + 0.5 + 1e-7) / k; };
  const fmt = (x, d) => halfUp(x, d).toFixed(d).replace(".", ",").replace("-", "−");
  const sign = x => (x < 0 ? "−" : "+");
  /** Pure calculation: pal = PAL, el = ELEMENTS. Returns null for an invalid formula. */
  function calcola(formula, pal, el) {
    const f = pal.parse(formula);
    if (!f || !Object.keys(f).length) return null;
    const ordine = ["C", "H", ...Object.keys(f).filter(e => e !== "C" && e !== "H").sort()].filter(e => f[e]);
    const righe = ordine.map(e => {
      const n = f[e], nom = pal.NOMINAL[e], m = pal.exact({ [e]: 1 }, el);
      return { e, n, nom, nomTot: nom * n, m, mTot: m * n };
    });
    const Mnom = righe.reduce((a, r) => a + r.nomTot, 0), M = righe.reduce((a, r) => a + r.mTot, 0);
    const mH = pal.exact({ H: 1 }, el), mNa = pal.exact({ Na: 1 }, el);
    const ioni = [
      { nome: "[M+H]⁺", nom: Mnom + 1, passo: `${fmt(M, 4)} + ${fmt(mH, 4)} − ${fmt(ELECTRON, 4)}`, mz: M + mH - ELECTRON, z: "+" },
      { nome: "[M+Na]⁺", nom: Mnom + 23, passo: `${fmt(M, 4)} + ${fmt(mNa, 4)} − ${fmt(ELECTRON, 4)}`, mz: M + mNa - ELECTRON, z: "+" },
      { nome: "[M−H]⁻", nom: Mnom - 1, passo: `${fmt(M, 4)} − ${fmt(mH, 4)} + ${fmt(ELECTRON, 4)}`, mz: M - mH + ELECTRON, z: "−" },
    ];
    return { f, righe, Mnom, M, ioni, formula: pal.fstr(f), formulaHtml: pal.fhtml(f) };
  }
  if (typeof module !== "undefined") module.exports = { halfUp, fmt, calcola, ELECTRON };
  if (typeof TEORIA === "undefined" || !TEORIA.strumenti) return;

  TEORIA.strumenti.registra("massa", {
    titolo: "Massa e addotti",
    needs: [[() => typeof ELEMENTS !== "undefined", "../../elements.js"], [() => typeof PAL !== "undefined", "../pratica/motore.js"]],
    monta(body, arg) {
      body.innerHTML = `<p class="note">Scrivete una formula (C, H, N, O, F, Si, P, S, Cl, Br, I). Lo strumento somma le masse atomo per atomo: non dice quale composto sia.</p>
<label for="ms-f"><b>Formula</b></label><br><input id="ms-f" type="text" value="${arg ? String(arg).replace(/"/g, "") : "C8H14ClN5"}" spellcheck="false" autocomplete="off" aria-describedby="ms-st" style="width:100%">
<div id="ms-st" role="status" aria-live="polite" class="note"></div><div id="ms-out"></div>`;
      const inp = body.querySelector("#ms-f"), out = body.querySelector("#ms-out"), st = body.querySelector("#ms-st");
      function draw() {
        const r = calcola(inp.value, PAL, ELEMENTS);
        if (!r) { out.innerHTML = ""; st.textContent = inp.value.trim() ? "Formula non valida: servono simboli (C, H, N, O, F, Si, P, S, Cl, Br, I) seguiti dal numero di atomi." : "Scrivete una formula."; return; }
        st.textContent = `Formula ${r.formula}: massa nominale ${r.Mnom}, monoisotopica ${fmt(r.M, 4)}.`;
        out.innerHTML = `<h3>Atomo per atomo</h3><div class="tw"><table><caption class="note">Massa nominale = isotopo più leggero e abbondante, intero; massa monoisotopica = massa esatta di quell'isotopo.</caption>
<tr><th scope="col">Atomo</th><th scope="col" class="n">n</th><th scope="col" class="n">nominale</th><th scope="col" class="n">n × nominale</th><th scope="col" class="n">massa esatta</th><th scope="col" class="n">n × massa esatta</th></tr>` +
          r.righe.map(x => `<tr><th scope="row">${x.e}</th><td class="n">${x.n}</td><td class="n">${x.nom}</td><td class="n">${x.nomTot}</td><td class="n">${fmt(x.m, 6)}</td><td class="n">${fmt(x.mTot, 6)}</td></tr>`).join("") +
          `<tr><th scope="row">Somma</th><td></td><td></td><td class="n"><b>${r.Mnom}</b></td><td></td><td class="n"><b>${fmt(r.M, 6)}</b></td></tr></table></div>
<p>M nominale = <b>${r.Mnom}</b>; M monoisotopica = <b>${fmt(r.M, 4)}</b> (arrotondata a 4 decimali con il metodo half-up: 0,5 va sempre in su).</p>
<h3>Ioni</h3><div class="tw"><table><tr><th scope="col">Ione</th><th scope="col" class="n">m/z nominale</th><th scope="col">Conto</th><th scope="col" class="n">m/z esatto</th></tr>` +
          r.ioni.map(x => `<tr><th scope="row">${x.nome}</th><td class="n">${x.nom}</td><td>${x.passo}</td><td class="n"><b>${fmt(x.mz, 4)}</b></td></tr>`).join("") +
          `</table></div><p class="note">Si aggiunge o si toglie la massa dell'atomo (H o Na) e si corregge per la massa dell'elettrone (0,0005 u): uno ione positivo ha un elettrone in meno, uno negativo uno in più. Un m/z uguale a questo conto è compatibile con la formula, non la identifica.</p>`;
      }
      inp.addEventListener("input", draw); draw();
    },
  });
})();
