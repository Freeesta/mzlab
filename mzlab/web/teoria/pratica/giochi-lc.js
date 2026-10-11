/* mzLab - Pratica: the two LC-MS games of pratica-tp.html ("lc-dm": delta m of a transformation product; "lc-esi": ESI+ and ESI-).
   Classic script on top of teoria.js (TP), motore.js (PAL) and esercizi-lcms.js (LCMS). Data are CONSTRUCTED for teaching (generic scaffolds).
   The program never gives the answer before an attempt: aids (3 levels) narrow the reasoning, the coherence check gives numbers. */
"use strict";
const GIOCHI_LC = (() => {
  const { fx, sgn, sub, SHIFT, ADDUCT_LABEL } = LCMS;
  const last = {};                                          // last exercise of each game (read by the tests)
  const num = v => { const x = parseFloat(String(v).replace(",", ".")); return Number.isFinite(x) ? x : null; };
  const LV = `<option value="1">1 · una trasformazione, ±2 ppm</option><option value="2" selected>2 · candidati quasi isobari, ±2 ppm</option><option value="3">3 · due trasformazioni, ±5 ppm</option>`;
  const LVE = `<option value="1">1 · un composto: M e azoto</option><option value="2" selected>2 · tre composti: accoppiare + e −</option><option value="3">3 · addotti: Na, NH<sub>4</sub>, formiato</option>`;

  function frame(root, lvOpts, onNew) {
    root.innerHTML = `<div class="gbar noprint"><label>Livello <select class="lv">${lvOpts}</select></label><button type="button" class="nw">Nuovo problema</button><span class="pts" role="status"></span></div><div class="wk"></div>`;
    const bar = root.querySelector(".gbar"), wk = root.querySelector(".wk");
    bar.querySelector(".nw").onclick = onNew;
    bar.querySelector(".lv").onchange = onNew;
    return { bar, wk, level: () => +bar.querySelector(".lv").value, pts: game => { const s = PAL.stats(game); bar.querySelector(".pts").textContent = s.plays ? `${s.plays} problemi · media ${s.mean}` : ""; } };
  }
  const seed = () => Math.floor(Math.random() * 2 ** 31);
  const fbSet = (el, ok, html) => { el.className = "fb " + (ok ? "ok" : "no"); el.innerHTML = html; };
  const lock = st => { st.querySelectorAll("button,input,select").forEach(b => { b.disabled = true; }); };
  const open = st => { st.classList.remove("off"); st.removeAttribute("inert"); const c = st.querySelector("button,input,select"); if (c) c.focus(); };   // the answered step is disabled: focus moves on
  const off = st => { st.classList.add("off"); st.setAttribute("inert", ""); };

  /** aids button: reveals the next of 3 levels (cumulative) in a live region */
  function aids(box, help) {
    let n = 0;
    box.innerHTML = `<div class="row"><button type="button" class="hp">Aiuto (livello 1 di 3)</button><span class="note">Gli aiuti non dicono la risposta.</span></div><div class="fb hi hpt"></div>`;
    const b = box.querySelector(".hp"), t = box.querySelector(".hpt");
    b.onclick = () => {
      n++;
      t.innerHTML = help.slice(0, n).map((h, i) => `<p><b>Aiuto ${i + 1}.</b> ${h}</p>`).join("");
      if (n >= help.length) { b.disabled = true; b.textContent = "Aiuti finiti"; } else b.textContent = `Aiuto (livello ${n + 1} di 3)`;
    };
    return box;
  }
  const ds = '<p class="note">Dati costruiti a scopo didattico: formule inventate, non misure.</p>';

  // =========================================================== H1: delta m of the transformation product
  function dm(root) {
    const fr = frame(root, LV, next);
    function next() {
      const e = last.dm = LCMS.genDm(fr.level(), seed());
      const nomTP = e.nomObs, evenIon = nomTP % 2 === 0;
      fr.wk.innerHTML = `<h3>Δm del prodotto di trasformazione</h3>
        <p>Precursore (formula nota) <b>${sub(e.precursor.f)}</b>, ${e.desc}: [M+H]<sup>+</sup> esatto <b>m/z ${fx(e.precursor.mz, 4)}</b>. Candidato prodotto di trasformazione, stessa corsa, [M+H]<sup>+</sup> misurato <b>m/z ${fx(e.obs, 4)}</b> (errore casuale entro ±${e.ppm} ppm).</p>
        ${ds}<div class="hpbox noprint"></div>
        <div class="st"><h4 id="q1">1. Quale trasformazione è compatibile con la differenza di massa?</h4>
        <div class="opts" role="group" aria-labelledby="q1">${e.options.map((o, i) => `<button type="button" data-i="${i}">${o.label} <small>(${sgn(o.dm, 4)})</small></button>`).join("")}</div><div class="fb"></div></div>
        <div class="st off" inert><h4 id="q2">2. Il prodotto ha un numero pari o dispari di atomi di azoto?</h4>
        <p class="note">Regola dell'azoto per [M+H]<sup>+</sup>: massa nominale dello ione ${nomTP}, ${evenIon ? "pari" : "dispari"}.</p>
        <div class="opts" role="group" aria-labelledby="q2"><button type="button" data-n="even">pari (anche zero)</button><button type="button" data-n="odd">dispari</button></div><div class="fb"></div></div>
        <div class="st off" inert><h4 id="q3">3. Quanti atomi di ossigeno ha il prodotto?</h4>
        <div class="row"><label>O <input class="no" size="3" inputmode="numeric" aria-labelledby="q3"></label><button type="button" class="pri ck">Controlla</button></div><div class="fb"></div></div>`;
      aids(fr.wk.querySelector(".hpbox"), e.help);
      const st = fr.wk.querySelectorAll(".st");
      let s1 = 0, s2 = 0;
      st[0].querySelectorAll(".opts button").forEach(b => b.onclick = () => {
        const o = e.options[+b.dataset.i], c = LCMS.coherenceDm(e, o.id), ok = o.id === e.truthId;
        b.classList.add(ok ? "right" : "wrong"); lock(st[0]); s1 = ok ? 1 : 0;
        const coh = `Δm misurato ${sgn(e.dmObs, 4)} u; per «${o.label.replace(/<[^>]+>/g, "")}» il Δm teorico è ${sgn(o.dm, 4)} u: scarto ${sgn(c.diffMDa, 1)} mDa (${sgn(c.ppm, 1)} ppm), ${c.within ? "dentro" : "fuori da"} ±${fx(e.tolMDa, 1)} mDa.` +
          (c.iso ? " Un picco isotopico non è un prodotto di trasformazione." : c.productFormula ? ` Formula del prodotto: ${sub(c.productFormula)}, DBE ${fx(c.dbe, c.dbe % 1 ? 1 : 0)}${c.dbeOk ? "" : " (non è una struttura possibile)"}.` : " Questa scelta non dà una formula possibile.");
        fbSet(st[0].querySelector(".fb"), ok, `${ok ? "Giusto." : `No: la trasformazione compatibile era <b>${e.truthLabel}</b>.`} ${coh}`);
        open(st[1]);
      });
      st[1].querySelectorAll(".opts button").forEach(b => b.onclick = () => {
        const ok = (b.dataset.n === "odd") === e.product.nParityOdd; s2 = ok ? 1 : 0;
        b.classList.add(ok ? "right" : "wrong"); lock(st[1]);
        fbSet(st[1].querySelector(".fb"), ok, `${ok ? "Giusto." : "No."} Il prodotto ha ${e.product.nN} N (${e.product.nParityOdd ? "dispari" : "pari"}): la massa nominale ${nomTP} è ${evenIon ? "pari" : "dispari"}, cioè N ${evenIon ? "dispari" : "pari"} per uno ione [M+H]<sup>+</sup>.${e.product.nN !== (e.precursor.formula.N || 0) ? " L'azoto è cambiato rispetto al precursore: la parità di N dice se un atomo di N è stato perso." : ""}`);
        open(st[2]);
      });
      const check = () => {
        const v = num(st[2].querySelector(".no").value); if (v === null) return;
        const ok = v === e.product.nO; lock(st[2]);
        fbSet(st[2].querySelector(".fb"), ok, `${ok ? "Giusto." : "No."} Il prodotto è <b>${sub(e.product.f)}</b> (${e.product.nO} O, DBE ${e.product.dbe}).`);
        const s = (s1 + s2 + (ok ? 1 : 0)) / 3, sc = { tp: s };
        if (e.level >= 2) sc.hr = s1;
        if (e.dmObs < 0) sc.perdite = s1;
        PAL.record("lc-dm", e.id, e.d0, sc, s * 100); fr.pts("lc-dm");
      };
      st[2].querySelector(".ck").onclick = check;
      st[2].querySelector(".no").onkeydown = ev => { if (ev.key === "Enter") check(); };
    }
    next(); fr.pts("lc-dm");
  }

  // =========================================================== H2: ESI+ and ESI-
  function esi(root) {
    const fr = frame(root, LVE, next);
    function next() {
      const e = last.esi = LCMS.genEsi(fr.level(), seed());
      const sig = s => `<tr><th scope="row">${s.id}</th><td>ESI${s.mode === "+" ? "+" : "−"}</td><td class="n">${fx(s.mz, 4)}</td></tr>`;
      const table = `<table class="eitab"><caption class="note">Segnali (m/z misurato, errore casuale entro ±${e.ppm} ppm)</caption><tr><th scope="col">Segnale</th><th scope="col">Modo</th><th scope="col">m/z</th></tr>${[...e.pos, ...e.neg].map(sig).join("")}</table>`;
      const nOpts = `<option value="">—</option><option value="even">pari (anche zero)</option><option value="odd">dispari</option>`;
      let body, solve;
      if (e.task === "M") {
        body = `<div class="st"><h4>Quale massa neutra M, e quanti N?</h4><p class="note">Questi due segnali sono dello stesso composto.</p>
          <div class="row"><label>M (u, 3 decimali) <input class="vm" size="9" inputmode="decimal"></label><label>N <select class="vn">${nOpts}</select></label><button type="button" class="pri ck">Controlla</button></div><div class="fb"></div></div>`;
        solve = () => {
          const c = e.compounds[0], m = num(root.querySelector(".vm").value), n = root.querySelector(".vn").value; if (m === null || !n) return null;
          const a = Math.abs(m - c.M) <= 0.01, b = (n === "odd") === c.nOdd;
          return { items: [a, b], html: `${a ? "Giusto" : "No"}, M = <b>${fx(c.M, 4)}</b> (da [M+H]<sup>+</sup>: ${fx(e.pos[0].mz, 4)} − ${fx(SHIFT["[M+H]+"], 4)} = ${fx(e.pos[0].mz - SHIFT["[M+H]+"], 4)}; da [M−H]<sup>−</sup>: ${fx(e.neg[0].mz, 4)} + ${fx(-SHIFT["[M-H]-"], 4)} = ${fx(e.neg[0].mz - SHIFT["[M-H]-"], 4)}). N: ${b ? "giusto" : "no"}, ${c.nN} atomi, ${c.nOdd ? "dispari" : "pari"}; massa nominale dello ione ${Math.round(e.pos[0].mz)}.` };
        };
      } else if (e.task === "pair") {
        const negOpts = `<option value="">—</option>` + e.neg.map(s => `<option value="${s.id}">${s.id} · ${fx(s.mz, 4)}</option>`).join("");
        body = `<div class="st"><h4>Accoppia ogni segnale ESI+ con il suo ESI− e dite se N è pari o dispari</h4>
          <table class="eitab"><tr><th scope="col">ESI+</th><th scope="col">m/z</th><th scope="col">partner ESI−</th><th scope="col">N</th></tr>${e.pos.map(s => `<tr><th scope="row">${s.id}</th><td class="n">${fx(s.mz, 4)}</td><td><select class="pr" data-p="${s.id}" aria-label="Partner ESI− di ${s.id}">${negOpts}</select></td><td><select class="pn" data-p="${s.id}" aria-label="N del composto di ${s.id}">${nOpts}</select></td></tr>`).join("")}</table>
          <button type="button" class="pri ck">Controlla</button><div class="fb"></div></div>`;
        solve = () => {
          const rows = e.pos.map(s => ({ s, p: root.querySelector(`.pr[data-p=${s.id}]`).value, n: root.querySelector(`.pn[data-p=${s.id}]`).value }));
          if (rows.some(r => !r.p || !r.n)) return null;
          const items = []; const lines = rows.map(r => {
            const c = e.compounds.find(x => x.id === r.s.comp), tn = e.neg.find(x => x.comp === c.id);
            const a = r.p === tn.id, b = (r.n === "odd") === c.nOdd; items.push(a, b);
            return `${r.s.id}: partner ${tn.id} (${a ? "ok" : "no"}), M = ${fx(c.M, 4)}, N ${c.nOdd ? "dispari" : "pari"} (${b ? "ok" : "no"})`;
          });
          return { items, html: lines.join("<br>") };
        };
      } else {
        const adOpts = `<option value="">—</option>` + e.adductChoices.map(a => `<option value="${a}">${a.replace("-", "−")}</option>`).join("");
        body = `<div class="st"><h4>Assegna lo ione a ogni segnale, poi M e la parità di N</h4>
          <table class="eitab"><tr><th scope="col">Segnale</th><th scope="col">m/z</th><th scope="col">Ione</th></tr>${[...e.pos, ...e.neg].map(s => `<tr><th scope="row">${s.id}</th><td class="n">${fx(s.mz, 4)}</td><td><select class="ad" data-p="${s.id}" aria-label="Ione del segnale ${s.id}">${adOpts}</select></td></tr>`).join("")}</table>
          <div class="row"><label>M (u) <input class="vm" size="9" inputmode="decimal"></label><label>N <select class="vn">${nOpts}</select></label><button type="button" class="pri ck">Controlla</button></div><div class="fb"></div></div>`;
        solve = () => {
          const c = e.compounds[0], m = num(root.querySelector(".vm").value), n = root.querySelector(".vn").value;
          const ads = [...e.pos, ...e.neg].map(s => ({ s, v: root.querySelector(`.ad[data-p=${s.id}]`).value }));
          if (m === null || !n || ads.some(a => !a.v)) return null;
          const items = ads.map(a => a.v === a.s.adduct); items.push(Math.abs(m - c.M) <= 0.01, (n === "odd") === c.nOdd);
          return { items, html: `${ads.map(a => `${a.s.id} = ${ADDUCT_LABEL[a.s.adduct]} (${a.v === a.s.adduct ? "ok" : "no"})`).join(", ")}. M = <b>${fx(c.M, 4)}</b> (${items[items.length - 2] ? "ok" : "no"}); N ${c.nOdd ? "dispari" : "pari"} (${items[items.length - 1] ? "ok" : "no"}).` };
        };
      }
      fr.wk.innerHTML = `<h3>ESI+ ed ESI−</h3><p>${e.task === "M" ? "Un composto generico, un segnale in ESI+ e uno in ESI−." : e.task === "pair" ? "Tre composti generici: i segnali ESI+ e ESI− sono mescolati." : "Un composto generico con più segnali, anche addotti."} Strumenti del programma: la tabella degli addotti e la calcolatrice (vista Dati).</p>${ds}<div class="hpbox noprint"></div>${table}${body}`;
      aids(fr.wk.querySelector(".hpbox"), e.help);
      const st = fr.wk.querySelector(".st"), check = () => {
        const r = solve(); if (!r) { fbSet(st.querySelector(".fb"), false, "Completate tutte le risposte."); return; }
        const s = r.items.filter(Boolean).length / r.items.length; lock(st);
        fbSet(st.querySelector(".fb"), s === 1, `${s === 1 ? "Tutto giusto." : `${r.items.filter(Boolean).length} su ${r.items.length}.`} ${r.html}`);
        PAL.record("lc-esi", e.id, e.d0, { tp: s, ion: s }, s * 100); fr.pts("lc-esi");
      };
      st.querySelector(".ck").onclick = check;
    }
    next(); fr.pts("lc-esi");
  }
  return { dm, esi, last };
})();
