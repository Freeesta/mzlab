// @ts-check
// Card «Compare two MS² spectra» (low resolution): the user picks two selections (file, precursor, time window), the page asks the engine for each one at nominal
// mass (`api/spectrum?rebin=nominal`, the route `mzlab-cli spectrum compare` asks) and shows the `spectrum-compare/1` document of the function that command calls
// (the library .wasm on the site, the `speccompare` op in the app). JS selects and draws; the cosine, the matches and the warnings are the engine's.
// Loaded on first use by libreria.js (menu entry of the MS² spectrum); window.SPCMP.open(panel).

const W_ = /** @type {any} */ (window);
const t = (/** @type {string} */ k, /** @type {any} */ a) => W_.I18N.t(k, a);
const EHt = (/** @type {unknown} */ s) => EH(s == null ? "" : s);
const BASE = /Windows/.test(navigator.userAgent) ? "http://mzlab.localhost/" : "mzlab://localhost/";
const MATCH = 0.6;   // Da: two precursors this close are the one the file lists (the window of the route and of ms2_exps)

const CSS = `<style>#spcmp{width:min(980px,96vw);max-width:96vw}#spcmp .sides{display:flex;gap:12px;flex-wrap:wrap}#spcmp fieldset{flex:1 1 300px;border:1px solid var(--line);border-radius:8px;margin:8px 0;display:flex;flex-direction:column;gap:6px}
  #spcmp label{display:flex;gap:6px;align-items:center;flex-wrap:wrap}#spcmp input[data-num]{width:84px}#spcmp .qbar{display:flex;gap:12px;flex-wrap:wrap;align-items:center;margin:6px 0}
  #spcmp table{border-collapse:collapse;width:100%}#spcmp td,#spcmp th{padding:3px 8px;border-bottom:1px solid var(--line);text-align:left;font-size:12px}#spcmp td.num,#spcmp th.num{text-align:right;font-variant-numeric:tabular-nums}
  #spcmp .tbl{max-height:240px;overflow:auto;border:1px solid var(--line);border-radius:6px}#spcmp .cos{font-size:28px;font-weight:600;font-variant-numeric:tabular-nums}#spcmp .warn{color:var(--warn)}#spcmp .err{color:var(--bad)}
  #spcmp .lists{display:flex;gap:12px;flex-wrap:wrap}#spcmp .lists>div{flex:1 1 280px}</style>`;

/** The precursors a file lists for its MS² scans, ascending, those within 0.6 Da counted once. @param {any} f */
const precursorsOf = f => {
  /** @type {number[]} */ const out = [];
  for (const e of (f && f.ms2_exps) || []) if (e.prec != null && !out.some(p => Math.abs(p - e.prec) <= MATCH)) out.push(e.prec);
  return out.sort((a, b) => a - b);
};
/** The collision energies the file states for a precursor (the facts the page shows beside its spectra). @param {any} f @param {number} prec */
const cesOf = (f, prec) => {
  /** @type {number[]} */ const out = [];
  for (const e of (f && f.ms2_exps) || []) if (e.ce != null && e.prec != null && Math.abs(e.prec - prec) <= MATCH && !out.includes(e.ce)) out.push(e.ce);
  return out;
};
const filesOf = (/** @type {any} */ p) => tabFiles(p.tab || E.tab).filter((/** @type {any} */ f) => !f.gone);
const text = (/** @type {number} */ x) => W_.I18N.num(+x.toFixed(4));
/** a number, a string or a list of numbers as the words of a warning (as `spectrum compare` writes it on stderr) @param {any} x @returns {string} */
const words = x => Array.isArray(x) ? x.map(words).join("/") : x == null ? "" : String(x);

/** One side as the card holds it: file, precursor and time window. @param {HTMLElement} d @param {string} s */
const readSide = (d, s) => {
  const q = (/** @type {string} */ f) => /** @type {HTMLInputElement} */ (d.querySelector(`[data-s="${s}"] [data-f="${f}"]`));
  const f = E.files[+q("file").value], prec = parseFloat(q("prec").value), a = numIt(q("rt0").value), b = numIt(q("rt1").value);
  return { f, prec, a: Math.min(a, b), b: Math.max(a, b) };
};

/** The route's answer for one side, with what the file states about the scans, as the `{"a": .., "b": ..}` member of the engine's request.
 * @param {{ f: any, prec: number, a: number, b: number }} x */
async function sideOf(x) {
  const query = `k=${x.f.k}&rt0=${x.a}&rt1=${x.b}&level=2&precursor=${x.prec}&rebin=nominal`;
  const body = await (await fetch("api/spectrum?" + query)).text();
  let j = null; try { j = JSON.parse(body); } catch (_) { /* not JSON: the engine is asked below and says so */ }
  if (j && j.error) throw W_.I18N.err(j);
  return `{"name":${JSON.stringify(x.f.label || x.f.file)},"query":${JSON.stringify(query)},"precursor":${x.prec},"ce_values":${JSON.stringify(cesOf(x.f, x.prec))},"spectrum":${body}}`;
}

/** The `spectrum-compare/1` text of two sides: the `speccompare` op of the app, the library .wasm on the site. @param {string} params @param {string} sides */
async function engine(params, sides) {
  if (W_.MZLAB_DESKTOP) {
    const r = await fetch(BASE + "rpc", { method: "POST", body: JSON.stringify({ op: "speccompare", params, sides }) });
    const x = await r.text();
    if (r.status !== 200) { let j = null; try { j = JSON.parse(x); } catch (_) { /* plain text */ } throw j ? W_.I18N.err(j) : new Error(x); }
    return x;
  }
  return (await W_.LIB.call("compare", { params, sides })).text;
}

/** @param {any} doc */
function show(doc) {
  const side = (/** @type {string} */ s) => doc[s];
  const warns = doc.warnings.map((/** @type {any} */ w) => `<li class="warn">${EHt(t(w.key, Object.fromEntries(Object.entries(w).map(([k, v]) => [k, words(v)]))))}</li>`).join("");
  const sideRow = (/** @type {string} */ s) => { const x = side(s); return `<tr><th scope="row">${s.toUpperCase()}</th><td>${EHt(x.name)}</td><td class="num">${x.precursor == null ? "" : text(x.precursor)}</td><td class="num">${x.ce == null ? EHt(words(x.ce_values)) : text(x.ce)}</td>
    <td class="num">${x.scans ?? ""}</td><td class="num">${x.n_bins}</td><td class="num">${(x.explained * 100).toFixed(1)} %</td></tr>`; };
  const un = (/** @type {any[]} */ rows) => rows.length ? `<div class="tbl"><table><tr><th class="num">${t("spcmp.col.mz")}</th><th class="num">${t("spcmp.col.int")}</th></tr>${rows.map(r => `<tr><td class="num">${text(r.mz)}</td><td class="num">${text(r.y)}</td></tr>`).join("")}</table></div>` : `<p class="sm muted">${t("spcmp.none")}</p>`;
  const p = doc.params, dflt = (/** @type {string} */ k) => (p.default.includes(k) ? ` (${t("spcmp.default")})` : "");
  return `<div class="cos" role="status">${t("spcmp.cosine")} ${doc.cosine.toFixed(4)}</div>
    <p class="sm">${t("spcmp.summary", { n: doc.n_matched, tol: text(p.tol), ce: text(p.ce_tol) })} <span class="muted">${EHt(dflt("tol") + dflt("ce_tol"))}</span></p>
    ${warns ? `<ul>${warns}</ul>` : ""}
    <div class="tbl"><table><caption class="sr-only">${t("spcmp.sides")}</caption><tr><th></th><th>${t("spcmp.col.file")}</th><th class="num">${t("spcmp.col.prec")}</th><th class="num">${t("spcmp.col.ce")}</th><th class="num">${t("spcmp.col.scans")}</th><th class="num">${t("spcmp.col.bins")}</th><th class="num">${t("spcmp.col.explained")}</th></tr>${sideRow("a")}${sideRow("b")}</table></div>
    <h4>${t("spcmp.matched")}</h4>${doc.matched.length ? `<div class="tbl"><table><tr><th class="num">${t("spcmp.col.mzA")}</th><th class="num">${t("spcmp.col.mzB")}</th><th class="num">${t("spcmp.col.intA")}</th><th class="num">${t("spcmp.col.intB")}</th></tr>${doc.matched.map((/** @type {any} */ m) => `<tr><td class="num">${text(m.mz_a)}</td><td class="num">${text(m.mz_b)}</td><td class="num">${text(m.a)}</td><td class="num">${text(m.b)}</td></tr>`).join("")}</table></div>` : `<p class="sm muted">${t("spcmp.none")}</p>`}
    <div class="lists"><div><h4>${t("spcmp.onlyA")}</h4>${un(doc.only_a)}</div><div><h4>${t("spcmp.onlyB")}</h4>${un(doc.only_b)}</div></div>`;
}

/** Opens the card on the MS² spectrum panel `p`: side A is its selection, side B the same selection in another file of the tab when there is one. @param {any} p */
function open(p) {
  const files = filesOf(p).filter((/** @type {any} */ f) => precursorsOf(f).length);
  if (!files.length) return W_.toast ? W_.toast(t("spcmp.noMs2")) : undefined;
  let d = /** @type {HTMLDialogElement | null} */ (document.getElementById("spcmp"));
  if (!d) { d = document.createElement("dialog"); d.id = "spcmp"; d.setAttribute("aria-labelledby", "spcmp-h"); document.body.appendChild(d); }
  if (!document.getElementById("spcmp-css")) { const s = document.createElement("div"); s.id = "spcmp-css"; s.innerHTML = CSS; document.head.appendChild(/** @type {Node} */ (s.firstChild)); }
  const dlg = d, own = files.find((/** @type {any} */ f) => f.k === p.k) || files[0], other = files.find((/** @type {any} */ f) => f !== own) || own;
  const near = (/** @type {number[]} */ ps, /** @type {number} */ v) => ps.reduce((m, x) => (Math.abs(x - v) < Math.abs(m - v) ? x : m), ps[0]);
  const from = Number.isFinite(+p.r0) ? +p.r0 : 0, to = Number.isFinite(+p.r1) ? +p.r1 : from, prec0 = Number.isFinite(+p.prec) && p.prec !== "" ? +p.prec : null;
  /** @param {string} s @param {any} f */
  const sideHtml = (s, f) => h`<fieldset data-s="${s}"><legend>${s === "a" ? t("spcmp.side.a") : t("spcmp.side.b")}</legend>
      <label>${t("spcmp.file")} <select data-f="file">${files.map((/** @type {any} */ x) => raw(h`<option value="${x.k}"${x === f ? " selected" : ""}>${x.label || x.file}</option>`))}</select></label>
      <label>${t("spcmp.prec")} <select data-f="prec"></select></label>
      <label>${t("spcmp.time")} <input data-f="rt0" data-num="0:" value="${+from.toFixed(4)}" aria-label="${t("spcmp.time.from")}"> – <input data-f="rt1" data-num="0:" value="${+to.toFixed(4)}" aria-label="${t("spcmp.time.to")}"> ${t("spcmp.unit.min")}</label></fieldset>`;
  dlg.innerHTML = h`<div style="display:flex;align-items:center"><h3 id="spcmp-h" style="margin:0;flex:1">${t("spcmp.title")}</h3><button type="button" class="x" id="spcmp-x" aria-label="${t("common.close")}" title="${t("common.close")}">&times;</button></div>
    <div class="sides">${raw(sideHtml("a", own))}${raw(sideHtml("b", other))}</div>
    <div class="qbar"><label>${t("spcmp.tol")} <input id="spcmp-tol" data-num="0: v" placeholder="0" title="${t("spcmp.tol.tip")}"> ${t("spcmp.unit.da")}</label><label>${t("spcmp.ceTol")} <input id="spcmp-cetol" data-num="0: v" placeholder="0.5" title="${t("spcmp.ceTol.tip")}"> ${t("spcmp.unit.ev")}</label>
      <button type="button" id="spcmp-go">${t("spcmp.go")}</button></div><div id="spcmp-msg" class="sm muted" aria-live="polite"></div><div id="spcmp-out"></div>`;
  // the precursors of the chosen file, the one nearest to the wanted m/z selected
  const fill = (/** @type {string} */ s, /** @type {number | null} */ want) => {
    const sel = /** @type {HTMLSelectElement} */ (dlg.querySelector(`[data-s="${s}"] [data-f="prec"]`)), f = E.files[+/** @type {HTMLSelectElement} */ (dlg.querySelector(`[data-s="${s}"] [data-f="file"]`)).value], ps = precursorsOf(f);
    const pick = want != null && ps.length ? near(ps, want) : ps[0];
    sel.innerHTML = h`${ps.map(v => raw(h`<option value="${v}"${v === pick ? " selected" : ""}>${text(v)}</option>`))}`;
  };
  for (const s of ["a", "b"]) {
    fill(s, prec0);
    /** @type {HTMLSelectElement} */ (dlg.querySelector(`[data-s="${s}"] [data-f="file"]`)).onchange = () => fill(s, parseFloat(/** @type {HTMLSelectElement} */ (dlg.querySelector(`[data-s="${s}"] [data-f="prec"]`)).value));
  }
  const msg = (/** @type {string} */ m, bad = false) => { const e = /** @type {HTMLElement} */ (dlg.querySelector("#spcmp-msg")); e.textContent = m; e.classList.toggle("err", bad); };
  const out = /** @type {HTMLElement} */ (dlg.querySelector("#spcmp-out"));
  const go = async () => {
    const a = readSide(dlg, "a"), b = readSide(dlg, "b");
    if (![a.prec, a.a, a.b, b.prec, b.a, b.b].every(Number.isFinite)) return msg(t("spcmp.check"), true);
    // only the parameters the user typed: the engine logs the ones that took their default
    /** @type {Record<string, number>} */ const par = {};
    for (const [id, k] of [["#spcmp-tol", "tol"], ["#spcmp-cetol", "ce_tol"]]) { const v = /** @type {HTMLInputElement} */ (dlg.querySelector(id)).value.trim(); if (v) par[k] = numIt(v); }
    if (Object.values(par).some(v => !Number.isFinite(v) || v < 0)) return msg(t("spcmp.check"), true);
    msg(t("spcmp.working")); out.innerHTML = "";
    try {
      const sides = `{"a":${await sideOf(a)},"b":${await sideOf(b)}}`;
      out.innerHTML = show(JSON.parse(await engine(JSON.stringify(par), sides)));
      msg("");
    } catch (e_) { msg(t("spcmp.error", { message: /** @type {Error} */ (e_).message }), true); }
  };
  /** @type {HTMLElement} */ (dlg.querySelector("#spcmp-go")).onclick = go;
  /** @type {HTMLElement} */ (dlg.querySelector("#spcmp-x")).onclick = () => dlg.close();
  dlg.querySelectorAll("input").forEach(i => { i.onkeydown = ev => { if (ev.key === "Enter") { i.blur(); i.focus(); if (i.getAttribute("aria-invalid") !== "true") go(); } }; });   // the blur lets the number gate refuse first
  dlg.showModal();
}

W_.SPCMP = { open };
