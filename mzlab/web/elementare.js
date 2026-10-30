// @ts-strict
"use strict";
// «Composizione elementare» in the style of Xcalibur: the table of the elements in use and the
// periodic table that the expanded form of composizione.js (COMP.open) also uses, and the right-click entry «Genera formula» of a high-resolution spectrum.
// The numbers come from the server (composition.py through /api/composition); here there are only the forms. Results are CANDIDATES, never identifications.
// High-resolution world only. Classic script, loaded after composizione.js (it uses COMP at run time).
const ELEM = (() => {
  const css0 = document.createElement("style");
  css0.textContent = `.cx,.cx-els{--cx-row:24px}
.cx-els{border-collapse:collapse;width:100%;table-layout:fixed}.cx-els th,.cx-els td{padding:0 3px;text-align:left;white-space:nowrap;font-size:11px;height:var(--cx-row)}
.cx-els th{position:sticky;top:0;background:var(--soft);z-index:1;font-weight:600}.cx-els td.num,.cx-els th.num{text-align:right;font-variant-numeric:tabular-nums}
.cx-els th:nth-child(1){width:17%}.cx-els th:nth-child(2),.cx-els th:nth-child(3){width:19%}.cx-els th:nth-child(4){width:15%}.cx-els th:nth-child(5){width:21%}.cx-els th:last-child{width:calc(var(--cx-row) + 4px)}
.cx-els input{width:100%!important;min-width:0;box-sizing:border-box;text-align:right}.cx-els td button{padding:0;width:var(--cx-row)}.cx-els td:last-child{text-align:center}
.cx-add{display:grid;grid-template-columns:minmax(0,1fr) auto;gap:var(--sp-3);margin-top:var(--sp-3);padding-top:var(--sp-3);border-top:1px dashed var(--line);align-items:center}
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
        ${rows.map((r, i) => r.raw != null ? `<tr><td colspan="5" class="muted">${EH(r.raw)}</td><td><button type="button" data-del="${i}" aria-label="${t("cx.remove", { el: EH(r.raw) })}" title="${t("cx.remove", { el: EH(r.raw) })}">×</button></td></tr>` : (() => {
          const ii = isoInfo(r.s);
          return `<tr><td><b>${lab(r.s)}</b></td><td class="num"><input type="number" min="0" data-i="${i}" data-f="min" value="${r.n0}" aria-label="${t("cx.col.min")} ${EH(r.s)}"></td><td class="num"><input type="number" min="0" data-i="${i}" data-f="max" value="${r.n1}" aria-label="${t("cx.col.max")} ${EH(r.s)}"></td>
            <td class="num">${DBEQ[baseOf(r.s)] != null ? DBEQ[baseOf(r.s)] : ""}</td><td class="num">${ii ? ii.mass.toFixed(5) : ""}</td><td><button type="button" data-del="${i}" aria-label="${t("cx.remove", { el: EH(r.s) })}" title="${t("cx.remove", { el: EH(r.s) })}">×</button></td></tr>`;
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
  let pending = null;
  const takeCtx = () => { const c = pending; pending = null; return c; };
  const hasCtx = () => !!pending;
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
  return { generate, takeCtx, hasCtx, copyText, bindEls, bindPeriodic, DEFAULT_ELS, parse, ser, applyTo, peakFor };
})();
window.ELEM = ELEM;
