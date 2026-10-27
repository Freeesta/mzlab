// @ts-strict
"use strict";
// «Composizione elementare» in the style of Xcalibur: the compact panel of the drawer (Formule ▾ → Composizione elementare), the table of the elements in use and the
// periodic table that the expanded form of composizione.js (COMP.open) also uses, and the right-click entry «Genera formula» of a high-resolution spectrum.
// The numbers come from the server (composition.py through /api/composition); here there are only the forms. Results are CANDIDATES, never identifications.
// High-resolution world only. Classic script, loaded after composizione.js (it uses COMP at run time).
const ELEM = (() => {
  const css0 = document.createElement("style");
  css0.textContent = `.cx,.cx-els{--cx-row:24px}.cx{display:flex;flex-direction:column;gap:var(--sp-4);min-width:0}.cx-title{font-size:14px;margin:0}
.cx-sec{font-weight:600;font-size:12px;margin:var(--sp-2) 0 0;padding-top:var(--sp-3);border-top:1px solid var(--line)}
.cx-grid{display:grid;grid-template-columns:minmax(7em,40%) minmax(0,1fr);gap:var(--sp-3) var(--sp-4);align-items:center}.cx-grid>label{color:var(--muted)}
.cx input[type=text],.cx input[type=number],.cx select{box-sizing:border-box;width:100%;min-width:0;min-height:var(--cx-row);padding:2px 5px}.cx input[type=number]{text-align:right}
.cx-pair{display:flex;gap:var(--sp-3);align-items:center;min-width:0}.cx-pair>*{flex:1 1 0}.cx-pair>span{flex:0 0 auto}
.cx button{min-height:var(--cx-row);min-width:var(--cx-row);padding:2px 8px}
.cx-out{max-height:calc(var(--cx-row) * 6 + 2px);overflow:auto;border:1px solid var(--line);border-radius:6px;background:var(--panel)}
.cx-out table,.cx-els{border-collapse:collapse;width:100%}.cx-out th,.cx-out td,.cx-els th,.cx-els td{padding:0 6px;text-align:left;white-space:nowrap;font-size:11px;height:var(--cx-row)}
.cx-out th,.cx-els th{position:sticky;top:0;background:var(--soft);z-index:1;font-weight:600}.cx-out td.num,.cx-out th.num,.cx-els td.num,.cx-els th.num{text-align:right;font-variant-numeric:tabular-nums}
.cx-out tr[data-i]{cursor:pointer}.cx-out tr[data-i]:hover{background:var(--sel)}.cx-out tr[aria-selected=true]{background:var(--sel);font-weight:600;outline:1px solid var(--accent);outline-offset:-1px}
.cx-out td[data-copy]{cursor:copy}.cx-out td:focus-visible,.cx-out tr:focus-visible{outline:2px solid var(--accent);outline-offset:-2px}.cx-out th[title]{cursor:help}
.cx-els{table-layout:fixed}.cx-els th:nth-child(1){width:17%}.cx-els th:nth-child(2),.cx-els th:nth-child(3){width:19%}.cx-els th:nth-child(4){width:15%}.cx-els th:nth-child(5){width:21%}.cx-els th:last-child{width:calc(var(--cx-row) + 4px)}
.cx-els th,.cx-els td{padding:0 3px}.cx-els input{width:100%!important;min-width:0;box-sizing:border-box;text-align:right}.cx-els td button{padding:0;width:var(--cx-row)}.cx-els td:last-child{text-align:center}
.cx-add{display:grid;grid-template-columns:minmax(0,1fr) auto;gap:var(--sp-3);margin-top:var(--sp-3);padding-top:var(--sp-3);border-top:1px dashed var(--line);align-items:center}
.cx-btns{display:flex;flex-wrap:wrap;gap:var(--sp-3);align-items:center}.cx-set{padding-bottom:var(--sp-3)}
.cx-pt{display:grid;grid-template-columns:repeat(18,minmax(0,1fr));gap:2px;margin-top:4px}.cx-pt button{padding:1px 0;font-size:11px;min-width:0;line-height:1.4}.cx-pt button.on{background:var(--accent);color:var(--on-accent);border-color:var(--accent)}.cx-pt button:disabled{opacity:.3}
@media (pointer:coarse){.cx,.cx-els{--cx-row:40px}}`;
  document.head.appendChild(css0);
  const t = (k, p) => I18N.t(k, p);
  // the elements of composition.py (Python MASS)
  const SUPPORTED = ["C", "H", "N", "O", "S", "P", "F", "Cl", "Br", "I", "Si", "Na", "K", "Se"];
  // isotopes that can be an element on their own (Python ISOTOPES)
  const ISOT = ["2H", "13C", "15N", "18O", "34S", "37Cl", "81Br"];
  // ring and double bond equivalents (composition.py DBEQ)
  const DBEQ = { C: 1, H: -0.5, N: 0.5, O: 0, S: 0, P: 0.5, F: -0.5, Cl: -0.5, Br: -0.5, I: -0.5, Si: 1, Na: -0.5, K: -0.5, Se: 0 };
  const DEFAULT_ELS = "C:0-13,H:0-25,N:0-4,O:0-3,S:0-1";
  const SUPD = "⁰¹²³⁴⁵⁶⁷⁸⁹";
  const parse = s => String(s || "").split(/[,;]/).map(x => x.trim()).filter(Boolean).map(x => { const m = /^(\d*[A-Z][a-z]?)\s*:\s*(\d+)(?:\s*-\s*(\d+))?$/.exec(x); return m ? { s: m[1], n0: +m[2], n1: m[3] != null ? +m[3] : +m[2] } : { raw: x }; });
  const ser = rows => rows.map(r => (r.raw != null ? r.raw : `${r.s}:${r.n0}-${r.n1}`)).join(",");
  const baseOf = s => (/^\d+([A-Z][a-z]?)$/.exec(s) || [0, s])[1];
  // nominal mass and exact mass of an element (most abundant isotope) or of a labelled isotope ("13C"), from the table of the page (elements.js)
  const isoInfo = s => {
    const m = /^(\d+)([A-Z][a-z]?)$/.exec(s), e = typeof ELEMENTS !== "undefined" ? ELEMENTS.find(x => x.s === (m ? m[2] : s)) : null;
    if (!e || !e.iso || !e.iso.length) return null;
    const it = m ? e.iso.find(i => i[0] === +m[1]) : e.iso.reduce((a, b) => (b[2] > a[2] ? b : a));
    return it && it[1] != null ? { nom: it[0], mass: it[1] } : null;
  };
  const lab = s => { const i = isoInfo(s); return `<sup>${i ? i.nom : ""}</sup>${EH(baseOf(s))}`; };
  const optLabel = s => { const i = isoInfo(s); return (i ? String(i.nom).replace(/\d/g, d => SUPD[d]) : "") + baseOf(s); };

  // ---------------------------------------------------------------- table of the elements in use: the text "C:0-13,H:0-25" (a textarea) is the value that is saved, the table edits it
  function bindEls(host, ta, onch) {
    const draw = () => {
      const rows = parse(ta.value), used = new Set(rows.map(r => r.s));
      host.innerHTML = `<table class="cx-els"><thead><tr><th>${t("cx.col.iso")}</th><th class="num">${t("cx.col.min")}</th><th class="num">${t("cx.col.max")}</th><th class="num">${t("cx.col.db")}</th><th class="num">${t("cx.col.mass")}</th><th></th></tr></thead><tbody>
        ${rows.map((r, i) => r.raw != null ? `<tr><td colspan="5" class="muted">${EH(r.raw)}</td><td><button type="button" data-del="${i}" aria-label="${t("cx.remove", { el: EH(r.raw) })}">×</button></td></tr>` : (() => {
          const ii = isoInfo(r.s);
          return `<tr><td><b>${lab(r.s)}</b></td><td class="num"><input type="number" min="0" data-i="${i}" data-f="min" value="${r.n0}" aria-label="${t("cx.col.min")} ${EH(r.s)}"></td><td class="num"><input type="number" min="0" data-i="${i}" data-f="max" value="${r.n1}" aria-label="${t("cx.col.max")} ${EH(r.s)}"></td>
            <td class="num">${DBEQ[baseOf(r.s)] != null ? DBEQ[baseOf(r.s)] : ""}</td><td class="num">${ii ? ii.mass.toFixed(5) : ""}</td><td><button type="button" data-del="${i}" aria-label="${t("cx.remove", { el: EH(r.s) })}">×</button></td></tr>`;
        })()).join("")}</tbody></table>
        <div class="cx-add"><select aria-label="${t("cx.add")}">${[...SUPPORTED, ...ISOT].filter(s => !used.has(s)).map(s => `<option value="${s}">${optLabel(s)}</option>`).join("")}</select><button type="button" data-add>${t("cx.add")}</button></div>`;
      const put = () => { ta.value = ser(rows); if (onch) onch(); };
      host.querySelectorAll("input[data-i]").forEach(inp => { inp.oninput = () => { const r = /** @type {any} */ (rows[+inp.dataset.i]), v = Math.max(0, parseInt(inp.value) || 0), f = inp.dataset.f; r[f === "min" ? "n0" : "n1"] = v; if (r.n1 < r.n0) { if (f === "min") r.n1 = v; else r.n0 = v; } put(); }; });
      host.querySelectorAll("[data-del]").forEach(b => { b.onclick = () => { rows.splice(+b.dataset.del, 1); put(); draw(); }; });
      host.querySelector("[data-add]").onclick = () => { const s = host.querySelector(".cx-add select").value; if (!s) return; rows.push({ s, n0: 0, n1: 2 }); put(); draw(); };
    };
    ta.addEventListener("input", draw); draw();
  }
  // periodic table (custom): the elements of the server; a click puts one in / takes it out of the elements in use
  function bindPeriodic(host, ta, onch) {
    const draw = () => {
      const used = new Set(parse(ta.value).map(r => r.s)), EE = typeof ELEMENTS !== "undefined" ? ELEMENTS : [];
      host.innerHTML = `<div class="cx-pt" role="group" aria-label="${t("cx.pt")}">${EE.filter(e => e.r <= 7).map(e => `<button type="button" data-s="${e.s}" style="grid-row:${e.r};grid-column:${e.c}" class="${used.has(e.s) ? "on" : ""}" aria-pressed="${used.has(e.s)}" ${SUPPORTED.includes(e.s) ? "" : "disabled"} title="${EH(I18N.t(`el.${e.s}`))}">${e.s}</button>`).join("")}</div><div class="muted sm">${t("cx.pt.hint")}</div>`;
      host.querySelectorAll("button[data-s]").forEach(b => {
        b.onclick = () => { const rows = parse(ta.value), s = b.dataset.s, i = rows.findIndex(r => r.s === s); if (i >= 0) rows.splice(i, 1); else rows.push({ s, n0: 0, n1: s === "C" ? 30 : s === "H" ? 60 : 4 }); ta.value = ser(rows); ta.dispatchEvent(new Event("input")); if (onch) onch(); };
      });
    };
    ta.addEventListener("input", draw); draw();
  }

  // ---------------------------------------------------------------- candidates → peaks of the spectrum
  // the most intense point of the spectrum d = {mz, y} inside the tolerance window of the calculated m/z of a candidate; null when there is none
  function peakFor(d, mzCalc, tol, unit) {
    if (!d || !d.mz) return null;
    const w = unit === "mda" ? tol / 1000 : mzCalc * tol * 1e-6; let b = -1;
    d.mz.forEach((m, i) => { if (Math.abs(m - mzCalc) <= w && (b < 0 || d.y[i] > d.y[b])) b = i; });
    return b < 0 ? null : d.mz[b];
  }
  // «Apply»: the formula (a candidate) is written under the m/z of the matching peak of the spectrum panel
  function applyTo(sp, r, q) {
    if (!sp) return nearMsg(t("bn.needSpec"));
    const d = sp._a && sp._a.data && sp._a.data[0] && sp._a.data[0].d, m = peakFor(d, r.mz_calc, q.tol, q.unit);
    if (m == null) return nearMsg(t("cx.applyNone"));
    sp.fm = (sp.fm || []).filter(z => Math.abs(z.m - m) > 1e-4).concat({ m, text: r.formula });
    draw(sp); nearMsg(t("cx.applied", { f: r.formula }));
  }
  const copyText = async s => { try { await navigator.clipboard.writeText(s); } catch (e) { const a = document.createElement("textarea"); a.value = s; document.body.appendChild(a); a.select(); try { document.execCommand("copy"); } catch (e2) { /* no clipboard */ } a.remove(); } };

  // ---------------------------------------------------------------- the compact panel of the drawer
  // the panel lists at most MAX_RES candidates, DEF_RES by default (the table scrolls beyond the visible rows)
  const MAX_RES = 10, DEF_RES = 5;
  let pending = null;
  const takeCtx = () => { const c = pending; pending = null; return c; };
  function panel(b, ctx) {
    const C = COMP, st0 = C.rd(), pol = ctx.polarity === "negative" ? "-" : ctx.polarity === "positive" ? "+" : "";
    const opt = (v, cur, l) => `<option value="${EH(v)}"${v === cur ? " selected" : ""}>${l}</option>`;
    const presets = { ...C.BUILTIN, ...(st0.presets || {}) };
    const ions = C.IONS.filter(i => !pol || (pol === "+" ? !/-$/.test(i[0]) : /-$/.test(i[0])));
    const ion0 = st0["ion" + (pol || "+")] || (pol === "-" ? "[M-H]-" : "[M+H]+");
    b.innerHTML = `<div class="cx">
      <h3 class="cx-title">${t("cx.title")}</h3>
      <div class="cx-grid"><label for="cx-mz">${t("cx.mass")}</label><input id="cx-mz" type="text" inputmode="decimal" value="${ctx.mz != null ? (+ctx.mz).toFixed(5) : ""}">
        <label for="cx-max">${t("comp.max")}</label><input id="cx-max" type="number" min="1" max="${MAX_RES}" value="${Math.min(MAX_RES, Math.max(1, st0.cxmax ?? DEF_RES))}"></div>
      <h4 class="cx-sec" id="cx-h-res">${t("cx.results")}</h4>
      <div id="cx-out" class="cx-out" role="region" aria-labelledby="cx-h-res" aria-live="polite"></div>
      <span id="cx-desc" class="sr-only"></span><span id="cx-live" class="sr-only" role="status"></span>
      <h4 class="cx-sec">${t("cx.limits")}</h4>
      <div class="cx-grid"><label for="cx-ion">${t("cx.charge")}</label><select id="cx-ion">${ions.map(i => opt(i[0], ion0, i[1])).join("")}</select>
        <label for="cx-n">${t("comp.nitrogen")}</label><select id="cx-n">${opt("none", st0.n || "none", t("comp.n.none"))}${opt("even", st0.n || "none", t("comp.n.even"))}${opt("odd", st0.n || "none", t("comp.n.odd"))}</select>
        <label for="cx-tol">${t("comp.tol")}</label><span class="cx-pair"><input id="cx-tol" type="number" min="0" step="any" value="${st0.tol ?? 5}"><select id="cx-unit" aria-label="${t("comp.tol")}">${opt("ppm", st0.unit || "ppm", "ppm")}${opt("mda", st0.unit || "ppm", "mDa")}</select></span>
        <label for="cx-r0" id="cx-rl">${t("comp.col.rdb")}</label><span class="cx-pair" role="group" aria-labelledby="cx-rl"><input id="cx-r0" type="number" step="any" value="${st0.r0 ?? -1}" aria-label="${t("comp.rdbFrom")}"><span aria-hidden="true">–</span><input id="cx-r1" type="number" step="any" value="${st0.r1 ?? 100}" aria-label="${t("comp.rdbTo")}"></span></div>
      <h4 class="cx-sec">${t("comp.elements")}</h4>
      <div class="cx-grid cx-set"><label for="cx-pre">${t("cx.set")}</label><select id="cx-pre">${Object.keys(presets).map(k => `<option>${EH(k)}</option>`).join("")}</select></div>
      <div class="cx-btns cx-set"><button id="cx-load" type="button" title="${t("cx.load.tip")}">${t("comp.load")}</button><button id="cx-save" type="button" title="${t("cx.save.tip")}">${t("comp.saveAs")}</button></div>
      <textarea id="cx-els" hidden>${EH(st0.els || DEFAULT_ELS)}</textarea><div id="cx-elt"></div>
      <div class="cx-btns"><button id="cx-apply" type="button" class="go">${t("cx.apply")}</button><button id="cx-exp" type="button" aria-label="${t("cx.expand")}" title="${t("cx.expand")}">⤢</button></div></div>`;
    const $ = s => b.querySelector(s), ta = $("#cx-els"); let res = null, sel = 0, tm = 0, seq = 0, busy = false, again = false, lastSig = "";
    const read = () => ({ mz: C.num($("#cx-mz").value), ion: $("#cx-ion").value, tol: C.num($("#cx-tol").value) ?? 5, unit: $("#cx-unit").value,
      max: Math.min(MAX_RES, Math.max(1, Number.isNaN(parseInt($("#cx-max").value)) ? DEF_RES : parseInt($("#cx-max").value))), n: $("#cx-n").value, r0: C.num($("#cx-r0").value) ?? -1, r1: C.num($("#cx-r1").value) ?? 100, els: ta.value.replace(/\s+/g, "") });
    // what the screen reader hears: the value copied, or the note on the table
    const say = m => { $("#cx-live").textContent = ""; setTimeout(() => { $("#cx-live").textContent = m; }, 30); };
    const copied = v => { copyText(v); say(t("cx.copied", { f: v })); nearMsg(t("cx.copied", { f: v })); };
    const show = () => {
      const o = $("#cx-out"), d = $("#cx-desc");
      if (!res.results.length) { d.textContent = ""; o.innerHTML = `<div class="muted sm" style="padding:6px">${t("comp.none")}</div>`; return; }
      const sc = res.results.some(r => r.score), mda = res.unit === "mda", s = C.rd(), tip = t("cx.score.tip", { s: s.sppm ?? 3, w: [s.w0 ?? 1, s.w1 ?? 2, s.w2 ?? 1].join(" : ") });
      d.textContent = t("cx.desc") + " " + (sc ? tip : t("cx.sort.err"));
      const dl = r => String(mda ? r.delta_mda : r.delta_ppm), cell = (cls, v, html) => `<td${cls ? ` class="${cls}"` : ""} data-copy="${EH(v)}" tabindex="-1">${html}</td>`;
      o.innerHTML = `<table aria-labelledby="cx-h-res" aria-describedby="cx-desc"><thead><tr><th>${t("cx.idx")}</th><th title="${EH(t("cx.desc"))}">${t("cx.formula")}</th><th class="num">${t("comp.col.rdb")}</th><th class="num">${t(mda ? "cx.delta.mda" : "cx.delta.ppm")}</th>${sc ? `<th class="num" title="${EH(tip)}">${t("cx.score")}</th>` : ""}</tr></thead><tbody>
        ${res.results.map((r, i) => `<tr data-i="${i}" tabindex="0" aria-selected="${i === sel}"><td>${i + 1}</td>${cell("", r.formula, `<b>${fmtFormula(r.formula)}</b>`)}${cell("num", String(r.rdb), String(r.rdb))}${cell("num", dl(r), dl(r))}${sc ? cell("num", r.score ? (+r.score.total).toFixed(2) : "", r.score ? (+r.score.total).toFixed(2) : "") : ""}</tr>`).join("")}</tbody></table>`;
      const rows = [...o.querySelectorAll("tr[data-i]")];
      const pick = i => { sel = i; rows.forEach((x, j) => x.setAttribute("aria-selected", String(j === i))); };
      // a cell copies its own value; the row (and Ctrl+C or Enter on it) copies the formula
      const cellsOf = (/** @type {Element} */ tr) => /** @type {HTMLElement[]} */ ([...tr.querySelectorAll("td[tabindex]")]);
      rows.forEach((tr, i) => {
        tr.onclick = ev => { pick(i); const c = /** @type {HTMLElement|null} */ ((/** @type {Element} */ (ev.target)).closest("td[data-copy]")), v = c ? c.dataset.copy : res.results[i].formula; if (v) copied(v); };
        tr.onkeydown = ev => {
          const c = /** @type {HTMLElement|null} */ ((/** @type {Element} */ (ev.target)).closest("td[tabindex]")), cs = cellsOf(tr), k = c ? cs.indexOf(c) : -1;
          if (((ev.ctrlKey || ev.metaKey) && ev.key.toLowerCase() === "c") || ev.key === "Enter" || ev.key === " ") { ev.preventDefault(); pick(i); const v = c ? c.dataset.copy : res.results[i].formula; if (v) copied(v); }
          else if (ev.key === "ArrowDown" && rows[i + 1]) { ev.preventDefault(); (k >= 0 ? cellsOf(rows[i + 1])[k] : rows[i + 1]).focus(); pick(i + 1); }
          else if (ev.key === "ArrowUp" && rows[i - 1]) { ev.preventDefault(); (k >= 0 ? cellsOf(rows[i - 1])[k] : rows[i - 1]).focus(); pick(i - 1); }
          else if (ev.key === "ArrowRight" && k < cs.length - 1) { ev.preventDefault(); cs[k + 1].focus(); }
          else if (ev.key === "ArrowLeft" && k >= 0) { ev.preventDefault(); (k > 0 ? cs[k - 1] : tr).focus(); }
        };
      });
    };
    const run = async () => {
      const v = read(), o = $("#cx-out");
      if (v.mz == null || !(v.mz > 0)) { res = null; lastSig = ""; $("#cx-desc").textContent = ""; o.innerHTML = `<div class="muted sm" style="padding:6px">${t("cx.noMz")}</div>`; return; }
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
        if (my === seq && !again) { res = C.afterFit(j, s); sel = 0; show(); }
      } catch (e_) { const e = /** @type {Lanciato} */ (e_); lastSig = ""; if (my === seq) o.innerHTML = `<span class="fail" style="padding:6px">${EH(e.message)}</span>`; }
      busy = false; o.removeAttribute("aria-busy");
      if (again) { again = false; run(); }
    };
    const later = (ms = 120) => { clearTimeout(tm); tm = setTimeout(run, ms); };
    const now = () => { clearTimeout(tm); run(); };
    // typing waits a moment; Enter, leaving the field and the menus calculate at once
    ["#cx-mz", "#cx-max", "#cx-tol", "#cx-r0", "#cx-r1"].forEach(s => { $(s).oninput = () => later(); $(s).onchange = now; });
    ["#cx-ion", "#cx-n", "#cx-unit"].forEach(s => { $(s).onchange = now; });
    $("#cx-mz").onkeydown = ev => { if (ev.key === "Enter") now(); };
    $("#cx-max").onchange = () => { $("#cx-max").value = String(read().max); now(); };
    bindEls($("#cx-elt"), ta, later);
    $("#cx-load").onclick = () => { ta.value = presets[$("#cx-pre").value] || ta.value; ta.dispatchEvent(new Event("input")); now(); };
    $("#cx-save").onclick = async () => { const n = await ask(t("comp.presetName"), ""); if (!n) return; const s = C.rd(); s.presets = { ...(s.presets || {}), [n]: ta.value }; C.wr(s); toast(t("comp.saved")); };
    $("#cx-apply").onclick = () => { if (!res || !res.results.length) return nearMsg(t("comp.none")); applyTo(ctx.sp, res.results[sel], read()); };
    $("#cx-exp").onclick = () => { const v = read(); C.open({ mz: v.mz, spec: ctx.spec, polarity: ctx.polarity, sp: ctx.sp, ion: v.ion }); };
    run();
  }
  // right click on a peak of a high-resolution spectrum → «Genera formula»: the panel opens with that m/z and the last parameters
  function generate(o) {
    pending = { mz: o.mz, sp: o.sp, spec: o.spec, polarity: o.polarity };
    if (window.BARRA) BARRA.setTab("cmp");
    if (window.BANCO) BANCO.renderInfo();
  }
  // the actions of the spectrum menu (registry azioni/1, high resolution): «Generate formula» for the peak, clear the formulas drawn
  if (typeof AZIONI !== "undefined") {
    AZIONI.registra({ id: "spec.generate", gruppo: "gen", ordine: 1, viste: ["spec"], visibile: c => !!c.a.hrp, disponibile: c => c.has, nome: c => I18N.t("ctx.spec.generate", { mz: c.lab }),
      esegui: c => generate({ mz: c.m, sp: c.p, spec: c.a.data[0].d, polarity: c.pol }) });
    AZIONI.registra({ id: "spec.fmClear", gruppo: "gen", ordine: 2, viste: ["spec"], visibile: c => !!c.a.hrp, disponibile: c => !!(c.p.fm && c.p.fm.length), nome: () => I18N.t("cx.clear"), esegui: c => { c.p.fm = []; draw(c.p); } });
  }
  return { panel, generate, takeCtx, bindEls, bindPeriodic, DEFAULT_ELS, parse, ser, applyTo, peakFor };
})();
window.ELEM = ELEM;
