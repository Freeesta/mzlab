// @ts-strict
"use strict";
// «Composizione elementare» in the style of Xcalibur: the compact panel of the drawer (Formule ▾ → Composizione elementare), the table of the elements in use and the
// periodic table that the expanded form of composizione.js (COMP.open) also uses, and the right-click entry «Genera formula» of a high-resolution spectrum.
// The numbers come from the server (composition.py through /api/composition); here there are only the forms. Results are CANDIDATES, never identifications.
// High-resolution world only. Classic script, loaded after composizione.js (it uses COMP at run time).
const ELEM = (() => {
  const css0 = document.createElement("style");
  css0.textContent = `.cx{display:flex;flex-direction:column;gap:6px}.cx-row{display:flex;flex-wrap:wrap;gap:4px 10px;align-items:center}.cx-row label{display:inline-flex;align-items:center;gap:4px}
.cx input[type=text],.cx input[type=number]{width:64px;padding:2px 5px;min-width:0}.cx select{padding:2px 4px;min-width:0;max-width:100%}
.cx-sec{font-weight:600;font-size:12px;margin-top:2px;padding-top:4px;border-top:1px solid var(--line)}
.cx-out{max-height:210px;overflow:auto;border:1px solid var(--line);border-radius:6px;background:var(--panel)}
.cx-out table,.cx-els{border-collapse:collapse;width:100%}.cx-out th,.cx-out td,.cx-els th,.cx-els td{padding:1px 6px;text-align:left;white-space:nowrap;font-size:11px;line-height:1.5}
.cx-out th,.cx-els th{position:sticky;top:0;background:var(--soft);z-index:1}.cx-out td.num,.cx-out th.num,.cx-els td.num,.cx-els th.num{text-align:right}
.cx-out tr[data-i]{cursor:pointer}.cx-out tr[data-i]:hover{background:var(--sel)}.cx-out tr[aria-selected=true]{background:var(--sel);font-weight:600;outline:1px solid var(--accent);outline-offset:-1px}
.cx-els input{width:44px!important;text-align:right}.cx-els td button{padding:0 6px}.cx-add{display:flex;gap:4px;margin-top:3px;align-items:center}
.cx-btns{display:flex;flex-wrap:wrap;gap:4px;align-items:center}.cx-btns button{padding:2px 8px}.cx-btns select{flex:1 1 90px}
.cx-pt{display:grid;grid-template-columns:repeat(18,minmax(0,1fr));gap:2px;margin-top:4px}.cx-pt button{padding:1px 0;font-size:11px;min-width:0;line-height:1.4}.cx-pt button.on{background:var(--accent);color:#fff;border-color:var(--accent)}.cx-pt button:disabled{opacity:.3}`;
  document.head.appendChild(css0);
  const t = (k, p) => I18N.t(k, p);
  const SUPPORTED = ["C", "H", "N", "O", "S", "P", "F", "Cl", "Br", "I", "Si", "Na", "K", "Se"];   // the elements of composition.py (Python MASS)
  const ISOT = ["2H", "13C", "15N", "18O", "34S", "37Cl", "81Br"];                                  // isotopes that can be an element on their own (Python ISOTOPES)
  const DBEQ = { C: 1, H: -0.5, N: 0.5, O: 0, S: 0, P: 0.5, F: -0.5, Cl: -0.5, Br: -0.5, I: -0.5, Si: 1, Na: -0.5, K: -0.5, Se: 0 };      // ring and double bond equivalents (composition.py DBEQ)
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
  // «Applica»: the formula (a candidate) is written under the m/z of the matching peak of the spectrum panel
  function applyTo(sp, r, q) {
    if (!sp) return nearMsg(t("bn.needSpec"));
    const d = sp._a && sp._a.data && sp._a.data[0] && sp._a.data[0].d, m = peakFor(d, r.mz_calc, q.tol, q.unit);
    if (m == null) return nearMsg(t("cx.applyNone"));
    sp.fm = (sp.fm || []).filter(z => Math.abs(z.m - m) > 1e-4).concat({ m, text: r.formula });
    draw(sp); nearMsg(t("cx.applied", { f: r.formula }));
  }
  const copyText = async s => { try { await navigator.clipboard.writeText(s); } catch (e) { const a = document.createElement("textarea"); a.value = s; document.body.appendChild(a); a.select(); try { document.execCommand("copy"); } catch (e2) { /* no clipboard */ } a.remove(); } };

  // ---------------------------------------------------------------- the compact panel of the drawer
  let pending = null;
  const takeCtx = () => { const c = pending; pending = null; return c; };
  function panel(b, ctx) {
    const C = COMP, st0 = C.rd(), pol = ctx.polarity === "negative" ? "-" : "+";
    const opt = (v, cur, l) => `<option value="${EH(v)}"${v === cur ? " selected" : ""}>${l}</option>`;
    const presets = { ...C.BUILTIN, ...(st0.presets || {}) };
    b.innerHTML = `<div class="cx">
      <div class="cx-row"><label>${t("cx.mass")} <input id="cx-mz" type="text" inputmode="decimal" style="width:96px" value="${ctx.mz != null ? (+ctx.mz).toFixed(5) : ""}"></label><label>${t("comp.max")} <input id="cx-max" type="number" min="1" max="200" value="${st0.max ?? 10}"></label></div>
      <div id="cx-out" class="cx-out" aria-live="polite"></div>
      <div class="cx-sec">${t("cx.limits")}</div>
      <div class="cx-row"><label>${t("cx.charge")} <select id="cx-ion">${C.IONS.filter(i => (pol === "+" ? !/-$/.test(i[0]) : /-$/.test(i[0]))).map(i => opt(i[0], st0["ion" + pol] || (pol === "+" ? "[M+H]+" : "[M-H]-"), i[1])).join("")}</select></label>
        <label>${t("comp.nitrogen")} <select id="cx-n">${opt("none", st0.n || "none", t("comp.n.none"))}${opt("even", st0.n || "none", t("comp.n.even"))}${opt("odd", st0.n || "none", t("comp.n.odd"))}</select></label></div>
      <div class="cx-row"><label>${t("comp.tol")} <input id="cx-tol" type="number" min="0" step="any" value="${st0.tol ?? 5}"> <select id="cx-unit">${opt("ppm", st0.unit || "ppm", "ppm")}${opt("mda", st0.unit || "ppm", "mDa")}</select></label>
        <label>${t("comp.col.rdb")} <input id="cx-r0" type="number" step="any" value="${st0.r0 ?? -1}" aria-label="${t("comp.rdbFrom")}"> – <input id="cx-r1" type="number" step="any" value="${st0.r1 ?? 100}" aria-label="${t("comp.rdbTo")}"></label></div>
      <div class="cx-sec">${t("comp.elements")}</div>
      <textarea id="cx-els" hidden>${EH(st0.els || DEFAULT_ELS)}</textarea><div id="cx-elt"></div>
      <div class="cx-btns"><select id="cx-pre" aria-label="${t("comp.load")}">${Object.keys(presets).map(k => `<option>${EH(k)}</option>`).join("")}</select><button id="cx-load" type="button">${t("comp.load")}</button><button id="cx-save" type="button">${t("comp.saveAs")}</button></div>
      <div class="cx-btns"><button id="cx-apply" type="button" class="go">${t("cx.apply")}</button><button id="cx-help" type="button">${t("cx.help")}</button><button id="cx-exp" type="button" aria-label="${t("cx.expand")}" title="${t("cx.expand")}">⤢</button></div>
      <div class="muted sm">${t("cx.note")}</div></div>`;
    const $ = s => b.querySelector(s), ta = $("#cx-els"); let res = null, sel = 0, tm = 0, seq = 0;
    const read = () => ({ mz: C.num($("#cx-mz").value), ion: $("#cx-ion").value, tol: C.num($("#cx-tol").value) ?? 5, unit: $("#cx-unit").value, max: Math.max(1, parseInt($("#cx-max").value) || 10),
      n: $("#cx-n").value, r0: C.num($("#cx-r0").value) ?? -1, r1: C.num($("#cx-r1").value) ?? 100, els: ta.value.replace(/\s+/g, "") });
    const show = () => {
      const o = $("#cx-out");
      if (!res.results.length) { o.innerHTML = `<div class="muted sm" style="padding:6px">${t("comp.none")}</div>`; return; }
      const sc = res.results.some(r => r.score);
      o.innerHTML = `<table aria-label="${t("comp.title")}"><thead><tr><th>${t("cx.idx")}</th><th>${t("cx.formula")}</th><th class="num">${t("comp.col.rdb")}</th><th class="num">Δ ppm</th>${sc ? `<th class="num">${t("cx.score")}</th>` : ""}</tr></thead><tbody>
        ${res.results.map((r, i) => `<tr data-i="${i}" tabindex="0" aria-selected="${i === sel}"><td>${i + 1}</td><td><b>${fmtFormula(r.formula)}</b></td><td class="num">${r.rdb}</td><td class="num">${r.delta_ppm}</td>${sc ? `<td class="num">${r.score ? (+r.score.total).toFixed(2) : ""}</td>` : ""}</tr>`).join("")}</tbody></table>`;
      const rows = [...o.querySelectorAll("tr[data-i]")];
      const pick = (i, copy) => { sel = i; rows.forEach((x, j) => x.setAttribute("aria-selected", String(j === i))); if (copy) { const r = res.results[i]; copyText(`${r.formula}\t${r.rdb}\t${r.delta_ppm}`); nearMsg(t("cx.copied", { f: r.formula })); } };
      rows.forEach((tr, i) => { tr.onclick = () => pick(i, true); tr.onkeydown = ev => {
        if (((ev.ctrlKey || ev.metaKey) && ev.key.toLowerCase() === "c") || ev.key === "Enter" || ev.key === " ") { ev.preventDefault(); pick(i, true); }
        else if (ev.key === "ArrowDown" && rows[i + 1]) { ev.preventDefault(); rows[i + 1].focus(); pick(i + 1, false); }
        else if (ev.key === "ArrowUp" && rows[i - 1]) { ev.preventDefault(); rows[i - 1].focus(); pick(i - 1, false); } }; });
    };
    const run = async () => {
      const v = read(), o = $("#cx-out"), my = ++seq;
      if (v.mz == null || !(v.mz > 0)) { res = null; o.innerHTML = `<div class="muted sm" style="padding:6px">${t("cx.noMz")}</div>`; return; }
      C.wr({ ...C.rd(), tol: v.tol, unit: v.unit, max: v.max, n: v.n, r0: v.r0, r1: v.r1, els: ta.value, ["ion" + pol]: v.ion });
      const s = C.rd(), q = { mz: v.mz, ion: v.ion, tol: v.tol, unit: v.unit, max: v.max, n: v.n, rdb: `${v.r0},${v.r1}`, elements: v.els, rules: (s.rules || []).join(",") };
      C.scoreParams(q, ctx.spec ? C.prepSpec(ctx.spec, s) : null, "", s);
      try {
        const j = await J("api/composition?" + C.qs(q));
        if (my !== seq) return;
        res = C.afterFit(j, s); sel = 0; show();
      } catch (e) { if (my === seq) o.innerHTML = `<span class="fail" style="padding:6px">${EH(e.message)}</span>`; }
    };
    const later = () => { clearTimeout(tm); tm = setTimeout(run, 250); };
    ["#cx-mz", "#cx-max", "#cx-ion", "#cx-n", "#cx-tol", "#cx-unit", "#cx-r0", "#cx-r1"].forEach(s => { $(s).oninput = later; $(s).onchange = later; });
    bindEls($("#cx-elt"), ta, later);
    $("#cx-load").onclick = () => { ta.value = presets[$("#cx-pre").value] || ta.value; ta.dispatchEvent(new Event("input")); later(); };
    $("#cx-save").onclick = async () => { const n = await ask(t("comp.presetName"), ""); if (!n) return; const s = C.rd(); s.presets = { ...(s.presets || {}), [n]: ta.value }; C.wr(s); toast(t("comp.saved")); };
    $("#cx-apply").onclick = () => { if (!res || !res.results.length) return nearMsg(t("comp.none")); applyTo(ctx.sp, res.results[sel], read()); };
    $("#cx-help").onclick = () => big(t("cx.help.title"), `<div style="white-space:pre-line">${EH(t("cx.help.body"))}</div>`);
    $("#cx-exp").onclick = () => { const v = read(); C.open({ mz: v.mz, spec: ctx.spec, polarity: ctx.polarity, sp: ctx.sp, ion: v.ion }); };
    run();
  }
  // right click on a peak of a high-resolution spectrum → «Genera formula»: the panel opens with that m/z and the last parameters
  function generate(o) {
    pending = { mz: o.mz, sp: o.sp, spec: o.spec, polarity: o.polarity };
    if (window.BARRA) BARRA.setTab("cmp");
    if (window.BANCO) BANCO.renderInfo();
  }
  return { panel, generate, takeCtx, bindEls, bindPeriodic, DEFAULT_ELS, parse, ser, applyTo, peakFor };
})();
window.ELEM = ELEM;
