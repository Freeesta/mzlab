// QqQ lab - Pratica: «Dallo spettro alla struttura». A real EI spectrum (pratica/ei-dati.js) and the steps of the McLafferty procedure
// (chapter 15): molecular ion, formula and RDB, key ions and their mechanisms, structure drawn in Ketcher and compared with
// OpenChemLib (same molecule, isomer or wrong formula). Levels 1-4 give feedback at every step; "Modalità orale" asks for the written
// reasoning and corrects only at the end. Globals from classic scripts: TP (teoria.js), EI_DATA, EISPEC, PAL.
// These exercises use library compounds with a known answer: they never look at the student's own data files.
// OpenChemLib (1.1 MB) is loaded only when a structure has to be checked or drawn, Ketcher (30 MB) only when the editor is opened.
let OCL = null;
const ocl = async () => OCL || (OCL = await import("../../vendor/openchemlib.js"));

const $ = (q, el = document) => el.querySelector(q);
const EH = t => String(t).replace(/[&<>"]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));
const ITEMS = EI_DATA.items, PLAY = ITEMS.filter(i => i.play);
const FAMNAME = { alcani: "alcani", alcheni: "alcheni e cicloalcani", aromatici: "aromatici", alcoli: "alcoli", eteri: "eteri", carbonilici: "aldeidi e chetoni",
  acidi: "acidi ed esteri", azotati: "composti azotati", zolfo: "composti dello zolfo", alogenati: "alogenati", fenoli: "fenoli ed eterocicli", ambientali: "contaminanti ambientali" };
const STEPNAME = { M: "Ione molecolare", formula: "Formula e RDB", ioni: "Ioni chiave", struttura: "Struttura" };
const LOSSNAME = { 1: "H•", 15: "CH₃•", 16: "O / NH₂•", 17: "OH• / NH₃", 18: "H₂O", 19: "F•", 20: "HF", 26: "C₂H₂", 27: "HCN", 28: "CO / C₂H₄", 29: "CHO• / C₂H₅•",
  30: "CH₂O / NO", 31: "CH₃O•", 32: "CH₃OH / S", 34: "H₂S", 35: "Cl•", 36: "HCl", 42: "CH₂CO / C₃H₆", 43: "CH₃CO• / C₃H₇•", 44: "CO₂ / C₃H₈", 45: "COOH• / C₂H₅O•", 46: "NO₂•", 57: "C₄H₉•", 60: "CH₃COOH", 79: "Br•" };
const P = new URLSearchParams(location.search);

let S = null;          // state of the current problem
let K = null, kStart = null;   // Ketcher

// ------------------------------------------------------------------ helpers
const peak = (it, m) => { const p = it.peaks.find(q => q[0] === m); return p ? p[1] : 0; };
const mVisible = it => peak(it, it.M) >= 2;
const ratio = (it, m, d) => { const a = peak(it, m); return a ? peak(it, m + d) / a * 100 : null; };
const pct = v => v == null ? "–" : v.toFixed(v < 10 ? 1 : 0).replace(".", ",") + "%";
const hintCost = n => (S.level >= 3 ? 2 : 1) * [5, 10, 20][Math.min(n, 2)];
function pay(n, why) { S.pts = Math.max(0, S.pts - n); S.log.push(why); $("#pts").textContent = `${S.pts} punti`; }
function fb(el, cls, html) { el.className = "fb " + cls; el.innerHTML = html; }
const parity = f => { const r = PAL.rdb(f); return r === Math.floor(r) ? "dispari" : "pari"; };   // electrons
function ionParts(ion) { const m = /^([A-Za-z0-9]+)(\+\.?)$/.exec(ion); return m ? { f: PAL.parse(m[1]), odd: m[2] === "+." } : null; }

// ------------------------------------------------------------------ choosing the problem
function pool() {
  const fam = $("#fam").value, lv = $("#lv").value;
  let a = PLAY.filter(i => !fam || i.family === fam);
  if (lv === "1") a = a.filter(i => i.level <= 1).length ? a.filter(i => i.level <= 1) : a.filter(i => i.level <= 2);
  else if (lv === "2") a = a.filter(i => i.level <= 2);
  return a.length ? a : PLAY;
}
function next(id) {
  let it = id && ITEMS.find(i => i.id === id);
  if (!it && S && S.chall) it = S.chall.list[S.chall.k];
  if (!it) {
    const items = pool().map(i => ({ id: i.id, d0: i.level - 2, skills: ["M", "formula", "meccanismi", "struttura"], fam: i.family }));
    const ch = PAL.pick($("#lv").value === "orale" ? "ei-orale" : "ei", items, { avoidFam: S && S.item ? S.item.family : null });
    it = ITEMS.find(i => i.id === ch.id);
  }
  start(it);
}

// ------------------------------------------------------------------ the spectrum card (click = select a peak, Shift+click = second peak and Δm)
function spectrumCard(el, it, opt = {}) {
  const box = document.createElement("div"); box.className = "eispec"; el.appendChild(box);
  box.innerHTML = `<h4>${opt.title || "Composto incognito"}</h4>`;
  const c = TP.canvas(box, opt.h || 280);
  const read = document.createElement("div"); read.className = "fb hi"; read.setAttribute("aria-live", "polite"); box.appendChild(read);
  let ax = null;
  const draw = () => { ax = EISPEC.plot(c, it, { sel: S.sel, keys: opt.keys }); };
  c.onresize = draw; draw();
  if (opt.table !== false) box.insertAdjacentHTML("beforeend", EISPEC.table(it));
  box.insertAdjacentHTML("beforeend", `<p class="src">${EISPEC.source(it)}</p>`);
  if (opt.click !== false) {
    c.cv.style.cursor = "crosshair";
    const pick = (best, second) => {
      S.sel = second && S.sel.length ? [S.sel[0], best] : [best];
      draw();
      const [a, b] = S.sel;
      read.innerHTML = b != null ? `m/z ${a} → ${b}: Δm = <b>${Math.abs(a - b)}</b>${LOSSNAME[Math.abs(a - b)] ? ` (perdita tipica: ${LOSSNAME[Math.abs(a - b)]})` : ""}` :
        `Selezionato m/z <b>${a}</b> (${pct(peak(it, a) / 9.99)} del picco base). Maiusc+clic su un altro picco per la differenza.`;
    };
    c.cv.addEventListener("click", ev => {
      const r = c.cv.getBoundingClientRect(), x = ev.clientX - r.left;
      let best = null, bd = 12;
      it.peaks.forEach(([m, v]) => { const d = Math.abs(ax.X(m) - x); if (d < bd && v >= 2) { bd = d; best = m; } });
      if (best != null) pick(best, ev.shiftKey);
    });
    // keyboard: the arrows walk through the peaks (Shift = second peak, as Shift+click); Home / End go to the first / last
    const ms = it.peaks.filter(p => p[1] >= 2).map(p => p[0]).sort((p, q) => p - q);
    c.cv.tabIndex = 0; c.cv.setAttribute("role", "group");
    c.cv.dataset.title = (opt.title || "Spettro EI") + " (selezione dei picchi con le frecce sinistra e destra; Maiusc+freccia per un secondo picco)";
    draw();
    c.cv.addEventListener("keydown", ev => {
      if (!ms.length || !/^(ArrowLeft|ArrowRight|Home|End)$/.test(ev.key)) return;
      ev.preventDefault();
      const cur = S.sel.length ? S.sel[S.sel.length - 1] : null;
      let i = cur == null ? (ev.key === "ArrowLeft" || ev.key === "End" ? ms.length - 1 : 0) : ms.indexOf(cur) + (ev.key === "ArrowLeft" ? -1 : ev.key === "ArrowRight" ? 1 : 0);
      if (ev.key === "Home") i = 0; if (ev.key === "End") i = ms.length - 1;
      pick(ms[Math.max(0, Math.min(ms.length - 1, i))], ev.shiftKey);
    });
    read.innerHTML = "Clic su un picco per selezionarlo (da tastiera: frecce sul grafico); Maiusc+clic o Maiusc+freccia su un secondo picco per la differenza di massa.";
  } else read.remove();
  return { redraw: draw };
}

// ------------------------------------------------------------------ one problem
function start(it) {
  const lv = $("#lv").value;
  S = { item: it, level: lv === "orale" ? 5 : +lv, oral: lv === "orale", pts: 100, sel: [], log: [], res: {}, hints: {}, chall: S && S.chall, t0: Date.now() };
  $("#pts").textContent = "100 punti";
  const g = $("#game"); g.innerHTML = "";
  if (S.chall) g.insertAdjacentHTML("beforeend", `<p class="pill">Sfida ${S.chall.code}: problema ${S.chall.k + 1} di ${S.chall.list.length}</p>`);
  S.spec = spectrumCard(g, it, S.oral ? { table: $("#fmt") && $("#fmt").value === "tab", click: false, title: "Spettro EI (70 eV) da interpretare", h: 340 } : { table: S.level !== 4 });
  if (S.oral) return oral(g, it);
  const wk = document.createElement("div"); wk.className = "wk"; g.appendChild(wk);
  wk.innerHTML = `<h3>Scheda di lavoro</h3>
    <div class="st" id="s1"></div><div class="st off" id="s2"></div><div class="st off" id="s3"></div><div class="st off" id="s4"></div><div id="end"></div>`;
  stepM($("#s1"));
}

function hintBtn(el, step, texts) {
  const b = document.createElement("button"); b.className = "hint";
  const out = document.createElement("div"); out.className = "fb";
  const upd = () => { const n = S.hints[step] || 0; b.textContent = n < texts.length ? `Suggerimento ${n + 1} (−${hintCost(n)})` : "Nessun altro suggerimento"; b.disabled = n >= texts.length; };
  b.onclick = () => { const n = S.hints[step] || 0; if (n >= texts.length) return; pay(hintCost(n), "suggerimento " + step); S.hints[step] = n + 1; fb(out, "hi", texts.slice(0, n + 1).map((t, i) => `<b>${i + 1}.</b> ${t}`).join("<br>")); upd(); };
  upd(); el.querySelector(".row").appendChild(b); el.appendChild(out);
}
function done(el, ok, key, score) {
  S.res[key] = score; el.classList.add("done");
  el.querySelectorAll("button:not(.keep)").forEach(b => { b.disabled = true; });
  const nx = el.nextElementSibling;
  if (nx && nx.classList.contains("st")) { nx.classList.remove("off"); const f = { s2: stepFormula, s3: stepIons, s4: stepStructure }[nx.id]; f && f(nx); nx.scrollIntoView({ behavior: "smooth", block: "nearest" }); }
}

// ---- step 1: molecular ion
function stepM(el) {
  const it = S.item, vis = mVisible(it);
  el.innerHTML = `<h4>1. Ione molecolare <small>(passi 2.1-2.3)</small></h4>
    <p>Seleziona sullo spettro il picco che secondo te è M<sup>+•</sup> e conferma; oppure, se pensi che lo ione molecolare non compaia, dillo.</p>
    <div class="row"><button class="pri" id="m-ok">Questo è M</button><button id="m-no">M non è visibile</button></div><div class="fb" id="m-fb"></div>`;
  let wrong = 0;
  const top = Math.max(...it.peaks.filter(p => p[1] >= 10).map(p => p[0]));
  hintBtn(el, "M", [
    "Guarda il gruppo di picchi più a destra e le differenze fra il picco più alto e i suoi vicini.",
    `I tre test (capitolo 15): massa più alta (a parte gli isotopi), elettroni dispari (regola dell'azoto), perdite logiche verso gli ioni importanti. Perdite di 4-14 o 21-25 unità sono impossibili.`,
    vis ? `M = ${it.M}.` : `Lo ione molecolare (M = ${it.M}) non compare: il picco più alto è un frammento.`]);
  const ok = msg => { fb($("#m-fb"), "ok", msg + (S.level <= 2 ? ` ${it.M % 2 ? "M dispari: un numero dispari di azoti." : "M pari: nessun azoto o un numero pari."}` : "")); done(el, true, "M", wrong ? 0.5 : 1); };
  const ko = msg => {
    wrong++; pay(5, "M sbagliato");
    if (wrong >= (S.level <= 2 ? 2 : 3)) { fb($("#m-fb"), "no", msg + ` <br><b>Soluzione:</b> ${vis ? `M = ${it.M}` : `M = ${it.M}, ma non si vede nello spettro`}.`); done(el, false, "M", 0); }
    else fb($("#m-fb"), "no", msg + " Riprova.");
  };
  $("#m-ok").onclick = () => {
    const m = S.sel[0];
    if (m == null) { fb($("#m-fb"), "hi", "Prima seleziona un picco con un clic sullo spettro."); return; }
    if (m === it.M && vis) return ok(`Giusto: M = ${it.M}.`);
    if (m > it.M) return ko(`m/z ${m} è un picco isotopico (M+${m - it.M}): per convenzione M contiene l'isotopo più abbondante di ogni elemento.`);
    const d = (vis ? it.M : top) - m;
    if (vis && m < it.M) return ko(`C'è ancora un picco più in alto (m/z ${it.M}) che può essere lo ione molecolare: primo test.` + (d >= 4 && d <= 14 || d >= 21 && d <= 25 ? ` Inoltre fra ${it.M} e ${m} ci sono ${d} unità, una perdita impossibile.` : ""));
    return ko(`m/z ${m} è un frammento. ${!vis ? "In questo composto lo ione molecolare non si vede: ragiona su quale massa spiegherebbe i picchi più alti con perdite logiche." : ""}`);
  };
  $("#m-no").onclick = () => vis ? ko(`Lo ione molecolare c'è: guarda bene il gruppo più alto (anche picchi piccoli contano).`) : ok(`Giusto: lo ione molecolare non compare. Dai picchi più alti e dalle perdite logiche si ricava M = ${it.M}.`);
}

// ---- step 2: formula and rings plus double bonds
function stepFormula(el) {
  const it = S.item, f = PAL.parse(it.formula), vis = mVisible(it);
  const r1 = ratio(it, it.M, 1), r2 = ratio(it, it.M, 2), r4 = ratio(it, it.M, 4);
  el.innerHTML = `<h4>2. Formula e insaturazioni <small>(passi 1.3-1.9)</small></h4>
    <p>M = <b>${it.M}</b>. Scrivi la formula molecolare (es. C6H12O) e il valore di anelli + doppi legami (RDB).</p>
    <div class="row"><label>Formula <input id="f-in" class="mono" size="14" spellcheck="false"></label><label>RDB <input id="f-rdb" size="4" inputmode="decimal"></label><button class="pri" id="f-ok">Verifica</button>
    ${S.level <= 3 ? `<button id="f-exact" class="keep">Compra la massa esatta (−20)</button>` : ""}</div>
    <div class="fb" id="f-ratios"></div><div class="fb" id="f-fb"></div>`;
  const warn = it.iso && !it.iso.ok && vis ? ` <b>Attenzione</b>: in questo spettro il gruppo di M non è affidabile (M debole, M+1 gonfiato da [M+H]<sup>+</sup> per autoprotonazione o gruppo incompleto): per i carboni fidati di più della massa e dei frammenti.` : "";
  const ratios = () => fb($("#f-ratios"), "hi", (vis ? `Rapporti misurati: M+1/M = <b>${pct(r1)}</b>, M+2/M = <b>${pct(r2)}</b>, M+4/M = <b>${pct(r4)}</b> (circa 1,1% per atomo di C a M+1; 32% per Cl e 97% per Br a M+2; 4,5% per S).` : "Lo ione molecolare non si vede: niente rapporti isotopici su M; usa i gruppi dei frammenti (per esempio un cluster del cloro) e la massa.") + warn);
  if (S.level <= 2) ratios();
  else { const b = document.createElement("button"); b.textContent = "Mostra i rapporti isotopici (−5)"; b.className = "keep"; b.onclick = () => { pay(5, "rapporti"); ratios(); b.remove(); }; el.querySelector(".row").appendChild(b); }
  if ($("#f-exact")) $("#f-exact").onclick = () => {
    pay(20, "massa esatta"); const r = PAL.rng(it.id.length * 7919 + it.M), err = (r() - 0.5) * 4;
    const m = PAL.exact(f) - 0.000548579909, meas = m * (1 + err * 1e-6);
    fb($("#f-ratios"), "hi", `Massa esatta di M<sup>+•</sup> misurata ad alta risoluzione: <b>${meas.toFixed(4).replace(".", ",")}</b> (errore dello strumento entro ±2 ppm). Confronta con la massa della tua formula (capitolo 13).`);
    $("#f-exact").disabled = true;
  };
  hintBtn(el, "formula", [
    "Prima M+2 (Cl, Br, S), poi M+1 (carboni: rapporto / 1,1), poi ossigeni e idrogeni per completare la massa; RDB = C − (H+X)/2 + N/2 + 1.",
    `Atomi pesanti: ${["Cl", "Br", "S", "N", "O"].map(e => `${e} ${f[e] || 0}`).join(", ")}.`,
    `Formula ${PAL.fhtml(f)}, RDB = ${PAL.rdb(f)}.`]);
  let wrong = 0;
  $("#f-ok").onclick = () => {
    const g = PAL.parse($("#f-in").value.trim()), rv = parseFloat(String($("#f-rdb").value).replace(",", "."));
    if (!g) return fb($("#f-fb"), "no", "Formula non valida: usa i simboli degli elementi con le maiuscole giuste (C, H, N, O, S, Cl, Br, F, Si, P, I).");
    const msgs = [];
    if (PAL.nominal(g) !== it.M) msgs.push(`La massa nominale di ${PAL.fhtml(g)} è ${PAL.nominal(g)}, non ${it.M}.`);
    const rg = PAL.rdb(g);
    if (rg < 0 || rg !== Math.floor(rg)) msgs.push(`RDB di ${PAL.fhtml(g)} = ${String(rg).replace(".", ",")}: una molecola deve avere RDB intero e non negativo (regola dell'azoto).`);
    if (!msgs.length && PAL.fstr(g) !== PAL.fstr(f)) {
      ["Cl", "Br", "S"].forEach(e => { if ((g[e] || 0) !== (f[e] || 0)) msgs.push(`Controlla ${e}: con ${g[e] || 0} atomi ti aspetti M+2 ≈ ${pct(PAL.isoPattern(g)[2])}; ${vis ? `nello spettro M+2 ≈ ${pct(r2)}` : "guarda i gruppi isotopici dei frammenti"}.`); });
      if ((g.C || 0) !== (f.C || 0) && vis && r1 != null && (!it.iso || it.iso.ok)) msgs.push(`Carboni: M+1/M = ${pct(r1)} → circa ${Math.round(r1 / 1.1)} C (±1).`);
      if (!msgs.length) msgs.push("Massa e isotopi tornano, ma la formula non è quella giusta: pensa ai frammenti (ci sono perdite di O, N?) e all'RDB che serve per spiegarli.");
    }
    const formulaOk = !msgs.length, rdbOk = Number.isFinite(rv) && rv === PAL.rdb(f);
    if (formulaOk && rdbOk) { fb($("#f-fb"), "ok", `Giusto: ${PAL.fhtml(f)}, RDB = ${PAL.rdb(f)}.${S.level <= 2 ? ` ${PAL.rdb(f) >= 4 ? "RDB ≥ 4: probabile anello aromatico." : PAL.rdb(f) === 0 ? "RDB 0: niente anelli né doppi legami." : "Un'insaturazione o un anello per ogni unità di RDB."}` : ""}`); return done(el, true, "formula", wrong ? Math.max(0.3, 1 - 0.25 * wrong) : 1); }
    if (formulaOk && !rdbOk) msgs.push(`Formula giusta. L'RDB non torna: RDB = C − (H + X)/2 + N/2 + 1.`);
    wrong++; pay(5, "formula");
    if (wrong >= 3) { fb($("#f-fb"), "no", msgs.join(" ") + `<br><b>Soluzione:</b> ${PAL.fhtml(f)}, RDB = ${PAL.rdb(f)}.`); return done(el, false, "formula", 0); }
    fb($("#f-fb"), "no", msgs.join(" "));
  };
}

// ---- step 3: key ions and their mechanisms
const MECH = ["alpha", "i", "sigma", "mclafferty", "rda", "tropilio", "orto", "perdita", "riarr", "serie"];
function stepIons(el) {
  const it = S.item, r = PAL.rng(it.M * 31 + it.id.length);
  const pref = { mclafferty: 0, rda: 0, orto: 0, tropilio: 1, alpha: 1, i: 2, riarr: 2, sigma: 3, serie: 3, perdita: 4 };
  const ks = it.keys.filter(k => k.type !== "M" && k.type !== "isotopo").sort((a, b) => (pref[a.type] ?? 5) - (pref[b.type] ?? 5));
  const n = S.level === 1 ? 1 : S.level === 2 ? 2 : 3, qs = ks.slice(0, n);
  if (!qs.length) { el.innerHTML = `<h4>3. Ioni chiave</h4><p>Per questo composto non ci sono ioni chiave da discutere oltre allo ione molecolare.</p>`; return done(el, true, "ioni", 1); }
  el.innerHTML = `<h4>3. Ioni chiave <small>(passi 2.5-2.7, 3.2-3.3)</small></h4><p>Da che cosa nasce ciascuno di questi picchi?</p><div id="qq"></div><div class="row"></div>`;
  let right = 0, answered = 0;
  qs.forEach((k, qi) => {
    const ion = ionParts(k.ion), d = it.M - k.mz;
    const opts = PAL.shuffle([k.type, ...PAL.shuffle(MECH.filter(t => t !== k.type), r).slice(0, 3)], r);
    const q = document.createElement("div"); q.className = "st";
    q.innerHTML = `<p><b>m/z ${k.mz}</b>${S.level <= 2 ? ` (${EISPEC.ionHtml(k.ion)}, ione a elettroni ${ion && ion.odd ? "dispari" : "pari"})` : ""}${d > 0 && S.level <= 2 ? `, M − ${d}` : ""}:</p><div class="opts"></div><div class="fb"></div>`;
    const box = $(".opts", q), out = $(".fb", q);
    let tries = 0;
    opts.forEach(t => {
      const b = document.createElement("button"); b.textContent = EISPEC.TYPE[t] || t;
      b.onclick = () => {
        if (b.disabled) return;
        if (t === k.type) {
          b.classList.add("right"); box.querySelectorAll("button").forEach(x => { x.disabled = true; });
          fb(out, "ok", `${EISPEC.ionHtml(k.ion)}: ${EH(k.text)}`); if (!tries) right++; answered++;
          if (answered === qs.length) done(el, true, "ioni", right / qs.length);
        } else {
          tries++; pay(5, "ione"); b.classList.add("wrong"); b.disabled = true;
          const io = ionParts(k.ion);
          fb(out, "no", t === "mclafferty" || t === "rda" || t === "orto" ? `No: questo meccanismo dà ioni a elettroni dispari${io && !io.odd ? ", e questo ione è a elettroni pari" : ""}.` :
            ["alpha", "i", "sigma", "tropilio", "serie"].includes(t) && io && io.odd ? "No: una scissione semplice dà uno ione a elettroni pari, e questo è a elettroni dispari (massa pari senza azoti, o RDB intero)." : "No, riprova: pensa a quale legame si rompe e a dove resta la carica.");
          if (tries >= 2) { box.querySelectorAll("button").forEach(x => { x.disabled = true; if ((x.textContent) === (EISPEC.TYPE[k.type] || k.type)) x.classList.add("right"); }); fb(out, "hi", `<b>Soluzione:</b> ${EISPEC.TYPE[k.type]} — ${EISPEC.ionHtml(k.ion)}: ${EH(k.text)}`); answered++; if (answered === qs.length) done(el, true, "ioni", right / qs.length); }
        }
      };
      box.appendChild(b);
    });
    $("#qq", el).appendChild(q);
  });
  hintBtn(el, "ioni", ["Per ogni ione chiediti: è a elettroni pari o dispari? Quanto vale M − m/z e quale radicale o molecola è stata persa?",
    "Elettroni dispari a massa pari (senza N) = due legami rotti: McLafferty, retro-Diels-Alder, eliminazione. Elettroni pari = una rottura: α, induttiva, σ."]);
}

// ---- step 4: structure (Ketcher + OpenChemLib)
function startKetcher(fr) {
  if (kStart && K && fr.contentWindow && fr.contentWindow.ketcher === K) return kStart;
  K = null;
  kStart = new Promise((resolve, reject) => {
    const on = e => { if (e.source === fr.contentWindow && e.data && e.data.type === "ketcher-ready") { removeEventListener("message", on); K = fr.contentWindow.ketcher; try { const st = fr.contentDocument.createElement("style"); st.textContent = '[data-testid="polymer-toggler"]{display:none!important}'; fr.contentDocument.head.appendChild(st); } catch (_) { /* cosmetic */ } resolve(K); } };
    addEventListener("message", on);
    fr.src = "../vendor/ketcher/index.html";
    setTimeout(() => reject(new Error("L'editor non si è avviato: scrivi lo SMILES qui sotto.")), 60000);
  });
  return kStart;
}
const norm = mol => { const m = mol.getCompactCopy ? mol.getCompactCopy() : mol; m.stripStereoInformation(); return m.getIDCode(); };
const idOf = smi => { try { return norm(OCL.Molecule.fromSmiles(smi)); } catch (_) { return null; } };
async function proposal() {
  const smi = ($("#k-smi") || {}).value;
  if (smi && smi.trim()) return OCL.Molecule.fromSmiles(smi.trim());
  if (!K) throw new Error("Disegna la struttura (l'editor si sta avviando) oppure scrivi lo SMILES.");
  const mf = await K.getMolfile();
  const mol = OCL.Molecule.fromMolfile(mf);
  if (!mol.getAllAtoms()) throw new Error("Non hai disegnato niente.");
  return mol;
}
function stepStructure(el) {
  const it = S.item;
  el.innerHTML = `<h4>4. Struttura <small>(passi 4.1-4.2)</small></h4>
    <p>Disegna la molecola (non lo ione) e verifica. Se proponi un isomero il gioco te lo dice e, se lo conosce, ti mostra il suo spettro.</p>
    <iframe class="kframe" title="Editor di strutture" sandbox="allow-scripts allow-same-origin allow-modals allow-downloads"></iframe>
    <div class="row"><button class="pri" id="k-ok">Verifica la struttura</button><label>oppure SMILES <input id="k-smi" class="mono" size="22" spellcheck="false"></label></div>
    <div class="fb" id="k-fb"></div><div id="k-mirror"></div>`;
  startKetcher($("iframe", el)).catch(e => fb($("#k-fb"), "hi", EH(e.message)));
  hintBtn(el, "struttura", [
    `Riassumi: ${PAL.fhtml(PAL.parse(it.formula))}, RDB ${PAL.rdb(PAL.parse(it.formula))}; quali gruppi spiegano gli ioni chiave?`,
    `Famiglia: ${FAMNAME[it.family] || it.family}.`,
    `È ${EH(it.name)}.`]);
  let wrong = 0;
  $("#k-ok").onclick = async () => {
    let mol;
    try { await ocl(); mol = await proposal(); } catch (e) { return fb($("#k-fb"), "hi", EH(e.message)); }
    const want = idOf(it.smiles), got = norm(mol), gf = mol.getMolecularFormula().formula;
    $("#k-mirror").innerHTML = "";
    if (got === want) { fb($("#k-fb"), "ok", `Esatto: <b>${EH(it.name)}</b>.`); return done(el, true, "struttura", wrong ? Math.max(0.4, 1 - 0.2 * wrong) : 1), finish(); }
    wrong++; pay(5, "struttura");
    const same = PAL.fstr(PAL.parse(gf) || {}) === PAL.fstr(PAL.parse(it.formula));
    let msg = same ? `È un isomero (${EH(gf)} è la formula giusta) ma non la molecola giusta. Controlla che la tua struttura spieghi gli ioni chiave.` : `La tua struttura è ${EH(gf)}: non corrisponde alla formula ${PAL.fhtml(PAL.parse(it.formula))}.`;
    const other = same && ITEMS.find(o => o.id !== it.id && idOf(o.smiles) === got);
    if (other) {
      msg += ` La tua proposta è <b>${EH(other.name)}</b>: sotto il suo spettro a confronto.`;
      const m = $("#k-mirror"), a = document.createElement("div"); m.appendChild(a);
      const c = TP.canvas(a, 220), draw = () => EISPEC.plot(c, other, {}); c.onresize = draw; draw();
      a.insertAdjacentHTML("afterbegin", `<p><b>Spettro di ${EH(other.name)}</b> (la tua proposta), da confrontare con quello del problema:</p>`);
    }
    if (wrong >= 3) { fb($("#k-fb"), "no", msg + `<br><b>Soluzione:</b> ${EH(it.name)}.`); done(el, false, "struttura", 0); return finish(); }
    fb($("#k-fb"), "no", msg);
  };
}

// ------------------------------------------------------------------ end of a problem
const solutionSvg = it => `<div class="solsvg" data-smi="${EH(it.smiles)}"></div>`;
function fillSvg(root) { ocl().then(O => root.querySelectorAll(".solsvg").forEach(d => { try { d.innerHTML = (window.MOLUI ? MOLUI.svg(O.Molecule.fromSmiles(d.dataset.smi).toSVG(260, 170)) : O.Molecule.fromSmiles(d.dataset.smi).toSVG(260, 170)); } catch (_) { /* no drawing */ } })).catch(() => { /* offline file: no drawing */ }); }
function finish() {
  const it = S.item, R = S.res, total = S.pts;
  const sc = { M: R.M ?? 0, formula: R.formula ?? 0, iso: R.formula ?? 0, meccanismi: R.ioni ?? 0, serie: R.ioni ?? 0, struttura: R.struttura ?? 0 };
  PAL.record("ei", it.id, it.level - 2, sc, total);
  const end = $("#end");
  end.innerHTML = `<div class="st"><h4>Risultato: <span class="score">${total}</span> punti</h4>
    <div class="grid2"><div>${solutionSvg(it)}</div><div><p><b>${EH(it.name)}</b>, ${PAL.fhtml(PAL.parse(it.formula))}, M = ${it.M}, famiglia: ${FAMNAME[it.family] || it.family}.</p>
    <p>${Object.entries(STEPNAME).map(([k, n]) => `${n}: ${R[k] == null ? "–" : R[k] >= 0.99 ? "✓" : R[k] > 0 ? "in parte" : "✗"}`).join(" · ")}</p></div></div>
    ${EISPEC.keysHtml(it)}
    <p>Rileggi: <a href="15-ei-metodo.html">capitolo 15</a>, <a href="16-ei-famiglie.html">capitolo 16 (${FAMNAME[it.family] || it.family})</a>.</p>
    <div class="row"><button class="pri keep" id="e-next">${S.chall ? "Prossimo della sfida" : "Problema nuovo"}</button><button class="keep" id="e-fam">Un altro della stessa famiglia</button><button class="keep" id="e-print">Stampa</button></div></div>`;
  if (S.chall) { S.chall.total += total; S.chall.k++; }
  $("#e-next").onclick = () => { if (S.chall && S.chall.k >= S.chall.list.length) return challengeEnd(); next(); };
  $("#e-fam").onclick = () => { $("#fam").value = it.family; S.chall = null; next(); };
  $("#e-print").onclick = () => print();
  fillSvg(end);
  S.spec.redraw();
  end.scrollIntoView({ behavior: "smooth", block: "start" });
}

// ------------------------------------------------------------------ oral mode
function oral(g, it) {
  const wk = document.createElement("div"); wk.className = "wk"; g.appendChild(wk);
  const sec = [["o1", "1. Composizione elementare (passi 1.x): isotopi, carboni, eteroatomi"], ["o2", "2. Ione molecolare (passi 2.x): M, regola dell'azoto, ioni a elettroni dispari, perdite"],
    ["o3", "3. Aspetto generale e famiglia (passi 3.x): serie di ioni, ioni caratteristici"], ["o4", "4. Meccanismi: spiega due o tre ioni importanti"], ["o5", "5. Struttura proposta e verifica (passi 4.x): che cosa la conferma, che cosa resta incerto"]];
  wk.innerHTML = `<h3>Modalità orale</h3><p>Scrivi il ragionamento come lo diresti all'esame, sempre con i numeri dei passi (capitolo 15). Nessun aiuto: la correzione arriva quando consegni. <span id="o-clock" class="pill"></span></p>
    ${sec.map(([id, t]) => `<div class="st"><h4>${t}</h4><textarea id="${id}"></textarea></div>`).join("")}
    <div class="st"><h4>Formula molecolare e struttura</h4><p>All'orale la struttura si disegna a mano sul foglio: fate lo stesso (anche sul foglio stampato) e poi, per la correzione automatica, ridisegnatela nell'editor o scrivetene lo SMILES.</p>
    <div class="row"><label>Formula <input id="o-f" class="mono" size="14" spellcheck="false"></label><button id="o-ked" class="noprint">Apri l'editor di strutture</button><label>oppure SMILES <input id="k-smi" class="mono" size="22" spellcheck="false"></label></div>
    <div id="o-kbox"></div><div class="printonly" style="height:9cm;border:1px dashed #999;border-radius:6px;margin-top:8px"><small>Struttura proposta (a mano)</small></div></div>
    <div class="row noprint"><button class="pri" id="o-done">Consegna</button><button id="o-print">Stampa il foglio</button></div><div id="end"></div>`;
  $("#o-ked").onclick = () => {
    $("#o-ked").disabled = true;
    $("#o-kbox").innerHTML = `<iframe class="kframe" title="Editor di strutture" sandbox="allow-scripts allow-same-origin allow-modals allow-downloads"></iframe>`;
    startKetcher($("#o-kbox iframe")).catch(() => { /* SMILES field remains */ });
  };
  const t0 = Date.now(), tick = setInterval(() => { const s = Math.floor((Date.now() - t0) / 1000); const el = $("#o-clock"); if (!el) return clearInterval(tick); el.textContent = `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")} (all'orale 10-15 minuti)`; }, 1000);
  $("#o-print").onclick = () => print();
  $("#o-done").onclick = async () => {
    clearInterval(tick);
    const f = PAL.parse(it.formula), g2 = PAL.parse($("#o-f").value.trim());
    const fOk = !!g2 && PAL.fstr(g2) === PAL.fstr(f);
    let sOk = false, sMsg = "nessuna struttura";
    try { await ocl(); const mol = await proposal(); sOk = norm(mol) === idOf(it.smiles); sMsg = sOk ? "giusta" : `non è ${EH(it.name)}`; } catch (_) { /* nothing drawn */ }
    const vis = mVisible(it), iso = PAL.isoPattern(f);
    const model = [
      `M+2/M ${vis ? `≈ ${pct(ratio(it, it.M, 2))}` : "(M non visibile)"}: ${(f.Cl || f.Br || f.S) ? ["Cl", "Br", "S"].filter(e => f[e]).map(e => `${f[e]} ${e}`).join(", ") + ` (atteso M+2 ≈ ${pct(iso[2])})` : "nessun Cl, Br, S"}; M+1/M ${vis ? `≈ ${pct(ratio(it, it.M, 1))}` : ""} → ${f.C} C. Formula ${PAL.fhtml(f)}, RDB ${PAL.rdb(f)}.`,
      `${vis ? `M = ${it.M}` : `M = ${it.M}, non visibile`}: ${it.M % 2 ? "dispari → numero dispari di N" : "pari → 0 o 2 N"}. ${it.keys.filter(k => k.type !== "M" && k.type !== "isotopo" && ionParts(k.ion) && ionParts(k.ion).odd).map(k => `Ione a elettroni dispari importante: ${k.mz} (${EISPEC.TYPE[k.type]}).`).join(" ")}`,
      `Famiglia: ${FAMNAME[it.family] || it.family}.`,
      it.keys.filter(k => k.type !== "M").map(k => `m/z ${k.mz} ${EISPEC.ionHtml(k.ion)}: ${EISPEC.TYPE[k.type]}. ${EH(k.text)}`).join("<br>"),
      `${EH(it.name)}.`];
    const end = $("#end");
    end.innerHTML = `<div class="st"><h4>Correzione</h4><p>Formula: ${fOk ? "<span class=ok>giusta</span>" : `<span class=bad>${g2 ? PAL.fhtml(g2) : "non data"}</span> (giusta: ${PAL.fhtml(f)})`}. Struttura: <span class="${sOk ? "ok" : "bad"}">${sMsg}</span>.</p>
      <div class="grid2"><div>${solutionSvg(it)}</div><div>${EISPEC.keysHtml(it)}</div></div>
      <p>Confronta il tuo ragionamento con la soluzione modello e valuta ogni parte:</p>
      ${sec.map(([id, t], i) => `<div class="st"><h4>${t}</h4><div class="grid2"><div><small>Tu</small><p>${EH($("#" + id).value) || "<i>(vuoto)</i>"}</p></div><div><small>Soluzione</small><p>${model[i]}</p></div></div>
        <div class="opts" data-i="${i}"><button data-v="1">l'avevo detto</button><button data-v="0.5">in parte</button><button data-v="0">no</button></div></div>`).join("")}
      <div class="row"><button class="pri keep" id="o-score">Calcola il punteggio</button></div><div class="fb" id="o-fb"></div></div>`;
    const self = {};
    end.querySelectorAll(".opts").forEach(o => o.querySelectorAll("button").forEach(b => b.onclick = () => { o.querySelectorAll("button").forEach(x => x.classList.toggle("right", x === b)); self[o.dataset.i] = +b.dataset.v; }));
    fillSvg(end);
    $("#o-score").onclick = () => {
      const vals = sec.map((_, i) => self[i] ?? 0), avg = vals.reduce((a, b) => a + b, 0) / vals.length;
      const total = Math.round(100 * (0.2 * fOk + 0.3 * sOk + 0.5 * avg));
      PAL.record("ei-orale", it.id, it.level - 2, { M: vals[1], formula: fOk ? 1 : vals[0], iso: vals[0], meccanismi: vals[3], serie: vals[2], struttura: sOk ? 1 : 0 }, total);
      fb($("#o-fb"), total >= 70 ? "ok" : "no", `<b>${total}</b> punti (formula 20, struttura 30, ragionamento 50 secondo la tua autovalutazione). ${total >= 70 ? "Buona prova." : "Rileggi le parti segnate «no» e riprova con un altro spettro."} <a href="pratica.html">Vedi i progressi</a> · <a href="#" id="o-next">Altro spettro</a>`);
      $("#o-next").onclick = e => { e.preventDefault(); next(); };
    };
    end.scrollIntoView({ behavior: "smooth", block: "start" });
  };
}

// ------------------------------------------------------------------ class challenge (same 5 problems for everyone with the same code)
function challenge(code) {
  const seed = PAL.seedOf(code); if (seed === null) return false;
  const r = PAL.rng(seed), list = PAL.shuffle(PLAY.filter(i => i.level <= 3), r).slice(0, 5);
  S = { chall: { code, list, k: 0, total: 0 } };
  $("#chall").innerHTML = `<span class="pill">Sfida ${code}</span>`;
  next();
  return true;
}
function challengeEnd() {
  const c = S.chall;
  $("#game").innerHTML = `<div class="wk"><h3>Sfida ${c.code} finita</h3><p class="score">${c.total} punti su ${c.list.length * 100}</p><p>Composti: ${c.list.map(i => EH(i.name)).join(", ")}.</p><div class="row"><button class="pri" onclick="location.href='pratica-ei.html'">Torna agli esercizi</button></div></div>`;
  S.chall = null;
}

// ------------------------------------------------------------------ start
function init() {
  const fams = [...new Set(PLAY.map(i => i.family))];
  $("#fam").insertAdjacentHTML("beforeend", fams.map(f => `<option value="${f}">${FAMNAME[f] || f}</option>`).join(""));
  if (P.get("fam") && fams.includes(P.get("fam"))) $("#fam").value = P.get("fam");
  if (P.get("orale")) $("#lv").value = "orale";
  $("#fmtl").hidden = $("#lv").value !== "orale";
  $("#new").onclick = () => { if (S) S.chall = null; $("#chall").innerHTML = ""; next(); };
  const fmtVis = () => { $("#fmtl").hidden = $("#lv").value !== "orale"; };
  $("#lv").onchange = () => { if (S) S.chall = null; fmtVis(); next(); };
  $("#fmt").onchange = () => { if (S && S.item) start(S.item); };
  $("#fam").onchange = () => { if (S) S.chall = null; next(); };
  if (P.get("sfida") && challenge(P.get("sfida").toUpperCase())) return;
  next(P.get("id"));
}
if (location.protocol !== "file:") addEventListener("DOMContentLoaded", () => setTimeout(init, 0));
window.PRATICA_EI = { start: id => next(id), state: () => S };      // for the tests
