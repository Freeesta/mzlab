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
  #spcmp label{display:flex;gap:6px;align-items:center;flex-wrap:wrap}#spcmp input[data-num]{width:84px}#spcmp input[data-f=off]{width:150px}#spcmp .qbar{display:flex;gap:12px;flex-wrap:wrap;align-items:center;margin:6px 0}
  #spcmp table{border-collapse:collapse;width:100%}#spcmp td,#spcmp th{padding:3px 8px;border-bottom:1px solid var(--line);text-align:left;font-size:12px}#spcmp td.num,#spcmp th.num{text-align:right;font-variant-numeric:tabular-nums}
  #spcmp .tbl{max-height:240px;overflow:auto;border:1px solid var(--line);border-radius:6px}#spcmp .cos{font-size:28px;font-weight:600;font-variant-numeric:tabular-nums}#spcmp .warn{color:var(--warn)}#spcmp .err{color:var(--bad)}
  #spcmp .lists{display:flex;gap:12px;flex-wrap:wrap}#spcmp .lists>div{flex:1 1 280px}#spcmp canvas.mirror{display:block;width:100%;height:280px;border:1px solid var(--line);border-radius:6px;margin:8px 0 4px}</style>`;

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

/** One side as the card holds it: file, precursor, time window and m/z offset text (empty = none; the route reads and checks it). @param {HTMLElement} d @param {string} s */
const readSide = (d, s) => {
  const q = (/** @type {string} */ f) => /** @type {HTMLInputElement} */ (d.querySelector(`[data-s="${s}"] [data-f="${f}"]`));
  const f = E.files[+q("file").value], prec = parseFloat(q("prec").value), a = numIt(q("rt0").value), b = numIt(q("rt1").value);
  return { f, prec, a: Math.min(a, b), b: Math.max(a, b), off: q("off").value.trim() };
};

/** The route's answer for one side, with what the file states about the scans, as the `{"a": .., "b": ..}` member of the engine's request.
 * @param {{ f: any, prec: number, a: number, b: number, off: string }} x */
async function sideOf(x) {
  const query = `k=${x.f.k}&rt0=${x.a}&rt1=${x.b}&level=2&precursor=${x.prec}&rebin=nominal${x.off ? "&mzoff=" + encodeURIComponent(x.off) : ""}`;
  const body = await (await fetch("api/spectrum?" + query)).text();
  let j = null; try { j = JSON.parse(body); } catch (_) { /* not JSON: the engine is asked below and says so */ }
  if (j && (j.error || j.error_key)) throw W_.I18N.err(j);
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

/** The bars of the mirror from the document: A above the axis, B below, each as a share of its own base peak; `m` marks a matched ion (both sides draw it, joined).
 * @param {any} doc @returns {{ side: string, mz: number, y: number, rel: number, m: boolean }[]} */
const barsOf = doc => {
  const top = (/** @type {number[]} */ ys) => maxOf(ys, 0) || 1, ta = top([...doc.matched.map((/** @type {any} */ r) => r.a), ...doc.only_a.map((/** @type {any} */ r) => r.y)]), tb = top([...doc.matched.map((/** @type {any} */ r) => r.b), ...doc.only_b.map((/** @type {any} */ r) => r.y)]);
  return [...doc.matched.map((/** @type {any} */ r) => ({ side: "a", mz: r.mz_a, y: r.a, rel: r.a / ta, m: true })), ...doc.only_a.map((/** @type {any} */ r) => ({ side: "a", mz: r.mz, y: r.y, rel: r.y / ta, m: false })),
    ...doc.matched.map((/** @type {any} */ r) => ({ side: "b", mz: r.mz_b, y: r.b, rel: r.b / tb, m: true })), ...doc.only_b.map((/** @type {any} */ r) => ({ side: "b", mz: r.mz, y: r.y, rel: r.y / tb, m: false }))];
};

/** Draws the mirror on the canvas `cv`: matched ions in one colour with a dot on the tip and a line between the two bars, the ions only one side has thinner in another colour
 * (Okabe-Ito blue and vermilion, so the two differ by width and the dot as well as by hue). What is drawn is also in `cv.dataset.bars` for the test, and the data table is `TDATI`'s.
 * @param {HTMLCanvasElement} cv @param {any} doc */
function mirror(cv, doc) {
  const bars = barsOf(doc), A = bars.filter(b => b.side === "a"), B = bars.filter(b => b.side === "b"), nm = doc.matched.length;
  cv.setAttribute("aria-label", t("spcmp.mirror.aria"));
  W_.TDATI.describe(cv, t("spcmp.mirror.desc", { a: A.length, b: B.length, n: nm }));
  W_.TDATI.addButton(cv, () => ({ title: t("spcmp.mirror.aria"), head: [t("spcmp.mirror.col.side"), t("spcmp.mirror.col.state"), t("spcmp.col.mz"), t("spcmp.col.int"), t("tdati.col.rel")], rowHeader: true,
    rows: bars.map(b => [b.side.toUpperCase(), t(b.m ? "spcmp.mirror.matched" : "spcmp.mirror.only"), W_.TDATI.num(b.mz, 4), W_.TDATI.num(b.y, 4), W_.TDATI.num(b.rel * 100, 1)]) }));
  cv.dataset.bars = JSON.stringify(bars.map(b => [b.side, b.mz, +b.rel.toFixed(4), b.m ? 1 : 0]));
  if (!bars.length) return;
  const s = setup(cv), g = s.g, W = s.W, H = s.H, ml = 44, mr = 14, mt = 22, mb = 30, mid = mt + (H - mt - mb) / 2, half = (H - mt - mb) / 2 - 14;
  const mz = bars.map(b => b.mz), lo = Math.floor(minOf(mz) / 10) * 10 - 5, hi = Math.ceil(maxOf(mz) / 10) * 10 + 5, X = (/** @type {number} */ m) => ml + (m - lo) / (hi - lo) * (W - ml - mr);
  const hit = css("--lv1"), miss = css("--lv3b"), ink = css("--ink"), mut = css("--muted"), line = css("--line");
  g.clearRect(0, 0, W, H); g.fillStyle = css("--panel"); g.fillRect(0, 0, W, H);
  g.font = fpx(12);
  // the line between the two bars of a matched ion
  g.save(); g.globalAlpha = 0.55; g.lineWidth = 1; g.strokeStyle = hit;
  doc.matched.forEach((/** @type {any} */ r, /** @type {number} */ i) => { g.beginPath(); g.moveTo(X(r.mz_a), mid - A[i].rel * half); g.lineTo(X(r.mz_b), mid + B[i].rel * half); g.stroke(); });
  g.restore();
  for (const b of bars) {
    const dir = b.side === "a" ? 1 : -1, x = X(b.mz), tip = mid - dir * b.rel * half, c = b.m ? hit : miss;
    g.strokeStyle = c; g.fillStyle = c; g.lineWidth = b.m ? 2.5 : 1.3; g.beginPath(); g.moveTo(x, mid); g.lineTo(x, tip); g.stroke();
    if (b.m) { g.beginPath(); g.arc(x, tip, 3, 0, 2 * Math.PI); g.fill(); }
    if (b.rel > 0.08) { g.textAlign = "center"; g.fillText(text(b.mz), x, dir > 0 ? tip - 6 : tip + 15); }
  }
  g.strokeStyle = line; g.lineWidth = 1; g.beginPath(); g.moveTo(ml, mid); g.lineTo(W - mr, mid); g.stroke();
  g.strokeStyle = mut; g.fillStyle = mut; g.textAlign = "center"; g.beginPath(); g.moveTo(ml, H - mb + 6); g.lineTo(W - mr, H - mb + 6); g.stroke();
  const step = [1, 2, 5, 10, 20, 25, 50, 100, 200].find(v => (hi - lo) / v <= 9) || 500;
  for (let v = Math.ceil(lo / step) * step; v <= hi; v += step) { g.beginPath(); g.moveTo(X(v), H - mb + 6); g.lineTo(X(v), H - mb + 10); g.stroke(); g.fillText(String(v), X(v), H - mb + 22); }
  g.textAlign = "right"; for (const [v, y] of [["100", mid - half], ["50", mid - half / 2], ["0", mid], ["50", mid + half / 2], ["100", mid + half]]) g.fillText(String(v), ml - 6, +y + 4);
  g.fillStyle = ink; g.textAlign = "left"; g.fillText("A · " + (doc.a.name || ""), ml + 4, mt - 6); g.fillText("B · " + (doc.b.name || ""), ml + 4, H - mb - 2);
  g.textAlign = "right"; g.fillStyle = hit; g.fillText(t("spcmp.mirror.key.matched", { n: nm }), W - mr - 4, mt - 6); g.fillStyle = miss; g.fillText(t("spcmp.mirror.key.only"), W - mr - 4, H - mb - 2);
  g.save(); g.fillStyle = mut; g.textAlign = "center"; g.translate(12, mid); g.rotate(-Math.PI / 2); g.fillText(t("spcmp.mirror.rel"), 0, 0); g.restore();
  /** @type {any} */ (cv)._mg = { X, mid, half };
}

/** @param {any} doc */
function show(doc) {
  const side = (/** @type {string} */ s) => doc[s];
  const warns = doc.warnings.map((/** @type {any} */ w) => `<li class="warn">${EHt(t(w.key, Object.fromEntries(Object.entries(w).map(([k, v]) => [k, words(v)]))))}</li>`).join("");
  const sideRow = (/** @type {string} */ s) => { const x = side(s); return `<tr><th scope="row">${s.toUpperCase()}</th><td>${EHt(x.name)}</td><td class="num">${x.precursor == null ? "" : text(x.precursor)}</td><td class="num">${x.ce == null ? EHt(words(x.ce_values)) : text(x.ce)}</td>
    <td class="num">${x.scans ?? ""}</td><td class="num">${x.n_bins}</td><td class="num">${(x.explained * 100).toFixed(1)} %</td></tr>`; };
  const un = (/** @type {any[]} */ rows) => rows.length ? `<div class="tbl"><table><tr><th class="num">${t("spcmp.col.mz")}</th><th class="num">${t("spcmp.col.int")}</th></tr>${rows.map(r => `<tr><td class="num">${text(r.mz)}</td><td class="num">${text(r.y)}</td></tr>`).join("")}</table></div>` : `<p class="sm muted">${t("spcmp.none")}</p>`;
  /** the offset a side used: Da, and for an `auto` estimate the ion, the scans, the spread and the status, with the engine's warning text */
  const evid = (/** @type {string} */ s) => {
    const x = side(s), e = x.mz_offset || {}, who = s.toUpperCase(), da = text(x.mz_offset_da);
    if (e.mode !== "auto") return `<li>${EHt(t("spcmp.off.used", { side: who, da }))}</li>`;
    const w = e.status === "not_found" ? "warn.mz.offset.notFound" : e.status === "wide" ? "warn.mz.offset.wide" : "";
    const detail = [e.apex_rt_min == null ? "" : t("spcmp.off.apex", { rt: text(e.apex_rt_min) }), e.spread_da == null ? "" : t("spcmp.off.spread", { da: text(e.spread_da) }),
      t("spcmp.off.scans", { n: e.n_scans }), t("spcmp.off.snr", { snr: text(e.snr) })].filter(Boolean).join(t("spcmp.off.sep"));
    const status = t(e.status === "ok" ? "spcmp.off.status.ok" : e.status === "wide" ? "spcmp.off.status.wide" : "spcmp.off.status.missing");
    const line = t(e.rt_min == null ? "spcmp.off.auto" : "spcmp.off.autoNear", { side: who, da, mz: text(e.ion_mz), rt: e.rt_min == null ? "" : text(e.rt_min), status, detail });
    return `<li${w ? ' class="warn"' : ""}>${EHt(w ? t("spcmp.off.warned", { line, warn: t(w) }) : line)}</li>`;
  };
  const p = doc.params, dflt = (/** @type {string} */ k) => (p.default.includes(k) ? ` (${t("spcmp.default")})` : "");
  return `<div class="cos" role="status">${t("spcmp.cosine")} ${doc.cosine.toFixed(4)}</div>
    <p class="sm">${t("spcmp.summary", { n: doc.n_matched, tol: text(p.tol), ce: text(p.ce_tol) })} <span class="muted">${EHt(dflt("tol") + dflt("ce_tol"))}</span></p>
    ${warns ? `<ul>${warns}</ul>` : ""}
    <canvas class="mirror" id="spcmp-cv"></canvas>
    <h4>${t("spcmp.off.h")}</h4><ul class="sm" id="spcmp-offs">${evid("a")}${evid("b")}</ul>
    <div class="tbl"><table><caption class="sr-only">${t("spcmp.sides")}</caption><tr><th></th><th>${t("spcmp.col.file")}</th><th class="num">${t("spcmp.col.prec")}</th><th class="num">${t("spcmp.col.ce")}</th><th class="num">${t("spcmp.col.scans")}</th><th class="num">${t("spcmp.col.bins")}</th><th class="num">${t("spcmp.col.explained")}</th></tr>${sideRow("a")}${sideRow("b")}</table></div>
    <h4>${t("spcmp.matched")}</h4>${doc.matched.length ? `<div class="tbl"><table><tr><th class="num">${t("spcmp.col.mzA")}</th><th class="num">${t("spcmp.col.mzB")}</th><th class="num">${t("spcmp.col.intA")}</th><th class="num">${t("spcmp.col.intB")}</th></tr>${doc.matched.map((/** @type {any} */ m) => `<tr><td class="num">${text(m.mz_a)}</td><td class="num">${text(m.mz_b)}</td><td class="num">${text(m.a)}</td><td class="num">${text(m.b)}</td></tr>`).join("")}</table></div>` : `<p class="sm muted">${t("spcmp.none")}</p>`}
    <div class="lists"><div><h4>${t("spcmp.onlyA")}</h4>${un(doc.only_a)}</div><div><h4>${t("spcmp.onlyB")}</h4>${un(doc.only_b)}</div></div>`;
}

/** the drawing of the open card, run again when the window is resized (the canvas takes the width of the card) */
let redraw = () => {};
window.addEventListener("resize", () => redraw());

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
      <label>${t("spcmp.off")} <input data-f="off" placeholder="${t("spcmp.off.ph")}" title="${t("spcmp.off.tip")}" autocomplete="off" spellcheck="false"></label>
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
  /** @type {any} */ let last = null;
  const draw = () => { const cv = /** @type {HTMLCanvasElement | null} */ (dlg.querySelector("#spcmp-cv")); if (last && cv && dlg.open) mirror(cv, last); };
  redraw = draw;
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
      const doc = JSON.parse(await engine(JSON.stringify(par), sides));
      out.innerHTML = show(doc); last = doc; draw();
      msg("");
    } catch (e_) { msg(t("spcmp.error", { message: /** @type {Error} */ (e_).message }), true); }
  };
  /** @type {HTMLElement} */ (dlg.querySelector("#spcmp-go")).onclick = go;
  /** @type {HTMLElement} */ (dlg.querySelector("#spcmp-x")).onclick = () => dlg.close();
  dlg.querySelectorAll("input").forEach(i => { i.onkeydown = ev => { if (ev.key === "Enter") { i.blur(); i.focus(); if (i.getAttribute("aria-invalid") !== "true") go(); } }; });   // the blur lets the number gate refuse first
  dlg.showModal();
}

W_.SPCMP = { open };
