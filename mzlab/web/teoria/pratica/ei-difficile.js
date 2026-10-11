// QqQ lab - Pratica EI, "Modalità difficile": the student gets ONLY the spectrum (no formula, no class, no step order) and has to draw the
// structure. The program never says "right" or "wrong" before «Consegna e correggi»: it only reports what does not hold together
// (nominal mass against the chosen M, nitrogen rule, integer RDB >= 0, isotope ratios of the proposed formula, impossible losses,
// ions that are not sub-formulae of the proposed formula, electron parity against the proposed mechanism). Peak annotations appear only
// after the student's own hypothesis for that peak. Three levels of hints (zone of the spectrum, family of ions, zone of the skeleton;
// the third is in ei-aiuti.js) cost 5, 10 and 20 points and never name a group, a class or the compound. Globals: PAL, EISPEC, TP,
// EI_AIUTI. Everything else comes from gioco-ei.js through `env`.
export const SKILLNAME = { M: "ione molecolare", isotopi: "isotopi", azoto: "regola dell'azoto", rdb: "RDB e insaturazioni", alpha: "scissione α",
  induttiva: "scissione induttiva", sigma: "rottura σ", mclafferty: "McLafferty", rda: "retro-Diels-Alder", benzilica: "scissione benzilica e tropilio",
  orto: "effetto orto", serie: "serie omologa", perdita: "perdita neutra", riarr: "riarrangiamento", "m-assente": "ione molecolare assente" };
const ODD_MECH = ["mclafferty", "rda", "orto"], EVEN_MECH = ["alpha", "i", "sigma", "tropilio", "serie"];
const MECHS = ["alpha", "i", "sigma", "mclafferty", "rda", "tropilio", "orto", "perdita", "riarr", "serie"];
const HINT_COST = [5, 10, 20];
const imposs = d => (d >= 4 && d <= 14) || (d >= 21 && d <= 25);           // losses that cannot come from a molecular ion (chapter 15)
const fmt = v => String(v).replace(".", ",");

/** An ion typed by the student: "C3H7O+", "C4H8+." or "C4H8+•" (the charge is optional). */
function parseIon(txt) {
  const m = /^([A-Za-z0-9]+?)(\+\.|\+•|\+)?$/.exec(String(txt || "").trim().replace(/\s+/g, ""));
  if (!m) return null;
  const f = PAL.parse(m[1]);
  return f ? { f, charge: m[2] ? (m[2] === "+" ? "e" : "o") : null, text: m[1] + (m[2] ? m[2].replace("•", ".") : "") } : null;
}
const subFormula = (a, b) => Object.entries(a).every(([e, n]) => n <= (b[e] || 0) + (e === "H" ? 1 : 0));
const cssCol = n => getComputedStyle(document.documentElement).getPropertyValue("--c" + n).trim() || "currentColor";

/** Impossible losses and peaks above M for a chosen M: only reports, never says which M is right. */
export function checkM(it, M, vis) {
  const out = [];
  const above = it.peaks.filter(([m, v]) => m > M + 4 && v >= 5).map(p => p[0]);
  if (vis && above.length) out.push({ ok: false, t: `Sopra m/z ${M} ci sono picchi non isotopici (${above.slice(0, 6).join(", ")}${above.length > 6 ? "…" : ""}): lo ione molecolare non può stare sotto di loro.` });
  // a peak one unit above a much taller one is its 13C companion, not a loss from M
  const companion = m => peak(it, m - 1) >= 3.3 * peak(it, m);
  const bad = it.peaks.filter(([m, v]) => m < M && v >= 50 && imposs(M - m) && !companion(m)).map(p => p[0]);
  if (bad.length) out.push({ ok: false, t: `Dalla massa ${M} verso m/z ${bad.slice(0, 6).join(", ")} ci sono perdite di 4-14 o 21-25 unità: impossibili da uno ione molecolare. Rivedi M o l'origine di quei picchi.` });
  if (!out.length) out.push({ ok: true, t: `Nessuna perdita impossibile da m/z ${M} e nessun picco non isotopico sopra: M è coerente con lo spettro.` });
  out.push({ ok: null, t: `M = ${M} è ${M % 2 ? "dispari: la molecola ha un numero dispari di N (1, 3…)" : "pari: la molecola ha 0 o un numero pari di N"} (regola dell'azoto).` });
  return out;
}

/** Coherence of a proposed formula with the chosen M and the spectrum. */
export function checkFormula(it, g, M, vis) {
  const out = [];
  const nom = PAL.nominal(g), rdb = PAL.rdb(g), nN = g.N || 0;
  if (M == null) out.push({ ok: null, t: "Fissa prima l'ione molecolare: non posso confrontare la massa." });
  else if (nom !== M) out.push({ ok: false, t: `La massa nominale di ${PAL.fhtml(g)} è ${nom}, non ${M}.` });
  else out.push({ ok: true, t: `La massa nominale di ${PAL.fhtml(g)} è ${nom}, uguale a M.` });
  if (M != null && (M % 2) !== (nN % 2)) out.push({ ok: false, t: `Regola dell'azoto: M = ${M} è ${M % 2 ? "dispari" : "pari"}, ma la formula ha ${nN} N.` });
  if (rdb < 0 || rdb !== Math.floor(rdb)) out.push({ ok: false, t: `RDB = ${fmt(rdb)}: una molecola ha RDB intero e non negativo.` });
  else out.push({ ok: null, t: `RDB = ${rdb} (anelli + doppi legami; un anello aromatico vale 4).` });
  if (vis && M != null && peak(it, M)) {
    const pat = PAL.isoPattern(g), r1 = ratio(it, M, 1), r2 = ratio(it, M, 2);
    const tol = e => Math.max(1.2, 0.35 * e);
    if (r1 != null && pat[1] != null) out.push({ ok: Math.abs(r1 - pat[1]) <= tol(pat[1]), t: `M+1/M: nello spettro ${pct(r1)}, atteso per ${PAL.fhtml(g)} circa ${pct(pat[1])}.` });
    if (r2 != null && pat[2] != null) out.push({ ok: Math.abs(r2 - pat[2]) <= tol(pat[2]), t: `M+2/M: nello spettro ${pct(r2)}, atteso circa ${pct(pat[2])} (Cl, Br e S lo alzano).` });
    if (it.iso && !it.iso.ok) out.push({ ok: null, t: "Nota: in questo spettro il gruppo di M può non essere affidabile (M debole o M+1 gonfiato dall'autoprotonazione)." });
  } else if (!vis) out.push({ ok: null, t: "M non è nello spettro: nessun controllo isotopico su M; usa i gruppi isotopici dei frammenti." });
  return out;
}
const peak = (it, m) => { const p = it.peaks.find(q => q[0] === m); return p ? p[1] : 0; };
const ratio = (it, m, d) => { const a = peak(it, m); return a ? peak(it, m + d) / a * 100 : null; };
const pct = v => v == null ? "–" : v.toFixed(v < 10 ? 1 : 0).replace(".", ",") + "%";

/** Coherence of one peak hypothesis: ion formula, mass, sub-formula, electron parity against the mechanism, loss from M. */
export function checkIon(it, mz, ion, mech, g, M, LOSSNAME) {
  const out = [];
  if (ion) {
    const nom = PAL.nominal(ion.f), rdb = PAL.rdb(ion.f), odd = rdb === Math.floor(rdb);
    if (nom !== mz) out.push({ ok: false, t: `La massa nominale di ${PAL.fhtml(ion.f)} è ${nom}, il picco è a m/z ${mz}.` });
    if (rdb < 0) out.push({ ok: false, t: `RDB di ${PAL.fhtml(ion.f)} = ${fmt(rdb)}: negativo, ione impossibile.` });
    if (ion.charge && ion.charge !== (odd ? "o" : "e")) out.push({ ok: false, t: `Hai scritto uno ione a elettroni ${ion.charge === "o" ? "dispari (+•)" : "pari (+)"}, ma con ${PAL.fhtml(ion.f)} l'RDB (${fmt(rdb)}) dice che è a elettroni ${odd ? "dispari" : "pari"}.` });
    if (g && !subFormula(ion.f, g)) out.push({ ok: false, t: `${PAL.fhtml(ion.f)} contiene atomi che la formula ${PAL.fhtml(g)} non ha (è ammesso un solo H in più per un trasferimento).` });
    if (mech && mech !== "perdita" && mech !== "riarr") {
      if (ODD_MECH.includes(mech) && !odd) out.push({ ok: false, t: `${EISPEC.TYPE[mech]} dà ioni a elettroni dispari, e ${PAL.fhtml(ion.f)} è a elettroni pari.` });
      if (EVEN_MECH.includes(mech) && odd) out.push({ ok: false, t: `Una scissione semplice (${EISPEC.TYPE[mech]}) dà ioni a elettroni pari, e ${PAL.fhtml(ion.f)} è a elettroni dispari.` });
    }
    if (!out.length) out.push({ ok: true, t: `${PAL.fhtml(ion.f)}: massa, atomi e parità degli elettroni sono coerenti.` });
  }
  if (M != null) {
    const d = M - mz;
    if (d < 0) out.push({ ok: null, t: `m/z ${mz} è ${-d} unità sopra M: un picco isotopico?` });
    else if (imposs(d)) out.push({ ok: false, t: `Δm da M = ${d}: una perdita di ${d} unità non esiste (4-14 e 21-25 sono impossibili): il picco non nasce direttamente da M (a meno che sia il picco isotopico di un vicino).` });
    else if (d > 0) out.push({ ok: null, t: `Δm da M = ${d}${LOSSNAME[d] ? ` (perdite tipiche di ${d}: ${LOSSNAME[d]})` : ""}.` });
  }
  return out;
}

const li = c => `<li class="${c.ok === true ? "ok" : c.ok === false ? "no" : ""}">${c.ok === true ? "✓ " : c.ok === false ? "✗ " : "· "}${c.t}</li>`;
const list = cs => `<ul class="hnote">${cs.map(li).join("")}</ul>`;

/** Level 1 and 2 of the hints, built from the spectrum itself (no m/z of the answer is written, only a zone). */
export function hintTexts(it) {
  const big = it.peaks.filter(p => p[1] >= 20), top = Math.max(...big.map(p => p[0])), band = [Math.max(0, top - 6), top + 4];
  const keys = it.keys.filter(k => k.type !== "M" && k.type !== "isotopo");
  const oe = keys.filter(k => /\+\.$/.test(k.ion)).length;
  const strong = it.peaks.filter(p => p[1] >= 100).map(p => p[0]).sort((a, b) => a - b);
  let run = 1, best = 1;
  for (let i = 1; i < strong.length; i++) { run = strong[i] - strong[i - 1] === 14 ? run + 1 : 1; best = Math.max(best, run); }
  const f = PAL.parse(it.formula), iso = (f.Cl || f.Br || f.S);
  const l1 = `Osservazione generale: la zona evidenziata sullo spettro contiene i picchi di massa più alta con intensità apprezzabile. Per ognuno chiediti se è un picco isotopico (distanza 1, 2 o 4 dal vicino) o un frammento, e quante unità lo separano dagli altri: perdite di 4-14 e di 21-25 unità non esistono.`;
  const l2 = [
    oe ? `Fra i picchi intensi, oltre a M ci sono ${oe} ioni a elettroni dispari (RDB intero): richiedono due legami rotti, cioè un riarrangiamento con perdita di una molecola neutra.` :
      `I picchi intensi, a parte M, sono tutti ioni a elettroni pari (RDB semintero): nascono da scissioni semplici, con la perdita di un radicale.`,
    best >= 3 ? `Molti picchi stanno a intervalli regolari di 14 unità (CH₂): è una serie.` : "Non si vede una serie regolare a intervalli di 14: i picchi non vengono da una catena che si accorcia di CH₂.",
    iso ? `In alto il gruppo di picchi isotopici ha una forma insolita: confronta altezze e distanze con quelle attese per 1 o 2 atomi di Cl, Br o S.` : `In alto il gruppo isotopico è quello di C, H, N, O: M+1 ≈ 1,1% per carbonio, M+2 piccolo.`,
  ].join(" ");
  return { band, l1, l2, l3: (typeof EI_AIUTI !== "undefined" && EI_AIUTI[it.id]) || "Riguarda gli ioni a elettroni dispari e le perdite di molecole neutre: ti dicono in che parte dello scheletro cercare." };
}

/** The worksheet of the hard mode. env: see HARD_ENV in gioco-ei.js; it: the compound; g: the container of the game. */
export function startHard(env, it, g) {
  const { $, EH, fb, pay, LOSSNAME, FAMNAME } = env;
  const S = env.state();
  const H = S.hard_state = { M: null, Mvis: true, f: null, marks: [], hint: 0, band: null, delivered: false, hintTexts: hintTexts(it) };
  S.hardHints = 0;
  const noSol = "Nessuna risposta, nemmeno parziale, fino a «Consegna e correggi».";
  const onpick = sel => { const e = $("#h-pk"); if (e) e.textContent = sel.length ? `picco selezionato: m/z ${sel[sel.length - 1]}` : "nessun picco selezionato"; };
  S.spec = env.spectrumCard(g, it, {
    title: "Composto incognito", table: true, h: 320, onpick,
    mark: () => H.marks.map(m => m.mz),
    deco: ax => {
      const { ctx, X, Y } = ax;
      if (H.band) { ctx.save(); ctx.globalAlpha = 0.16; ctx.fillStyle = cssCol(2); ctx.fillRect(X(H.band[0]), ax.m.t, X(H.band[1]) - X(H.band[0]), ax.H - ax.m.t - ax.m.b); ctx.restore(); }
      ctx.save(); ctx.font = "bold 12px system-ui,sans-serif"; ctx.textAlign = "center"; ctx.textBaseline = "bottom"; ctx.fillStyle = cssCol(1);
      H.marks.forEach(m => { if (m.ion) ctx.fillText(m.ion.text, X(m.mz), Y(Math.min(peak(it, m.mz) / 9.99, 100)) - 17); });
      ctx.restore();
    },
  });
  const wk = document.createElement("div"); wk.className = "wk"; g.appendChild(wk);
  wk.innerHTML = `<h3>Scheda di lavoro · modalità difficile</h3>
    <p>Hai solo lo spettro: niente formula, niente classe, nessun ordine obbligato. Il programma ti segnala soltanto ciò che non torna; ${noSol} Seleziona un picco con un clic (o con le frecce sul grafico), poi usa le sezioni qui sotto.</p>
    <div class="st" id="h1"><h4>1. Ione molecolare</h4>
      <div class="row"><button id="h-m-set" class="pri">Il picco selezionato è M</button>
      <label>oppure M non si vede e ipotizzo la massa <input id="h-m-val" size="5" inputmode="numeric" aria-label="massa ipotizzata di M"></label><button id="h-m-hid">M non è visibile</button></div>
      <div class="fb" id="h-m-fb" aria-live="polite"></div></div>
    <div class="st" id="h2"><h4>2. Formula molecolare</h4>
      <div class="row"><label>Formula <input id="h-f" class="mono" size="14" spellcheck="false" placeholder="es. C6H12O"></label><button id="h-f-ok" class="pri">Controlla la coerenza</button></div>
      <div class="fb" id="h-f-fb" aria-live="polite"></div></div>
    <div class="st" id="h3"><h4>3. Gli ioni: la tua ipotesi per ogni picco</h4>
      <p>Seleziona un picco, poi scrivi la formula dello ione (con la carica, es. <span class="mono">C3H7O+</span> o <span class="mono">C4H8+.</span>) e scegli il meccanismo. L'annotazione sullo spettro compare solo dopo la tua ipotesi.</p>
      <div class="row"><span id="h-pk" class="pill">nessun picco selezionato</span>
      <label>Ione <input id="h-ion" class="mono" size="12" spellcheck="false" aria-label="formula dello ione"></label>
      <label>Meccanismo <select id="h-mech" aria-label="meccanismo"><option value="">non so</option>${MECHS.map(m => `<option value="${m}">${EISPEC.TYPE[m]}</option>`).join("")}</select></label>
      <button id="h-add" class="pri">Annota il picco</button></div>
      <div class="fb" id="h-i-fb" aria-live="polite"></div><div id="h-tab"></div></div>
    <div class="st" id="h4"><h4>4. Struttura</h4>
      <p>Disegna la molecola (non lo ione) oppure scrivi lo SMILES. «Controlla la coerenza» confronta la tua struttura con la tua formula e con gli ioni che hai annotato; se è la molecola giusta lo scopri solo alla consegna.</p>
      <iframe class="kframe" title="Editor di strutture" sandbox="allow-scripts allow-same-origin allow-modals allow-downloads"></iframe>
      <div class="row"><button id="h-s-ok" class="pri">Controlla la coerenza</button><label>oppure SMILES <input id="k-smi" class="mono" size="22" spellcheck="false"></label></div>
      <div class="fb" id="h-s-fb" aria-live="polite"></div></div>
    <div class="st" id="h5"><h4>5. Aiuti <small>(−5, −10, −20 punti)</small></h4>
      <div class="row"><button id="h-hint" class="hint">Aiuto 1 (−5): una zona dello spettro</button></div>
      <div id="h-hints" aria-live="polite"></div></div>
    <div class="row"><button id="h-done" class="pri keep">Consegna e correggi</button></div><div id="end"></div>`;
  env.startKetcher($("iframe", wk)).catch(e => fb($("#h-s-fb"), "hi", EH(e.message)));

  // --- 1. molecular ion
  const setM = (M, vis) => {
    H.M = M; H.Mvis = vis;
    fb($("#h-m-fb"), "hi", `M = ${M}${vis ? "" : " (non visibile)"}.${list(checkM(it, M, vis))}`);
    if (H.f) $("#h-f-ok").click();
  };
  $("#h-m-set").onclick = () => { const m = S.sel[0]; if (m == null) return fb($("#h-m-fb"), "hi", "Prima seleziona un picco con un clic sullo spettro."); setM(m, true); };
  $("#h-m-hid").onclick = () => { const v = parseInt($("#h-m-val").value, 10); if (!(v > 0)) return fb($("#h-m-fb"), "hi", "Scrivi la massa che ipotizzi per M (un numero intero)."); setM(v, false); };

  // --- 2. formula
  $("#h-f-ok").onclick = () => {
    const f = PAL.parse($("#h-f").value.trim());
    if (!f) return fb($("#h-f-fb"), "no", "Formula non valida: usa i simboli degli elementi con le maiuscole giuste (C, H, N, O, S, Cl, Br, F, Si, P, I).");
    H.f = f;
    fb($("#h-f-fb"), "hi", list(checkFormula(it, f, H.M, H.Mvis)));
  };

  // --- 3. ions
  const renderTab = () => {
    $("#h-tab").innerHTML = H.marks.length ? `<table class="eitab htab"><caption>Le tue annotazioni</caption><tr><th>m/z</th><th>Ione</th><th>Δm da M</th><th>Meccanismo</th><th></th></tr>${H.marks.map((m, i) =>
      `<tr><td class="n">${m.mz}</td><td>${m.ion ? EISPEC.ionHtml(m.ion.text) : "–"}</td><td class="n">${H.M != null ? H.M - m.mz : "–"}</td><td>${m.mech ? EISPEC.TYPE[m.mech] : "non so"}</td><td><button data-rm="${i}" aria-label="Togli l'annotazione del picco ${m.mz}">Togli</button></td></tr>`).join("")}</table>` : "";
    $("#h-tab").querySelectorAll("[data-rm]").forEach(b => { b.onclick = () => { H.marks.splice(+b.dataset.rm, 1); renderTab(); S.spec.redraw(); }; });
  };
  $("#h-add").onclick = () => {
    const mz = S.sel.length ? S.sel[S.sel.length - 1] : null;
    if (mz == null) return fb($("#h-i-fb"), "hi", "Seleziona prima il picco da annotare (clic sul grafico o frecce).");
    const txt = $("#h-ion").value.trim(), ion = txt ? parseIon(txt) : null, mech = $("#h-mech").value;
    if (txt && !ion) return fb($("#h-i-fb"), "no", "Formula dello ione non valida: es. C3H7O+ oppure C4H8+. (la carica è facoltativa).");
    if (!txt && !mech) return fb($("#h-i-fb"), "hi", "Scrivi la formula dello ione o scegli un meccanismo: l'annotazione nasce dalla tua ipotesi.");
    const checks = checkIon(it, mz, ion, mech, H.f, H.M, LOSSNAME);
    H.marks = H.marks.filter(m => m.mz !== mz); H.marks.push({ mz, ion, mech });
    H.marks.sort((a, b) => a.mz - b.mz);
    fb($("#h-i-fb"), "hi", `<b>m/z ${mz}</b>${list(checks)}`);
    renderTab(); S.spec.redraw();
    $("#h-ion").value = ""; $("#h-mech").value = "";
  };

  // --- 4. structure
  const current = async () => { await env.ocl(); return env.proposal(); };
  $("#h-s-ok").onclick = async () => {
    let mol;
    try { mol = await current(); } catch (e) { return fb($("#h-s-fb"), "hi", EH(e.message)); }
    const gf = PAL.parse(mol.getMolecularFormula().formula), cs = [];
    cs.push({ ok: null, t: `La tua struttura è ${PAL.fhtml(gf)} (massa nominale ${PAL.nominal(gf)}, RDB ${PAL.rdb(gf)}).` });
    if (H.f) cs.push(PAL.fstr(gf) === PAL.fstr(H.f) ? { ok: true, t: "Coincide con la formula che hai proposto." } : { ok: false, t: `Non coincide con la formula che hai proposto (${PAL.fhtml(H.f)}).` });
    if (H.M != null) cs.push(PAL.nominal(gf) === H.M ? { ok: true, t: "La massa nominale coincide con M." } : { ok: false, t: `La massa nominale è diversa da M (${H.M}).` });
    H.marks.filter(m => m.ion).forEach(m => { if (!subFormula(m.ion.f, gf)) cs.push({ ok: false, t: `Lo ione annotato a m/z ${m.mz} (${PAL.fhtml(m.ion.f)}) non è un frammento possibile della tua struttura.` }); });
    fb($("#h-s-fb"), "hi", list(cs));
  };

  // --- 5. hints
  const hb = $("#h-hint");
  const refreshHint = () => { hb.textContent = H.hint < 3 ? `Aiuto ${H.hint + 1} (−${HINT_COST[H.hint]}): ${["una zona dello spettro", "una famiglia di ioni", "una zona della molecola"][H.hint]}` : "Nessun altro aiuto"; hb.disabled = H.hint >= 3; };
  hb.onclick = () => {
    if (H.hint >= 3) return;
    const t = H.hintTexts, n = H.hint;
    pay(HINT_COST[n], "aiuto " + (n + 1));
    S.hints.hard = H.hint = n + 1;
    if (n === 0) { H.band = t.band; S.spec.redraw(); }
    $("#h-hints").insertAdjacentHTML("beforeend", `<div class="fb hi"><b>Aiuto ${n + 1}.</b> ${EH([t.l1, t.l2, t.l3][n])}</div>`);
    refreshHint();
  };
  refreshHint();

  // --- delivery: the only point where the answer appears
  $("#h-done").onclick = async () => {
    if (H.delivered) return;
    H.delivered = true;
    wk.querySelectorAll("button:not(.keep), input, select").forEach(b => { b.disabled = true; });
    $("#h-done").disabled = true;
    const f = PAL.parse(it.formula), vis = env.mVisible(it);
    let mol = null; try { mol = await current(); } catch (_) { /* nothing drawn */ }
    const mOk = H.M === it.M && H.Mvis === vis;
    const fOk = !!H.f && PAL.fstr(H.f) === PAL.fstr(f);
    const isoOk = !!H.f && ["Cl", "Br", "S"].every(e => (H.f[e] || 0) === (f[e] || 0));
    const sOk = !!mol && env.norm(mol) === env.idOf(it.smiles);
    const qs = it.keys.filter(k => k.type !== "M" && k.type !== "isotopo");
    const hit = qs.filter(k => H.marks.some(m => m.mz === k.mz && m.mech === k.type)).length;
    const mech = qs.length ? hit / qs.length : 1;
    const total = Math.round(S.pts * (0.15 * mOk + 0.2 * fOk + 0.2 * mech + 0.45 * sOk));
    PAL.record(env.game, it.id, it.level - 1, { M: +mOk, iso: +isoOk, formula: +fOk, meccanismi: mech, serie: mech, struttura: +sOk }, total);
    const mark = ok => ok ? "✓" : "✗";
    const end = $("#end");
    end.innerHTML = `<div class="st"><h4>Correzione: <span class="score">${total}</span> punti</h4>
      <div class="grid2"><div>${env.solutionSvg(it)}</div><div><p><b>${EH(it.name)}</b>, ${PAL.fhtml(f)}, M = ${it.M}${vis ? "" : " (non visibile)"}, famiglia: ${FAMNAME[it.family] || it.family}.</p>
      <p>Ione molecolare ${mark(mOk)} · Formula ${mark(fOk)} · Isotopi ${mark(isoOk)} · Meccanismi ${hit} su ${qs.length} · Struttura ${mark(sOk)}${!mol ? " (non disegnata)" : ""}.</p>
      <p>Punti di partenza ${S.pts} (cento meno gli aiuti usati).</p></div></div>
      ${EISPEC.keysHtml(it)}
      ${it.skills ? `<p>Abilità esercitate: ${it.skills.map(k => SKILLNAME[k] || k).join(", ")}.</p>` : ""}
      <p>Rileggi: <a href="15-ei-metodo.html">capitolo 15</a>, <a href="16-ei-famiglie.html">capitolo 16</a>.</p>
      <div class="row"><button class="pri keep" id="h-next">Problema nuovo</button><button class="keep" id="h-print">Stampa</button></div></div>`;
    $("#h-next").onclick = () => env.again();
    $("#h-print").onclick = () => print();
    env.fillSvg(end);
    S.spec.redraw();
    end.scrollIntoView({ behavior: "smooth", block: "start" });
  };
}
