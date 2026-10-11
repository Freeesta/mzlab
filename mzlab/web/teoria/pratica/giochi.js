/* QqQ lab - Pratica: the short games (pages pratica-perdite, -isotopi, -formula, -strumento, -quadrupolo).
   Classic script on top of teoria.js (TP), motore.js (PAL) and ../elements.js (ELEMENTS). Each page calls GIOCHI.<name>(element).
   Every problem is generated here from the chemistry and the physics (no data file): neutral losses from their formulas,
   isotope clusters by convolution of the natural abundances, candidate formulas by enumeration, quadrupole stability
   from the boundaries of the first stability region of the Mathieu equation. */
"use strict";
const GIOCHI = (() => {
  const EH = t => String(t).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
  const rnd = (a, b) => a + Math.floor(Math.random() * (b - a + 1));
  const pickOne = a => a[Math.floor(Math.random() * a.length)];
  const fx = (v, d) => v.toFixed(d).replace(".", ",");
  const H1 = 1.007276, EM = 0.000548580;   // proton, electron
  const sub = s => String(s).replace(/(\d+)/g, "<sub>$1</sub>");

  /** Common frame: a bar (options, "Nuovo problema", points) and the problem area. */
  function frame(root, opts, onNew) {
    root.innerHTML = `<div class="gbar noprint">${opts}<button class="nw">Nuovo problema</button><span class="pts"></span></div><div class="wk"></div>`;
    const bar = root.querySelector(".gbar"), wk = root.querySelector(".wk");
    bar.querySelector(".nw").onclick = onNew;
    bar.querySelectorAll("select").forEach(s => { s.onchange = onNew; });
    return { bar, wk, pts: game => { const s = PAL.stats(game); bar.querySelector(".pts").textContent = s.plays ? `${s.plays} problemi · media ${s.mean}` : ""; } };
  }

  // =========================================================== 1. neutral losses
  // f: formula of the neutral; r: radical; ee: possible from an even-electron ion in CID (ESI); note: where it is typical
  const LOSSES = [
    { f: "H", r: 1, note: "M−1: aldeidi, ammine, alcoli (scissione α con perdita di H•)" },
    { f: "CH3", r: 1, note: "metile: ramificazioni, metilchetoni, metossi aromatici, gruppi tert-butile" },
    { f: "NH3", ee: 1, note: "ammine primarie protonate e ammidi (ESI)" },
    { f: "OH", r: 1, note: "acidi carbossilici aromatici, nitro aromatici (effetto orto)" },
    { f: "H2O", ee: 1, note: "alcoli, acidi; in ESI quasi ogni –OH alifatico protonato" },
    { f: "HF", ee: 1, note: "composti fluorurati" },
    { f: "C2H2", ee: 1, note: "aromatici: 91 → 65, 77 → 51" },
    { f: "HCN", ee: 1, note: "piridine, aniline, nitrili aromatici, eterocicli azotati" },
    { f: "CO", ee: 1, note: "fenoli, chinoni, benzoile 105 → 77, acilio → alchile" },
    { f: "C2H4", ee: 1, note: "esteri etilici, McLafferty, retro-Diels-Alder, catene alchiliche" },
    { f: "N2", ee: 1, note: "rara: sali di diazonio, azidi, azocomposti" },
    { f: "CHO", r: 1, note: "aldeidi (M−29), fenoli (perdita di CO + H)" },
    { f: "C2H5", r: 1, note: "etile: scissione α di chetoni, ammine, eteri" },
    { f: "CH2O", ee: 1, note: "metossi aromatici, alcoli benzilici" },
    { f: "NO", r: 1, note: "nitro aromatici (Ar–NO<sub>2</sub> → Ar–O<sup>+</sup>)" },
    { f: "CH3O", r: 1, note: "esteri metilici (M−31), eteri metilici" },
    { f: "CH4O", ee: 1, note: "esteri metilici in ESI, metossi" },
    { f: "Cl", r: 1, note: "cloroalcani e cloroaromatici (EI)" },
    { f: "HCl", ee: 1, note: "cloroderivati alifatici, anche in ESI" },
    { f: "C3H6", ee: 1, note: "isopropile su N o O (ESI), McLafferty con catena propilica" },
    { f: "C2H2O", ee: 1, note: "chetene: acetati di fenile, acetammidi (N-acetil)" },
    { f: "C3H7", r: 1, note: "propile o isopropile: scissione α" },
    { f: "C2H3O", r: 1, note: "acetile: metilchetoni (M−43)" },
    { f: "HNCO", ee: 1, note: "acido isocianico: ureee e carbammati (ESI)" },
    { f: "CO2", ee: 1, note: "acidi carbossilici in negativo ([M−H−CO<sub>2</sub>]<sup>−</sup>), lattoni, esteri" },
    { f: "C2H4O", ee: 1, note: "acetaldeide: eteri e alcoli, glicoli" },
    { f: "NO2", r: 1, note: "nitro composti (M−46)" },
    { f: "CH2O2", ee: 1, note: "acido formico: acidi carbossilici in positivo (H<sub>2</sub>O + CO)" },
    { f: "C2H6O", ee: 1, note: "etanolo: esteri etilici, etossi" },
    { f: "C4H8", ee: 1, note: "isobutene: gruppi tert-butile (ESI), McLafferty con butile" },
    { f: "SO2", ee: 1, note: "solfoni e solfonammidi (ESI)" },
    { f: "Br", r: 1, note: "bromoderivati (EI)" },
    { f: "HBr", ee: 1, note: "bromoderivati alifatici" },
  ].map(L => { const p = PAL.parse(L.f); return { ...L, p, nom: PAL.nominal(p), ex: PAL.exact(p) }; });

  function perdite(root) {
    const fr = frame(root, `<label>Sorgente <select class="src"><option value="">EI e ESI, mescolate</option><option value="ei">EI (M<sup>+•</sup>)</option><option value="esi">ESI-CID ([M+H]<sup>+</sup>)</option></select></label>
      <label>Risoluzione <select class="res"><option value="lr">unitaria (quadrupolo)</option><option value="hr">alta (massa esatta)</option></select></label>`, next);
    function next() {
      const srcSel = fr.bar.querySelector(".src").value, hr = fr.bar.querySelector(".res").value === "hr";
      const src = srcSel || (Math.random() < 0.5 ? "ei" : "esi");
      const pool = LOSSES.filter(L => src === "ei" || L.ee);
      const items = pool.map(L => ({ id: L.f + "-" + src + (hr ? "-hr" : ""), d0: L.r ? 0.3 : 0, skills: hr ? ["perdite", "hr"] : ["perdite"], L }));
      const it = PAL.pick("perdite", items), L = it.L;
      // precursor: the parity follows the nitrogen rule of an imaginary molecule (no N, or one N): it does not matter for the loss
      const P = rnd(140, 380), Fm = P - L.nom;
      const same = pool.filter(x => x.nom === L.nom);
      const near = pool.filter(x => x.nom !== L.nom && Math.abs(x.nom - L.nom) <= 2);
      const opts = PAL.shuffle([...same, ...PAL.shuffle(near).slice(0, Math.max(2, 5 - same.length))]);
      const dEx = L.ex + (Math.random() - 0.5) * 0.0012;           // measured difference, ±0.6 mDa
      const ion = src === "ei" ? `M<sup>+•</sup> m/z ${P} (EI, 70 eV)` : `[M+H]<sup>+</sup> m/z ${P} (ESI, product ion scan)`;
      fr.wk.innerHTML = `<h3>Che cosa ha perso?</h3>
        <p>Precursore: <b>${ion}</b> → frammento a <b>m/z ${Fm}</b>${hr ? `. Differenza misurata in alta risoluzione: <b>${fx(dEx, 4)} u</b>` : ""}.</p>
        <div class="st"><h4>1. Quale neutro è stato perso? ${hr ? "(una sola risposta)" : "(a risoluzione unitaria possono essere giuste più risposte: sceglietene una)"}</h4>
        <div class="opts">${opts.map((x, i) => `<button data-i="${i}">${sub(x.f)}${x.r ? "<sup>•</sup>" : ""} <small>(${hr ? fx(x.ex, 4) : x.nom})</small></button>`).join("")}</div><div class="fb"></div></div>
        <div class="st off"><h4>2. Il frammento è uno ione a elettroni pari o dispari?</h4>
        <div class="opts"><button data-e="ee">pari (catione)</button><button data-e="oe">dispari (radicale catione)</button></div><div class="fb"></div></div>`;
      const st = fr.wk.querySelectorAll(".st");
      let s1 = 0;
      st[0].querySelectorAll("button").forEach(b => b.onclick = () => {
        const x = opts[+b.dataset.i];
        const ok = hr ? x === L || Math.abs(x.ex - dEx) < 0.0015 : x.nom === L.nom;
        b.classList.add(ok ? "right" : "wrong");
        st[0].querySelectorAll("button").forEach(y => y.disabled = true);
        s1 = ok ? 1 : 0;
        const amb = same.length > 1 ? `A massa nominale ${L.nom} corrispondono ${same.map(y => sub(y.f) + (y.r ? "<sup>•</sup>" : "") + " (" + fx(y.ex, 4) + ")").join(", ")}: ${hr ? "la massa esatta le distingue (la differenza è di qualche decina di mDa, molto più dell'errore di misura)" : "a risoluzione unitaria non si distinguono; servono l'alta risoluzione, gli isotopi o il resto dello spettro"}.` : "";
        const fb = st[0].querySelector(".fb");
        fb.className = "fb " + (ok ? "ok" : "no");
        fb.innerHTML = `${ok ? "Giusto." : `No: ${hr ? `la differenza ${fx(dEx, 4)} corrisponde a ${sub(L.f)}${L.r ? "<sup>•</sup>" : ""} (${fx(L.ex, 4)})` : `la differenza è ${L.nom}`}.`} ${amb} <br><b>${sub(L.f)}${L.r ? "<sup>•</sup>" : ""}</b>: ${L.note}.`;
        st[1].classList.remove("off");
      });
      // electron parity of the product: EI M+. minus a radical -> even; minus a molecule -> odd. ESI [M+H]+ minus a molecule -> even
      const want = src === "ei" ? (L.r ? "ee" : "oe") : "ee";
      st[1].querySelectorAll("button").forEach(b => b.onclick = () => {
        const ok = b.dataset.e === want;
        b.classList.add(ok ? "right" : "wrong");
        st[1].querySelectorAll("button").forEach(y => y.disabled = true);
        const fb = st[1].querySelector(".fb"); fb.className = "fb " + (ok ? "ok" : "no");
        fb.innerHTML = (ok ? "Giusto. " : "No. ") + (src === "ei"
          ? (L.r ? "M<sup>+•</sup> ha un elettrone spaiato; se se ne va con un radicale, resta un catione a elettroni pari (scissione semplice)." : "M<sup>+•</sup> perde una molecola neutra intera: l'elettrone spaiato resta nello ione, che rimane a elettroni dispari (riarrangiamento: McLafferty, perdita di H<sub>2</sub>O, CO, ...). Uno ione a elettroni dispari fra i frammenti segnala spesso un riarrangiamento.")
          : "[M+H]<sup>+</sup> è a elettroni pari e in CID perde quasi sempre molecole neutre (regola degli elettroni pari): il frammento resta a elettroni pari.");
        const s = (s1 + (ok ? 1 : 0)) / 2;
        PAL.record("perdite", it.id, it.d0, Object.fromEntries(it.skills.map(k => [k, s])), s * 100);
        fr.pts("perdite");
      });
    }
    next(); fr.pts("perdite");
  }

  // =========================================================== 2. isotope clusters
  const HAL = [[0, 0], [0, 0], [1, 0], [2, 0], [3, 0], [4, 0], [0, 1], [0, 2], [0, 3], [1, 1], [2, 1], [1, 2]];
  function isotopi(root) {
    const fr = frame(root, `<label>Livello <select class="lv"><option value="1">1 · solo Cl e Br</option><option value="2" selected>2 · Cl, Br, S e carboni</option></select></label>`, next);
    let c = null;
    function next() {
      const lv = +fr.bar.querySelector(".lv").value;
      const [cl, br] = pickOne(HAL), s = lv >= 2 && Math.random() < 0.3 ? rnd(1, 2) : 0;
      const C = rnd(4, 18), x = cl + br, rdbv = rnd(0, Math.min(6, C)), H = 2 * C + 2 - x - 2 * rdbv;
      if (H < 1) return next();
      const f = { C, H }; if (cl) f.Cl = cl; if (br) f.Br = br; if (s) f.S = s;
      const M = PAL.nominal(f), pat = PAL.isoPattern(f, 9), top = Math.max(...pat);
      const rel = pat.map(v => v / top * 100);
      const id = `${cl}Cl${br}Br${s}S`;
      fr.wk.innerHTML = `<h3>Indovina gli elementi</h3><p>Il gruppo dello ione molecolare di una molecola (risoluzione unitaria). Il primo picco è M (m/z ${M}).</p>
        <div class="cv"></div>
        <table class="eitab"><tr><th>m/z</th>${rel.map((_, i) => `<td class="n">${M + i}</td>`).join("")}</tr><tr><th>%</th>${rel.map(v => `<td class="n">${fx(v, 1)}</td>`).join("")}</tr></table>
        <div class="row"><label>Cl <select class="a-cl">${[0, 1, 2, 3, 4].map(i => `<option>${i}</option>`).join("")}</select></label>
        <label>Br <select class="a-br">${[0, 1, 2, 3].map(i => `<option>${i}</option>`).join("")}</select></label>
        ${lv >= 2 ? `<label>S <select class="a-s">${[0, 1, 2].map(i => `<option>${i}</option>`).join("")}</select></label><label>carboni <input class="a-c" size="3" inputmode="numeric"></label>` : ""}
        <button class="pri ck">Controlla</button></div><div class="fb"></div>
        <details class="q"><summary>Promemoria</summary><p>M+2 ≈ 32% di M per ogni Cl, ≈ 97% per ogni Br, 4,4% per ogni S. Due Cl: 100 : 64 : 10; due Br: 51 : 100 : 49; Cl + Br: 77 : 100 : 24. Il numero di carboni da M+1: n<sub>C</sub> ≈ (M+1)/(M) × 100 / 1,1 (lo zolfo aggiunge 0,8% per atomo a M+1). Quando M+2 è più alto di M, calcolate i rapporti rispetto a M, non al picco più alto.</p></details>`;
      const cv = TP.canvas(fr.wk.querySelector(".cv"), 200);
      const draw = () => { const ax = TP.axes(cv, { x0: M - 1.5, x1: M + rel.length + 0.5, y0: 0, y1: 112, xl: "m/z", yl: "%", xticks: rel.map((_, i) => M + i), yfmt: v => v <= 100 ? v : "" }); TP.sticks(ax, rel.map((_, i) => M + i), rel, "#24231f", 5); };
      cv.onresize = draw; draw();
      const r1 = pat[1] / pat[0] * 100;
      c = { f, cl, br, s, C, r1 };
      fr.wk.querySelector(".ck").onclick = () => {
        const g = q => { const e = fr.wk.querySelector(q); return e ? e.value : null; };
        const okCl = +g(".a-cl") === cl, okBr = +g(".a-br") === br;
        const parts = [okCl, okBr];
        let msg = `Cl: ${cl} ${okCl ? "✓" : "✗"} · Br: ${br} ${okBr ? "✓" : "✗"}`;
        if (lv >= 2) {
          const okS = +g(".a-s") === s, cg = parseInt(g(".a-c"), 10), okC = Math.abs(cg - C) <= (C > 12 ? 2 : 1);
          parts.push(okS, okC);
          msg += ` · S: ${s} ${okS ? "✓" : "✗"} · carboni: ${C} ${okC ? "✓" : "✗"} (da M+1: ${fx(r1, 1)}% / 1,1 ≈ ${fx(r1 / 1.1, 1)}${s ? `, meno il contributo di <sup>33</sup>S` : ""})`;
        }
        const sc = parts.filter(Boolean).length / parts.length;
        const fb = fr.wk.querySelector(".fb"); fb.className = "fb " + (sc === 1 ? "ok" : sc >= 0.5 ? "hi" : "no");
        fb.innerHTML = `${msg}. La formula era ${PAL.fhtml(f)} (M = ${M}).`;
        fr.wk.querySelector(".ck").disabled = true;
        const scores = { iso: (+okCl + +okBr) / 2 }; if (lv >= 2) scores.formula = parts[3] ? 1 : 0;
        PAL.record("isotopi", id, cl + br > 2 ? 0.5 : 0, scores, sc * 100);
        fr.pts("isotopi");
      };
    }
    next(); fr.pts("isotopi");
  }

  // =========================================================== 3. exact mass -> formula
  const EXM = { C: 12, H: 1.007825032, N: 14.003074005, O: 15.994914622, S: 31.97207069, Cl: 34.968852707 };
  /** All ion formulas (CHNOSCl, cation: electron removed) within tol ppm of m. */
  function enumerate(m, tol) {
    const out = [], w = m * tol * 1e-6;
    for (let c = 1; c <= 34; c++) for (let n = 0; n <= 5; n++) for (let o = 0; o <= 8; o++) for (let s = 0; s <= 1; s++) for (let cl = 0; cl <= 2; cl++) {
      const rest = c * EXM.C + n * EXM.N + o * EXM.O + s * EXM.S + cl * EXM.Cl - EM;
      const h = Math.round((m - rest) / EXM.H);
      if (h < 1 || h > 2 * c + n + 3) continue;
      const mz = rest + h * EXM.H;
      if (Math.abs(mz - m) > w) continue;
      const f = { C: c, H: h }; if (n) f.N = n; if (o) f.O = o; if (s) f.S = s; if (cl) f.Cl = cl;
      out.push({ f, mz, ppm: (m - mz) / mz * 1e6, rdb: PAL.rdb(f) });
    }
    return out;
  }
  function formula(root) {
    const fr = frame(root, `<label>Livello <select class="lv"><option value="1">1 · errori in ppm già calcolati</option><option value="2" selected>2 · calcolate voi l'errore</option><option value="3">3 · formula da uno ione non-target</option><option value="4">4 · isotopi fini (S e N)</option></select></label>`, next);
    function gen() {
      for (let k = 0; k < 200; k++) {
        const C = rnd(6, 22), N = Math.random() < 0.3 ? 0 : rnd(1, 4), O = rnd(0, 6), S = Math.random() < 0.15 ? 1 : 0, Cl = Math.random() < 0.2 ? rnd(1, 2) : 0;
        const r = rnd(1, 11), H = 2 * C + 2 + N - Cl - 2 * r;
        if (H < 2 || H / C < 0.4 || H / C > 2.3) continue;
        const f = { C, H }; if (N) f.N = N; if (O) f.O = O; if (S) f.S = S; if (Cl) f.Cl = Cl;
        const M = Object.entries(f).reduce((a, [e, n]) => a + EXM[e] * n, 0);
        if (M < 150 || M > 480) continue;
        return f;
      }
      return { C: 10, H: 13, N: 1, O: 2 };
    }
    function next() {
      const lv = +fr.bar.querySelector(".lv").value;
      if (lv >= 3) { HRNTUI.render(fr, lv); return; }
      const f = gen(), ionf = { ...f, H: f.H + 1 };
      const M = Object.entries(f).reduce((a, [e, n]) => a + EXM[e] * n, 0), mzT = M + H1;
      const err = (Math.random() * 2 - 1) * 2.5, mz = mzT * (1 + err * 1e-6);
      const pat = PAL.isoPattern(ionf, 4), noise = v => Math.max(0, v * (1 + (Math.random() - 0.5) * 0.08) + (Math.random() - 0.5) * 0.4);
      const obs = [100, noise(pat[1]), noise(pat[2])];
      let cand = enumerate(mz, 12);
      const key = g => PAL.fstr(g);
      if (!cand.some(x => key(x.f) === key(ionf))) cand.push({ f: ionf, mz: mzT, ppm: (mz - mzT) / mzT * 1e6, rdb: PAL.rdb(ionf) });
      cand.sort((a, b) => Math.abs(a.ppm) - Math.abs(b.ppm));
      const right = cand.find(x => key(x.f) === key(ionf));
      cand = PAL.shuffle([right, ...cand.filter(x => x !== right).slice(0, 5)]).sort((a, b) => a.mz - b.mz);
      // a candidate is "consistent" if it passes every test a student can do: error within 5 ppm, even-electron ion, isotopes within tolerance
      const mism = x => { const p = PAL.isoPattern(x.f, 4); return (Math.abs(p[1] - obs[1]) + Math.abs(p[2] - obs[2])) / 2; };
      cand.forEach(x => { x.mis = mism(x); x.ok = Math.abs(x.ppm) <= 5 && x.rdb % 1 !== 0 && x.rdb >= -0.5 && x.mis < 2.5; });
      right.ok = true;
      fr.wk.innerHTML = `<h3>Formula esatta</h3><p>Uno ione [M+H]<sup>+</sup> misurato in alta risoluzione (accuratezza dello strumento ±5 ppm):</p>
        <p style="font-size:19px"><b>m/z ${fx(mz, 4)}</b> &nbsp; con M+1 = <b>${fx(obs[1], 1)}%</b> e M+2 = <b>${fx(obs[2], 1)}%</b> di M</p>
        <p>Il software propone queste formule dello ione. Quale è quella giusta?</p>
        <table><tr><th></th><th>Formula dello ione</th><th>m/z teorica</th>${lv === 1 ? "<th>errore (ppm)</th>" : ""}</tr>
        ${cand.map((x, i) => `<tr><td><input type="radio" name="fc" value="${i}" aria-label="Candidato ${i + 1}: ${PAL.fstr(x.f)}, m/z ${fx(x.mz, 4)}"></td><td>${PAL.fhtml(x.f)}<sup>+</sup></td><td class="n">${fx(x.mz, 4)}</td>${lv === 1 ? `<td class="n">${fx(x.ppm, 1)}</td>` : ""}</tr>`).join("")}</table>
        <div class="row"><button class="pri ck">Controlla</button></div><div class="fb"></div>
        <details class="q"><summary>Come si sceglie</summary><ol><li>Errore: (misurata − teorica)/teorica × 10<sup>6</sup>: entro l'accuratezza dello strumento.</li><li>Uno ione [M+H]<sup>+</sup> è a elettroni pari: RDB semintero (…,5). Un RDB intero indica un radicale catione: in ESI quasi mai.</li><li>Isotopi: M+1 ≈ 1,1% per C (+0,37% per N, +0,8% per S); M+2 ≈ 32% per Cl, 4,4% per S, ≈ (1,1 n<sub>C</sub>)<sup>2</sup>/200 + 0,2% per O.</li><li>Rimasti più candidati? Regola dell'azoto sulla molecola neutra, rapporto H/C sensato, MS2.</li></ol></details>`;
      fr.wk.querySelector(".ck").onclick = () => {
        const sel = fr.wk.querySelector("input[name=fc]:checked");
        if (!sel) return;
        const x = cand[+sel.value], good = x === right || x.ok;
        fr.wk.querySelector(".ck").disabled = true;
        const fb = fr.wk.querySelector(".fb"); fb.className = "fb " + (good ? "ok" : "no");
        fb.innerHTML = `${x === right ? "Giusto." : good ? `Accettabile: anche ${PAL.fhtml(x.f)}<sup>+</sup> supera tutti i controlli; la formula usata per generare il problema era ${PAL.fhtml(right.f)}<sup>+</sup>. Per decidere servirebbero la MS2 o una misura più accurata.` : `No: la formula giusta è ${PAL.fhtml(right.f)}<sup>+</sup> (molecola neutra ${PAL.fhtml(f)}).`}
          <table><tr><th>Formula</th><th>errore ppm</th><th>RDB</th><th>M+1, M+2 attesi (%)</th><th>verdetto</th></tr>${cand.map(y => { const p = PAL.isoPattern(y.f, 4); const why = Math.abs(y.ppm) > 5 ? "errore troppo grande" : y.rdb % 1 === 0 ? "RDB intero: radicale" : y.mis >= 2.5 ? "isotopi non tornano" : "compatibile"; return `<tr${y === right ? ' style="font-weight:700"' : ""}><td>${PAL.fhtml(y.f)}<sup>+</sup></td><td class="n">${fx(y.ppm, 1)}</td><td class="n">${fx(y.rdb, 1)}</td><td class="n">${fx(p[1], 1)} · ${fx(p[2], 1)}</td><td>${why}</td></tr>`; }).join("")}</table>`;
        PAL.record("formula", "f" + (lv), 0.3, { hr: good ? 1 : 0, formula: good ? 1 : 0 }, good ? 100 : 0);
        fr.pts("formula");
      };
    }
    next(); fr.pts("formula");
  }

  // =========================================================== 4. instrument cards
  const SLOTS = {
    sep: ["Separazione", { GC: "GC", LC: "LC (HPLC)", inf: "nessuna (infusione diretta)" }],
    src: ["Sorgente", { EI: "EI", CI: "CI", ESI: "ESI", APCI: "APCI" }],
    an: ["Analizzatore", { Q: "quadrupolo singolo", QqQ: "triplo quadrupolo", IT: "trappola ionica", TOF: "Q-TOF", OT: "Orbitrap" }],
    acq: ["Acquisizione", { FS: "full scan", SIM: "SIM", MRM: "MRM", PIS: "product ion scan / MS<sup>n</sup>", PREC: "precursor ion scan", NL: "neutral loss scan", DDA: "full scan + MS2 automatica (DDA)" }],
  };
  const CASES = [
    { id: "erbicida", t: "Quantificare a livello di ng/L un erbicida polare e acido in acque superficiali, con un metodo di routine da applicare a centinaia di campioni.", a: { sep: ["LC"], src: ["ESI"], an: ["QqQ"], acq: ["MRM"] },
      why: "Polare e acido: LC in fase inversa ed ESI (in negativo). Per quantificare tracce in matrice complessa la combinazione più sensibile e selettiva è il triplo quadrupolo in MRM, con due transizioni (quantificatore e qualificatore)." },
    { id: "solvente", t: "Identificare un solvente sconosciuto che contamina un campione di aria, confrontandone lo spettro con una libreria.", a: { sep: ["GC"], src: ["EI"], an: ["Q", "QqQ", "IT", "TOF", "OT"], acq: ["FS"] },
      why: "Volatile e apolare: GC. Il confronto con le librerie richiede spettri EI a 70 eV acquisiti in full scan; qualsiasi analizzatore va bene, il più comune è il quadrupolo singolo." },
    { id: "ci", t: "In GC-MS lo spettro EI di un alcol ramificato non mostra lo ione molecolare: volete confermarne la massa molecolare.", a: { sep: ["GC"], src: ["CI"], an: ["Q", "QqQ", "IT", "TOF", "OT"], acq: ["FS"] },
      why: "La ionizzazione chimica (metano, isobutano o ammoniaca) trasferisce poca energia e dà [M+H]<sup>+</sup> o addotti: la massa molecolare si legge nel full scan." },
    { id: "tp", t: "Trovare i prodotti di trasformazione sconosciuti di un farmaco dopo la fotocatalisi e proporne la formula molecolare e la struttura.", a: { sep: ["LC"], src: ["ESI"], an: ["TOF", "OT"], acq: ["DDA", "FS"] },
      why: "I TP sono in genere più polari del progenitore: LC-ESI. Per la formula serve la massa esatta (Q-TOF, Orbitrap); il full scan con MS2 automatica dà insieme masse esatte e frammenti, e si può rianalizzare a posteriori." },
    { id: "nl", t: "Con un triplo quadrupolo, trovare in un estratto tutti gli acidi carbossilici (che in negativo perdono CO<sub>2</sub>), senza sapere quali sono.", a: { sep: ["LC"], src: ["ESI"], an: ["QqQ"], acq: ["NL"] },
      why: "Neutral loss scan di 44: Q1 e Q3 scandiscono insieme con una differenza costante, e si vede solo chi perde quel neutro." },
    { id: "prec", t: "Con un triplo quadrupolo, trovare tutti i composti di una classe che in CID danno lo stesso ione caratteristico (per esempio m/z 77 dei derivati fenilici).", a: { sep: ["LC", "inf"], src: ["ESI", "APCI"], an: ["QqQ"], acq: ["PREC"] },
      why: "Precursor ion scan: Q3 fermo sullo ione caratteristico, Q1 scandisce. Ogni segnale è un precursore che produce quel frammento." },
    { id: "ipa", t: "Quantificare idrocarburi policiclici aromatici (IPA) in un suolo: apolari, stabili, con punti di ebollizione alti.", a: { sep: ["GC"], src: ["EI"], an: ["Q", "QqQ"], acq: ["SIM", "MRM"] },
      why: "Apolari e termicamente stabili: GC-EI, dove gli IPA danno M<sup>+•</sup> intensissimo. Per quantificare: SIM con un quadrupolo singolo o MRM con un triplo quadrupolo (più selettivo in matrici sporche). L'ESI li ionizza male." },
    { id: "apci", t: "Quantificare in LC uno steroide neutro poco polare, che in elettrospray dà un segnale debole e molto soppresso dalla matrice.", a: { sep: ["LC"], src: ["APCI"], an: ["QqQ"], acq: ["MRM"] },
      why: "Molecola poco polare e senza gruppi acidi o basici: l'APCI la ionizza in fase gas ed è meno sensibile all'effetto matrice. Per quantificare: triplo quadrupolo in MRM." },
    { id: "proteina", t: "Misurare la massa di una proteina intatta di 25 kDa.", a: { sep: ["LC", "inf"], src: ["ESI"], an: ["TOF", "OT"], acq: ["FS"] },
      why: "L'ESI dà ioni multicarica [M+zH]<sup>z+</sup> che cadono in un intervallo di m/z misurabile; dalla serie di cariche si ricava la massa (deconvoluzione). Serve risoluzione per separare gli stati di carica e gli isotopi: Q-TOF o Orbitrap." },
    { id: "msn", t: "Capire la struttura di un frammento frammentandolo a sua volta (MS<sup>3</sup>).", a: { sep: ["LC", "inf"], src: ["ESI", "APCI"], an: ["IT", "OT"], acq: ["PIS"] },
      why: "La trappola ionica isola e frammenta in sequenza gli stessi ioni (MS<sup>n</sup>); anche gli ibridi con trappola e Orbitrap lo fanno. Il triplo quadrupolo arriva solo a MS2 (il QTRAP sfrutta Q3 come trappola per l'MS3)." },
    { id: "pesticidi", t: "Screening di 300 pesticidi noti in estratti di frutta, con limiti di legge bassi e tempi di analisi brevi.", a: { sep: ["LC"], src: ["ESI"], an: ["QqQ"], acq: ["MRM"] },
      why: "Composti noti, molti e a basse concentrazioni: MRM programmato (ogni transizione è acquisita solo nella sua finestra di tempo di ritenzione, così restano abbastanza punti per picco)." },
    { id: "aromi", t: "Misurare composti odorosi volatili presenti in tracce in un vino.", a: { sep: ["GC"], src: ["EI"], an: ["Q", "QqQ"], acq: ["SIM", "MRM"] },
      why: "Volatili: GC (spesso con spazio di testa o microestrazione). In tracce: SIM sugli ioni caratteristici o MRM per più selettività." },
  ];
  function strumento(root) {
    const fr = frame(root, "", next);
    function next() {
      const it = PAL.pick("strumento", CASES.map(c => ({ id: c.id, d0: 0, skills: ["strumento"], c })));
      const c = it.c, ch = {};
      fr.wk.innerHTML = `<h3>Il problema</h3><p style="font-size:17px">${c.t}</p>` +
        Object.entries(SLOTS).map(([k, [name, o]]) => `<div class="st"><h4>${name}</h4><div class="cards2" role="group" aria-label="${name}" data-k="${k}">${Object.entries(o).map(([v, l]) => `<button data-v="${v}">${l}</button>`).join("")}</div></div>`).join("") +
        `<div class="row"><button class="pri ck">Controlla</button></div><div class="fb"></div>`;
      fr.wk.querySelectorAll(".cards2").forEach(g => g.querySelectorAll("button").forEach(b => b.onclick = () => {
        g.querySelectorAll("button").forEach(y => { y.classList.toggle("on", y === b); y.setAttribute("aria-pressed", String(y === b)); }); ch[g.dataset.k] = b.dataset.v;
      }));
      fr.wk.querySelector(".ck").onclick = () => {
        if (Object.keys(SLOTS).some(k => !ch[k])) { const fb = fr.wk.querySelector(".fb"); fb.className = "fb hi"; fb.textContent = "Scegliete una carta per ogni riga."; return; }
        let ok = 0;
        Object.keys(SLOTS).forEach(k => {
          const good = c.a[k].includes(ch[k]); ok += good;
          fr.wk.querySelectorAll(`.cards2[data-k=${k}] button`).forEach(b => { b.disabled = true; if (c.a[k].includes(b.dataset.v)) b.classList.add("on"); if (b.dataset.v === ch[k] && !good) b.style.borderColor = "var(--bad)"; });
        });
        const s = ok / 4;
        const fb = fr.wk.querySelector(".fb"); fb.className = "fb " + (s === 1 ? "ok" : s >= 0.5 ? "hi" : "no");
        fb.innerHTML = `${ok} su 4. Evidenziate le carte giuste. ${c.why}`;
        fr.wk.querySelector(".ck").disabled = true;
        PAL.record("strumento", c.id, 0, { strumento: s }, s * 100);
        fr.pts("strumento");
      };
    }
    next(); fr.pts("strumento");
  }

  // =========================================================== 5. drive the quadrupole
  // virtual instrument as in chapter 9: r0 = 4 mm, f = 1 MHz -> q = KQ * V / (m/z), a = 2 KQ * U / (m/z)
  const E = 1.602176634e-19, U_KG = 1.66053907e-27, R0 = 4e-3, OM = 2 * Math.PI * 1e6;
  const KQ = 4 * E / (U_KG * R0 * R0 * OM * OM);
  const aTop = q => Math.min(q * q / 2 - 7 * q ** 4 / 128 + 29 * q ** 6 / 2304,            // -a0(q): y-stability boundary
    1 - q - q * q / 8 + q ** 3 / 64 - q ** 4 / 1536 - 11 * q ** 5 / 36864);                 // b1(q): x-stability boundary
  const stable = (a, q) => q > 0 && q < 0.908 && a >= 0 && a <= aTop(q);
  /** Stable window on the scan line a = 2 lam q: [q1, q2] or null. */
  function windowQ(lam) {
    let q1 = null, q2 = null;
    for (let q = 0.0001; q < 0.908; q += 0.00002) { const ok = 2 * lam * q <= aTop(q); if (ok && q1 === null) q1 = q; if (ok) q2 = q; else if (q1 !== null) break; }
    return q1 === null ? null : [q1, q2];
  }
  function quadrupolo(root) {
    const fr = frame(root, `<label>Livello <select class="lv"><option value="10">1 · vicini a ±10</option><option value="3">2 · vicini a ±3</option><option value="1" selected>3 · vicini a ±1 (risoluzione unitaria)</option><option value="0.5">4 · vicini a ±0,5</option></select></label>`, next);
    let g = null;
    function next() {
      const sp = +fr.bar.querySelector(".lv").value, m = rnd(8, 60) * 10 + (sp < 1 ? 0 : rnd(0, 9));
      g = { m, sp, ions: [m - sp, m, m + sp], V: Math.round(0.5 * m / KQ), lam: 0.12, done: false };
      fr.wk.innerHTML = `<h3>Fate passare m/z ${m}, fermate m/z ${fx(m - sp, sp < 1 ? 1 : 0)} e ${fx(m + sp, sp < 1 ? 1 : 0)}</h3>
        <p>Scegliete l'ampiezza della radiofrequenza <i>V</i> e il rapporto <i>U</i>/<i>V</i>. Lo ione bersaglio (rosso) deve stare dentro la regione di stabilità, i due vicini (grigi) fuori. Strumento: r<sub>0</sub> = 4 mm, f = 1 MHz.</p>
        <div class="duo"><div class="cv1"></div><div class="cv2"></div></div>
        <div class="row"><label>V (V) <input type="range" class="sV" min="0" max="${Math.round(0.95 * (m + sp) / KQ)}" step="0.01" value="${g.V}" style="width:220px"> <input class="nV" size="8" value="${g.V}"></label></div>
        <div class="row"><label>U/V <input type="range" class="sL" min="0" max="0.168" step="0.0001" value="${g.lam}" style="width:220px"> <input class="nL" size="8" value="${g.lam}"></label> <span class="mono uu"></span></div>
        <div class="fb st0"></div><div class="row"><button class="pri ck">Conferma</button></div><div class="fb res"></div>
        <details class="q"><summary>Suggerimenti</summary><p>Lo ione bersaglio è al vertice della regione stabile se q = 0,706, cioè V = 0,706 (m/z)/K con K = ${fx(KQ, 4)} per volt (m/z in u): per m/z ${m} circa ${fx(0.706 * m / KQ, 1)} V. Poi alzate U/V verso 0,168 finché i vicini escono: più la retta si avvicina al vertice, più la finestra si stringe. Una finestra più larga del necessario però trasmette più ioni: il punteggio premia la finestra più larga che esclude ancora i vicini.</p></details>`;
      const c1 = TP.canvas(fr.wk.querySelector(".cv1"), 260), c2 = TP.canvas(fr.wk.querySelector(".cv2"), 260);
      const $ = q => fr.wk.querySelector(q);
      const sync = (from) => {
        if (from === "sV") $(".nV").value = $(".sV").value; if (from === "nV") $(".sV").value = $(".nV").value;
        if (from === "sL") $(".nL").value = $(".sL").value; if (from === "nL") $(".sL").value = $(".nL").value;
        g.V = Math.max(0, +String($(".nV").value).replace(",", ".") || 0); g.lam = Math.min(0.1684, Math.max(0, +String($(".nL").value).replace(",", ".") || 0));
        draw();
      };
      ["sV", "nV", "sL", "nL"].forEach(k => { $("." + k).oninput = () => sync(k); });
      function pts() { return g.ions.map(mz => { const q = KQ * g.V / mz; return { mz, q, a: 2 * g.lam * q, ok: stable(2 * g.lam * q, q) }; }); }
      function region(ax, q0, q1) {
        const qs = [], as = []; for (let q = q0; q <= q1; q += (q1 - q0) / 400) { qs.push(q); as.push(Math.max(0, aTop(q))); }
        const { ctx, X, Y } = ax; ctx.save(); ctx.fillStyle = "rgba(14,116,144,.13)"; ctx.beginPath(); ctx.moveTo(X(qs[0]), Y(0));
        qs.forEach((q, i) => ctx.lineTo(X(q), Y(as[i]))); ctx.lineTo(X(qs[qs.length - 1]), Y(0)); ctx.closePath(); ctx.fill(); ctx.restore();
        TP.line(ax, qs, as, "#0e7490", 1.5);
      }
      function plot(c, q0, q1, a0, a1, P) {
        const ax = TP.axes(c, { x0: q0, x1: q1, y0: a0, y1: a1, xl: "q", yl: "a", m: { l: 60 } });
        ax.clip(); region(ax, q0, q1);
        TP.line(ax, [0, 1], [0, 2 * g.lam], "#b45309", 1.5, [5, 4]);
        P.forEach((p, i) => { const { ctx, X, Y } = ax; ctx.fillStyle = i === 1 ? "#c2410c" : "#57534e"; ctx.beginPath(); ctx.arc(X(p.q), Y(p.a), i === 1 ? 6 : 5, 0, 7); ctx.fill(); });
        ax.ctx.restore();
        return ax;
      }
      function draw() {
        const P = pts(), t = P[1];
        plot(c1, 0, 1, 0, 0.3, P);
        // zoom around the target ion (or the apex if the target is far away)
        const qc = Math.min(Math.max(t.q, 0.6), 0.85), aq = Math.max(0.002, Math.abs(P[2].q - P[0].q) * 1.3), ac = 2 * g.lam * qc;
        plot(c2, qc - aq, qc + aq, Math.max(0, ac - aq * 0.7), Math.max(ac + aq * 0.7, 0.01), P);
        $(".uu").innerHTML = `U = ${fx(g.lam * g.V, 2)} V`;
        const w = windowQ(g.lam);
        const win = w ? [KQ * g.V / w[1], KQ * g.V / w[0]] : null;
        g.win = win; g.P = P;
        const f = $(".st0"); f.className = "fb hi st0";
        f.innerHTML = P.map((p, i) => `m/z ${fx(p.mz, sp < 1 ? 1 : 0)}: q = ${fx(p.q, 4)}, a = ${fx(p.a, 4)} → <b>${p.ok ? "stabile" : "instabile"}</b>`).join("<br>") +
          (win && g.V > 0 ? `<br>Finestra trasmessa: m/z ${fx(win[0], 2)} – ${fx(win[1], 2)} (larga ${fx(win[1] - win[0], 2)})` : "<br>Nessuno ione è stabile su questa retta.");
      }
      draw();
      $(".ck").onclick = () => {
        const P = g.P, win = g.win, ok = P[1].ok && !P[0].ok && !P[2].ok;
        const width = win ? win[1] - win[0] : 0, bonus = ok ? Math.min(1, width / (2 * sp)) : 0;
        const s = ok ? 0.6 + 0.4 * bonus : 0;
        const fb = $(".res"); fb.className = "fb res " + (ok ? "ok" : "no");
        fb.innerHTML = ok ? `Riuscito: passa solo m/z ${m}. Finestra larga ${fx(width, 2)} su un massimo utile di ${fx(2 * sp, 1)}: ${bonus > 0.7 ? "ottima trasmissione" : "potreste allargarla un po' (U/V più basso) e trasmettere di più"}. Punteggio ${Math.round(s * 100)}.`
          : (!P[1].ok ? "Lo ione bersaglio non è stabile: avvicinate il suo q a 0,706 con V e controllate che la retta passi sotto il vertice." : "Passa anche un vicino: alzate U/V (la retta si avvicina al vertice e la finestra si stringe).");
        if (ok && !g.done) { g.done = true; PAL.record("quadrupolo", "s" + sp, sp >= 3 ? -0.5 : sp >= 1 ? 0.3 : 1, { quad: s }, s * 100); fr.pts("quadrupolo"); }
        else if (!ok && !g.done) { g.fails = (g.fails || 0) + 1; if (g.fails === 3) { g.done = true; PAL.record("quadrupolo", "s" + sp, 0, { quad: 0 }, 0); fr.pts("quadrupolo"); } }
      };
    }
    next(); fr.pts("quadrupolo");
  }

  return { perdite, isotopi, formula, strumento, quadrupolo, LOSSES, enumerate, aTop, stable, windowQ, KQ, CASES };
})();
if (typeof module !== "undefined") module.exports = GIOCHI;
