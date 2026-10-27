// @ts-strict
// @ts-check
"use strict";
// The compact «Elemental composition» panel of the drawer (Formule ▾ → Composizione elementare, body `tCmp` of banco.js): mass and number of results, the table of
// candidates (one cell = one copyable value), the limits and the elements in use. The numbers come from /api/composition; the elements table, the apply-to-spectrum
// step and the copy helper are in elementare.js (ELEM). Results are CANDIDATES, never identifications. Module, loaded by carica.js after elementare.js.

// at most CX_MAX candidates are asked for, CX_DEF by default; the table shows six rows (header included) and scrolls beyond them
const CX_MAX = 10, CX_DEF = 5;
const cxCss = document.createElement("style");
cxCss.textContent = `.cx{display:flex;flex-direction:column;gap:var(--sp-4);min-width:0}.cx-title{font-size:14px;margin:0}
.cx-sec{font-weight:600;font-size:12px;margin:var(--sp-2) 0 0;padding-top:var(--sp-3);border-top:1px solid var(--line)}
.cx-grid{display:grid;grid-template-columns:minmax(7em,40%) minmax(0,1fr);gap:var(--sp-3) var(--sp-4);align-items:center}.cx-grid>label{color:var(--muted)}
.cx input[type=text],.cx input[type=number],.cx select{box-sizing:border-box;width:100%;min-width:0;min-height:var(--cx-row);padding:2px 5px}.cx input[type=number]{text-align:right}
.cx-pair{display:flex;gap:var(--sp-3);align-items:center;min-width:0}.cx-pair>*{flex:1 1 0}.cx-pair>span{flex:0 0 auto}
.cx button{min-height:var(--cx-row);min-width:var(--cx-row);padding:2px 8px}.cx-btns{display:flex;flex-wrap:wrap;gap:var(--sp-3);align-items:center}.cx-set{padding-bottom:var(--sp-3)}
.cx-out{max-height:calc(var(--cx-row) * 6 + 2px);overflow:auto;border:1px solid var(--line);border-radius:6px;background:var(--panel)}
.cx-out table{border-collapse:collapse;width:100%}.cx-out th,.cx-out td{padding:0 6px;text-align:left;white-space:nowrap;font-size:11px;height:var(--cx-row)}
.cx-out th{position:sticky;top:0;background:var(--soft);z-index:1;font-weight:600}.cx-out td.num,.cx-out th.num{text-align:right;font-variant-numeric:tabular-nums}
.cx-out tr[data-i]{cursor:pointer}.cx-out tr[data-i]:hover{background:var(--sel)}.cx-out tr[aria-selected=true]{background:var(--sel);font-weight:600;outline:1px solid var(--accent);outline-offset:-1px}
.cx-out td[data-copy]{cursor:copy}.cx-out td:focus-visible,.cx-out tr:focus-visible{outline:2px solid var(--accent);outline-offset:-2px}.cx-out th[title]{cursor:help}`;
document.head.appendChild(cxCss);

/** @param {string} k @param {any} [p] */
const cxT = (k, p) => I18N.t(k, p);
const cxOpt = (/** @type {string} */ v, /** @type {string} */ cur, /** @type {string} */ l) => `<option value="${EH(v)}"${v === cur ? " selected" : ""}>${l}</option>`;
/** The settings of the score as the tooltip of its column and the description of the table tell them. */
const cxScoreTip = (/** @type {any} */ s) => cxT("cx.score.tip", { s: s.sppm ?? 3, w: [s.w0 ?? 1, s.w1 ?? 2, s.w2 ?? 1].join(" : ") });

/** The form: title, mass and number of results, the (empty) results, limits and elements in use. */
/** @param {string} key */
const cxNote = key => `<div class="muted sm" style="padding:6px">${cxT(key)}</div>`;
/** @param {string} msg */
const cxFail = msg => `<span class="fail" style="padding:6px">${EH(msg)}</span>`;

function cxHtml(/** @type {any} */ st0, /** @type {any} */ ctx, /** @type {string} */ pol, /** @type {Record<string, string>} */ presets) {
  const ions = COMP.IONS.filter((/** @type {string[]} */ i) => !pol || (pol === "+" ? !/-$/.test(i[0]) : /-$/.test(i[0])));
  const ion0 = st0["ion" + (pol || "+")] || (pol === "-" ? "[M-H]-" : "[M+H]+");
  return `<div class="cx">
      <h3 class="cx-title">${cxT("cx.title")}</h3>
      <div class="cx-grid"><label for="cx-mz">${cxT("cx.mass")}</label><input id="cx-mz" type="text" inputmode="decimal" value="${ctx.mz != null ? (+ctx.mz).toFixed(5) : ""}">
        <label for="cx-max">${cxT("comp.max")}</label><input id="cx-max" type="number" min="1" max="${CX_MAX}" value="${Math.min(CX_MAX, Math.max(1, st0.cxmax ?? CX_DEF))}"></div>
      <h4 class="cx-sec" id="cx-h-res">${cxT("cx.results")}</h4>
      <div id="cx-out" class="cx-out" role="region" aria-labelledby="cx-h-res" aria-live="polite"></div>
      <span id="cx-desc" class="sr-only"></span><span id="cx-live" class="sr-only" role="status"></span>
      <h4 class="cx-sec">${cxT("cx.limits")}</h4>
      <div class="cx-grid"><label for="cx-ion">${cxT("cx.charge")}</label><select id="cx-ion">${ions.map((/** @type {string[]} */ i) => cxOpt(i[0], ion0, i[1])).join("")}</select>
        <label for="cx-n">${cxT("comp.nitrogen")}</label><select id="cx-n">${["none", "even", "odd"].map(v => cxOpt(v, st0.n || "none", cxT("comp.n." + v))).join("")}</select>
        <label for="cx-tol">${cxT("comp.tol")}</label><span class="cx-pair"><input id="cx-tol" type="number" min="0" step="any" value="${st0.tol ?? 5}"><select id="cx-unit" aria-label="${cxT("comp.tol")}">${cxOpt("ppm", st0.unit || "ppm", "ppm")}${cxOpt("mda", st0.unit || "ppm", "mDa")}</select></span>
        <label for="cx-r0" id="cx-rl">${cxT("comp.col.rdb")}</label><span class="cx-pair" role="group" aria-labelledby="cx-rl"><input id="cx-r0" type="number" step="any" value="${st0.r0 ?? -1}" aria-label="${cxT("comp.rdbFrom")}"><span aria-hidden="true">–</span><input id="cx-r1" type="number" step="any" value="${st0.r1 ?? 100}" aria-label="${cxT("comp.rdbTo")}"></span></div>
      <h4 class="cx-sec">${cxT("comp.elements")}</h4>
      <div class="cx-grid cx-set"><label for="cx-pre">${cxT("cx.set")}</label><select id="cx-pre">${Object.keys(presets).map(k => `<option>${EH(k)}</option>`).join("")}</select></div>
      <div class="cx-btns cx-set"><button id="cx-load" type="button" title="${cxT("cx.load.tip")}">${cxT("comp.load")}</button><button id="cx-save" type="button" title="${cxT("cx.save.tip")}">${cxT("comp.saveAs")}</button></div>
      <textarea id="cx-els" hidden>${EH(st0.els || ELEM.DEFAULT_ELS)}</textarea><div id="cx-elt"></div>
      <div class="cx-btns"><button id="cx-apply" type="button" class="go">${cxT("cx.apply")}</button><button id="cx-exp" type="button" aria-label="${cxT("cx.expand")}" title="${cxT("cx.expand")}">⤢</button></div></div>`;
}

/** The results table: Δ in the unit asked for in Tolerance, Score only when there is one; every value cell carries the text it copies. */
function cxTableHtml(/** @type {any} */ res, /** @type {number} */ sel, /** @type {any} */ s) {
  const sc = res.results.some((/** @type {any} */ r) => r.score), mda = res.unit === "mda";
  const dl = (/** @type {any} */ r) => String(mda ? r.delta_mda : r.delta_ppm), tot = (/** @type {any} */ r) => (r.score ? (+r.score.total).toFixed(2) : "");
  const cell = (/** @type {string} */ cls, /** @type {string} */ v, /** @type {string} */ html) => `<td${cls ? ` class="${cls}"` : ""} data-copy="${EH(v)}" tabindex="-1">${html}</td>`;
  return `<table aria-labelledby="cx-h-res" aria-describedby="cx-desc"><thead><tr><th>${cxT("cx.idx")}</th><th title="${EH(cxT("cx.desc"))}">${cxT("cx.formula")}</th><th class="num">${cxT("comp.col.rdb")}</th><th class="num">${cxT(mda ? "cx.delta.mda" : "cx.delta.ppm")}</th>${sc ? `<th class="num" title="${EH(cxScoreTip(s))}">${cxT("cx.score")}</th>` : ""}</tr></thead><tbody>
    ${res.results.map((/** @type {any} */ r, /** @type {number} */ i) => `<tr data-i="${i}" tabindex="0" aria-selected="${i === sel}"><td>${i + 1}</td>${cell("", r.formula, `<b>${fmtFormula(r.formula)}</b>`)}${cell("num", String(r.rdb), String(r.rdb))}${cell("num", dl(r), dl(r))}${sc ? cell("num", tot(r), tot(r)) : ""}</tr>`).join("")}</tbody></table>`;
}

/** Clicks and keys of the table: a cell copies its own value, the row (Ctrl+C, Enter, Space) the formula; arrows move between rows and cells. `st` = { res, sel }. */
function cxBindTable(/** @type {HTMLElement} */ o, /** @type {{ res: any, sel: number }} */ st, /** @type {(v: string) => void} */ copied) {
  const rows = /** @type {HTMLElement[]} */ ([...o.querySelectorAll("tr[data-i]")]);
  const pick = (/** @type {number} */ i) => { st.sel = i; rows.forEach((x, j) => x.setAttribute("aria-selected", String(j === i))); };
  const cellsOf = (/** @type {Element} */ tr) => /** @type {HTMLElement[]} */ ([...tr.querySelectorAll("td[tabindex]")]);
  const cellOf = (/** @type {EventTarget | null} */ t, /** @type {string} */ sel) => /** @type {HTMLElement | null} */ ((/** @type {Element} */ (t)).closest(sel));
  rows.forEach((tr, i) => {
    tr.onclick = ev => { pick(i); const c = cellOf(ev.target, "td[data-copy]"), v = c ? c.dataset.copy : st.res.results[i].formula; if (v) copied(v); };
    tr.onkeydown = ev => {
      const c = cellOf(ev.target, "td[tabindex]"), cs = cellsOf(tr), k = c ? cs.indexOf(c) : -1;
      if (((ev.ctrlKey || ev.metaKey) && ev.key.toLowerCase() === "c") || ev.key === "Enter" || ev.key === " ") { ev.preventDefault(); pick(i); const v = c ? c.dataset.copy : st.res.results[i].formula; if (v) copied(v); }
      else if (ev.key === "ArrowDown" && rows[i + 1]) { ev.preventDefault(); (k >= 0 ? cellsOf(rows[i + 1])[k] : rows[i + 1]).focus(); pick(i + 1); }
      else if (ev.key === "ArrowUp" && rows[i - 1]) { ev.preventDefault(); (k >= 0 ? cellsOf(rows[i - 1])[k] : rows[i - 1]).focus(); pick(i - 1); }
      else if (ev.key === "ArrowRight" && k < cs.length - 1) { ev.preventDefault(); cs[k + 1].focus(); }
      else if (ev.key === "ArrowLeft" && k >= 0) { ev.preventDefault(); (k > 0 ? cs[k - 1] : tr).focus(); }
    };
  });
}

/** Fills `b` with the panel of the drawer; `ctx` = { mz, spec, polarity, sp } (the peak, its spectrum, the polarity of the file, the spectrum panel). */
function cxPanel(/** @type {HTMLElement} */ b, /** @type {any} */ ctx) {
  const C = COMP, t = cxT, st0 = C.rd(), pol = ctx.polarity === "negative" ? "-" : ctx.polarity === "positive" ? "+" : "";
  const presets = { ...C.BUILTIN, ...(st0.presets || {}) };
  b.innerHTML = cxHtml(st0, ctx, pol, presets);
  const $ = (/** @type {string} */ s) => /** @type {any} */ (b.querySelector(s)), ta = $("#cx-els"), st = { res: /** @type {any} */ (null), sel: 0 };
  let tm = 0, seq = 0, busy = false, again = false, lastSig = "";
  const num = (/** @type {string} */ id) => parseInt($(id).value);
  const read = () => ({ mz: C.num($("#cx-mz").value), ion: $("#cx-ion").value, tol: C.num($("#cx-tol").value) ?? 5, unit: $("#cx-unit").value, max: Math.min(CX_MAX, Math.max(1, Number.isNaN(num("#cx-max")) ? CX_DEF : num("#cx-max"))),
    n: $("#cx-n").value, r0: C.num($("#cx-r0").value) ?? -1, r1: C.num($("#cx-r1").value) ?? 100, els: ta.value.replace(/\s+/g, "") });
  // what a screen reader hears: the value copied
  const copied = (/** @type {string} */ v) => { ELEM.copyText(v); $("#cx-live").textContent = ""; setTimeout(() => { $("#cx-live").textContent = t("cx.copied", { f: v }); }, 30); nearMsg(t("cx.copied", { f: v })); };
  const show = () => {
    const o = $("#cx-out"), res = st.res, s = C.rd();
    if (!res.results.length) { $("#cx-desc").textContent = ""; o.innerHTML = cxNote("comp.none"); return; }
    $("#cx-desc").textContent = t("cx.desc") + " " + (res.results.some((/** @type {any} */ r) => r.score) ? cxScoreTip(s) : t("cx.sort.err"));
    o.innerHTML = cxTableHtml(res, st.sel, s); cxBindTable(o, st, copied);
  };
  const run = async () => {
    const v = read(), o = $("#cx-out");
    if (v.mz == null || !(v.mz > 0)) { st.res = null; lastSig = ""; $("#cx-desc").textContent = ""; o.innerHTML = cxNote("cx.noMz"); return; }
    const sig = JSON.stringify(v) + (C.rd().rules || []); if (sig === lastSig) return;
    // one request at a time: while the engine works the last change waits, and only the newest input is sent afterwards
    if (busy) { again = true; return; }
    busy = true; lastSig = sig; const my = ++seq;
    C.wr({ ...C.rd(), tol: v.tol, unit: v.unit, cxmax: v.max, n: v.n, r0: v.r0, r1: v.r1, els: ta.value, ["ion" + (pol || "+")]: v.ion });
    const s = C.rd(), q = { mz: v.mz, ion: v.ion, tol: v.tol, unit: v.unit, max: v.max, n: v.n, rdb: `${v.r0},${v.r1}`, elements: v.els, rules: (s.rules || []).join(",") };
    C.scoreParams(q, ctx.spec ? C.prepSpec(ctx.spec, s) : null, "", s);
    o.setAttribute("aria-busy", "true");
    try {
      const j = await J("api/composition?" + C.qs(q));
      if (my === seq && !again) { st.res = C.afterFit(j, s); st.sel = 0; show(); }
    } catch (e_) { const e = /** @type {Lanciato} */ (e_); lastSig = ""; if (my === seq) o.innerHTML = cxFail(e.message); }
    busy = false; o.removeAttribute("aria-busy");
    if (again) { again = false; run(); }
  };
  const later = (ms = 120) => { clearTimeout(tm); tm = setTimeout(run, ms); };
  const now = () => { clearTimeout(tm); run(); };
  // typing waits a moment; Enter, leaving the field and the menus calculate at once
  ["#cx-mz", "#cx-max", "#cx-tol", "#cx-r0", "#cx-r1"].forEach(s => { $(s).oninput = () => later(); $(s).onchange = now; });
  ["#cx-ion", "#cx-n", "#cx-unit"].forEach(s => { $(s).onchange = now; });
  $("#cx-mz").onkeydown = (/** @type {KeyboardEvent} */ ev) => { if (ev.key === "Enter") now(); };
  $("#cx-max").onchange = () => { $("#cx-max").value = String(read().max); now(); };
  ELEM.bindEls($("#cx-elt"), ta, later);
  $("#cx-load").onclick = () => { ta.value = presets[$("#cx-pre").value] || ta.value; ta.dispatchEvent(new Event("input")); now(); };
  $("#cx-save").onclick = async () => { const n = await ask(t("comp.presetName"), ""); if (!n) return; const s = C.rd(); s.presets = { ...(s.presets || {}), [n]: ta.value }; C.wr(s); toast(t("comp.saved")); };
  $("#cx-apply").onclick = () => { if (!st.res || !st.res.results.length) return nearMsg(t("comp.none")); ELEM.applyTo(ctx.sp, st.res.results[st.sel], read()); };
  $("#cx-exp").onclick = () => { const v = read(); C.open({ mz: v.mz, spec: ctx.spec, polarity: ctx.polarity, sp: ctx.sp, ion: v.ion }); };
  run();
}
window.ELEMP = { panel: cxPanel };
