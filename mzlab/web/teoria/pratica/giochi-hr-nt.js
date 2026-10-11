/* mzLab - Pratica: the two HR levels of pratica-formula.html (game "hr-nt": H6 formula from a non-target ion, H8 fine isotopic structure).
   Classic script on top of esercizi-hr.js (HRNT), giochi-lc.js (LCUI helpers) and motore.js (PAL). Data are CONSTRUCTED for teaching.
   The program never gives the answer before an attempt: aids (3 levels) narrow the reasoning, the check gives numbers. */
"use strict";
const HRNTUI = (() => {
  const { fbSet, lock, open, aids, seed, num, last, ds } = LCUI;
  const { fx } = HRNT;
  const REASONS = [["", "scegliete…"], ["ok", "supera tutti i controlli"], ["parita", "regola dell'azoto (DBE non intero)"], ["dbe", "DBE negativo"], ["hc", "H/C fuori intervallo"], ["iso", "isotopi non compatibili"]];
  const FINE_TXT = { "13C": "¹³C", "15N": "¹⁵N", "2H": "²H", "33S": "³³S", "34S": "³⁴S", "13Cx2": "2 × ¹³C", "18O": "¹⁸O" };
  const opts = a => a.map(([v, t]) => `<option value="${v}">${t}</option>`).join("");
  const stepsOf = wk => wk.querySelectorAll(".st");
  const score = (...a) => a.reduce((x, y) => x + y, 0) / a.length;

  // =========================================================== H6: formula from a non-target ion
  function ntMarkup(e) {
    const rows = e.cands.map(c => `<tr><th scope="row">${c.id}</th><td>${PAL.fhtml(c.f)}</td><td class="n">${fx(c.mzT, 4)}</td><td class="n">${fx(c.ppm, 1)}</td>
      <td><select class="c1s" data-id="${c.id}" aria-label="Candidato ${c.id}, ${c.fs}: quale controllo non supera">${opts(REASONS)}</select></td></tr>`).join("");
    return `<h3>Formula da uno ione non-target</h3>
      <p>Uno ione [M+H]<sup>+</sup> misurato in alta risoluzione, senza sapere che cosa sia: <b>m/z ${fx(e.mz, 4)}</b>, con M+1 = <b>${fx(e.obs[1], 1)}%</b> e M+2 = <b>${fx(e.obs[2], 1)}%</b> di M. Il software propone queste formule della molecola neutra, tutte entro ±${e.tolPpm} ppm.</p>
      ${ds}<div class="hpbox noprint"></div>
      <div class="st"><h4 id="q1">1. Per ciascun candidato, quale controllo non supera?</h4>
      <p class="note">Controlli: regola dell'azoto (il DBE di una molecola neutra è intero), DBE ≥ 0, H/C tra ${HRNT.HC_MIN} e ${HRNT.HC_MAX}, isotopi (M+1 e M+2 attesi entro ${fx(HRNT.ISO_TOL, 1)} punti dai misurati). Se ne supera più d'uno, ne basta uno.</p>
      <div class="cin-tw"><table class="cin-tab" aria-label="Formule candidate"><tr><th>id</th><th>formula neutra</th><th>m/z teorica [M+H]<sup>+</sup></th><th>errore (ppm)</th><th>controllo</th></tr>${rows}</table></div>
      <div class="row"><button type="button" class="pri ck1">Controlla</button></div><div class="fb" role="status"></div></div>
      <div class="st off" inert><h4 id="q2">2. Quale formula tenete?</h4>
      <div class="opts" role="group" aria-labelledby="q2">${e.cands.map(c => `<button type="button" data-id="${c.id}">${c.id} · ${PAL.fhtml(c.f)}</button>`).join("")}</div><div class="fb" role="status"></div></div>`;
  }
  const ntNumbers = (e, c) => `<tr><th scope="row">${c.id}</th><td class="n">${fx(c.dbe, 1)}</td><td class="n">${fx(c.hc, 2)}</td><td class="n">${fx(c.expM1, 1)} · ${fx(c.expM2, 1)}</td></tr>`;
  function nt(fr, level) {
    const e = last.hrnt = HRNT.genNt(level, seed());
    fr.wk.innerHTML = ntMarkup(e);
    aids(fr.wk.querySelector(".hpbox"), e.help);
    const st = stepsOf(fr.wk);
    let s1 = 0;
    st[0].querySelector(".ck1").onclick = () => {
      const sel = [...st[0].querySelectorAll(".c1s")];
      if (sel.some(x => !x.value)) { fbSet(st[0].querySelector(".fb"), false, "Scegliete un controllo per ogni candidato."); return; }
      const good = sel.filter(x => HRNT.judgeReason(e.cands.find(c => c.id === x.dataset.id), x.value)).length;
      s1 = good / sel.length; lock(st[0]);
      fbSet(st[0].querySelector(".fb"), good === sel.length, `${good} risposte su ${sel.length} coerenti con i numeri. Misurati: M+1 = ${fx(e.obs[1], 1)}, M+2 = ${fx(e.obs[2], 1)}.
        <table class="cin-tab"><tr><th>id</th><th>DBE</th><th>H/C</th><th>M+1 · M+2 attesi (%)</th></tr>${e.cands.map(c => ntNumbers(e, c)).join("")}</table>`);
      open(st[1]);
    };
    st[1].querySelectorAll(".opts button").forEach(b => {
      b.onclick = () => {
        const c = e.cands.find(x => x.id === b.dataset.id), ok = c.truth;
        lock(st[1]); b.classList.add(ok ? "right" : "wrong");
        fbSet(st[1].querySelector(".fb"), ok, ok ? `Sì: ${PAL.fhtml(c.f)} supera tutti i controlli e nessun altro candidato li supera.` : `No: ${PAL.fhtml(c.f)} non supera ${c.fails.length ? c.fails.map(t => HRNT.TESTS[t]).join("; ") : "alcun controllo"}. La formula che supera tutti i controlli è ${PAL.fhtml(e.cands.find(x => x.truth).f)}.`);
        const s = score(s1, ok ? 1 : 0);
        PAL.record("hr-nt", e.id, e.d0, { hr: s, formula: s }, s * 100); fr.pts("hr-nt");
      };
    });
  }

  // =========================================================== H8: fine isotopic structure
  function fineMarkup(e) {
    const lab = FINE_TXT, rows = e.peaks.map(p => `<tr><th scope="row">${p.id}</th><td class="n">${fx(p.mz, 5)}</td><td class="n">${fx(p.rel, 2)}</td>
      <td><select class="c1s" data-id="${p.id}" aria-label="Picco ${p.id}, m/z ${fx(p.mz, 5)}: quale sostituzione isotopica">${opts([["", "scegliete…"], ...HRNT.FINE_LABELS.map(l => [l, lab[l]])])}</select></td></tr>`).join("");
    return `<h3>Isotopi fini</h3>
      <p>Spettro HR di un ione [M+H]<sup>+</sup> con M a <b>m/z ${fx(e.mz0, 4)}</b> (100). Intorno a M+1 e M+2 compaiono più picchi risolti, di intensità ≥ ${fx(HRNT.THRESHOLD, 2)}% di M.</p>
      ${ds}<div class="hpbox noprint"></div>
      <div class="st"><h4 id="q1">1. A quale sostituzione isotopica corrisponde ciascun picco?</h4>
      <div class="cin-tw"><table class="cin-tab" aria-label="Picchi risolti"><tr><th>id</th><th>m/z</th><th>intensità (% di M)</th><th>sostituzione</th></tr>${rows}</table></div>
      <div class="row"><button type="button" class="pri ck1">Controlla</button></div><div class="fb" role="status"></div></div>
      <div class="st off" inert><h4 id="q2">2. Quanti atomi di zolfo e quanti di azoto?</h4>
      <div class="row"><label>S <input class="nS" size="3" inputmode="numeric"></label><label>N <input class="nN" size="3" inputmode="numeric"></label><button type="button" class="pri ck2">Controlla</button></div><div class="fb" role="status"></div></div>`;
  }
  function fineNumbers(e) {
    return `<table class="cin-tab"><tr><th>id</th><th>Δm (u)</th><th>sostituzione</th><th>intensità per atomo (% di M)</th><th>atomi</th></tr>${e.peaks.map(p => {
      const k = HRNT.coherenceFine(e, p.id);
      return `<tr><th scope="row">${p.id}</th><td class="n">${fx(k.dm, 4)}</td><td>${FINE_TXT[k.label]}</td><td class="n">${k.perAtomPct == null ? "–" : fx(k.perAtomPct, 3)}</td><td class="n">${k.atoms == null ? "–" : fx(k.atoms, 1)}</td></tr>`;
    }).join("")}</table>`;
  }
  function fine(fr, level) {
    const e = last.hrfine = HRNT.genFine(level, seed());
    fr.wk.innerHTML = fineMarkup(e);
    aids(fr.wk.querySelector(".hpbox"), e.help);
    const st = stepsOf(fr.wk);
    let s1 = 0;
    st[0].querySelector(".ck1").onclick = () => {
      const sel = [...st[0].querySelectorAll(".c1s")];
      if (sel.some(x => !x.value)) { fbSet(st[0].querySelector(".fb"), false, "Scegliete una sostituzione per ogni picco."); return; }
      const good = sel.filter(x => e.peaks.find(p => p.id === x.dataset.id).label === x.value).length;
      s1 = good / sel.length; lock(st[0]);
      fbSet(st[0].querySelector(".fb"), good === sel.length, `${good} assegnazioni su ${sel.length} coerenti con le differenze di massa. Le differenze misurate e quelle attese per ogni sostituzione:${fineNumbers(e)}`);
      open(st[1]);
    };
    st[1].querySelector(".ck2").onclick = () => {
      const s = num(st[1].querySelector(".nS").value), n = num(st[1].querySelector(".nN").value);
      if (s == null || n == null) { fbSet(st[1].querySelector(".fb"), false, "Scrivete due numeri interi."); return; }
      const ok = s === e.nS && n === e.nN;
      lock(st[1]);
      fbSet(st[1].querySelector(".fb"), ok, ok ? `Sì: ${e.nS} S e ${e.nN} N, formula ${PAL.fhtml(e.formula)}.` : `No: la risposta (${s} S, ${n} N) non è coerente con le intensità; la formula usata per generare i dati era ${PAL.fhtml(e.formula)}.`);
      const t = score(s1, ok ? 1 : 0);
      PAL.record("hr-nt", e.id, e.d0, { hr: t, formula: t }, t * 100); fr.pts("hr-nt");
    };
  }
  /** entry point used by the formula page for levels 3 (H6) and 4 (H8) */
  const render = (fr, level) => (level === 3 ? nt(fr, 3) : fine(fr, 1));
  return { render, last };
})();
