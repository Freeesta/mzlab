// @ts-strict
"use strict";
// Peak table of the spectrum panel (button «Tabella picchi»): a small popover with the peak list the chart draws (p._a.data[0].d), the facts of the spectrum
// and four controls (threshold, max rows, sort, only labelled). Output: COPY (TSV + html on the clipboard, or Ctrl/Cmd+C on the table) and CSV (also in the panel download menu).
// Classic helpers h and raw (html-sicuro.js) build every HTML string. Numbers are data: I18N.num, always with the decimal point. Classic helpers used by name: E, I18N. State per panel in p._tp.
(() => {
  /** @param {string} k @param {any} [v] */
  const T = (k, v) => I18N.t(k, v);
  const IC = '<svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.4" aria-hidden="true" focusable="false"><rect x="1.5" y="2.5" width="13" height="11" rx="1.2"/><path d="M1.5 6h13M1.5 9.7h13M6 6v7.5"/></svg>';
  /** @param {any} p */
  const st = p => p._tp || (p._tp = { thr: 2, max: 50, sort: "mz", lab: false, pop: null, tm: 0 });
  /** the facts of the spectrum shown: [label, value] @param {any} p */
  function facts(p) {
    const a = p._a, x = a.data[0], f = x.f || {}, d = x.d, i = p.si != null ? p.si : d.i0, ms2 = p.level >= 2;
    const rt = /** @type {HTMLElement | null} */ (p.el.querySelector(".rtl")), pol = f.polarity === "negative" ? "−" : f.polarity === "positive" ? "+" : "";
    const ce = ms2 && p.prec != null ? ((f.ms2_exps || []).find((/** @type {any} */ e) => Math.abs(e.prec - p.prec) < 0.6) || {}).ce : null;
    /** @type {[string, string][]} */ const out = [[T("tabp.f.sample"), String(f.label || "")], [T("tabp.f.scan"), d.scans === 1 && i != null ? String(i + 1) + (d.n ? "/" + d.n : "") : T("spec.scanLine.avg", { n: d.scans })],
      [T("tabp.f.rt"), rt ? String(rt.textContent) : ""], [T("tabp.f.level"), "MS" + (ms2 ? p.level : 1)]];
    if (ms2 && p.prec != null) out.push([T("tabp.f.prec"), I18N.num(p.prec, a.hrp ? a.dec : 2)]);
    if (ce != null) out.push([T("tabp.f.ce"), I18N.num(ce, 1) + " eV"]);
    if (pol) out.push([T("tabp.f.pol"), pol === "+" ? T("tabp.pos") : T("tabp.neg")]);
    const z = zoomed(p); if (z) out.push([T("tabp.f.range"), I18N.num(z[0], 2) + " – " + I18N.num(z[1], 2)]);
    return out;
  }
  /** the m/z window of a zoomed spectrum, else null @param {any} p */
  const zoomed = p => (p.zoom && p._a && p._a.x0 != null && p._a.x1 != null ? [p._a.x0, p._a.x1] : null);
  /** the rows to show (strings, already formatted) and the header @param {any} p */
  function rows(p) {
    const a = p._a, s = st(p), d = a.data[0].d, ms2 = p.level >= 2 && p.prec != null, dec = a.hrp ? a.dec : 2, y = d.y0 || d.y;
    const z = zoomed(p), inz = (/** @type {number} */ j) => !z || (d.mz[j] >= z[0] && d.mz[j] <= z[1]);   // zoomed: the peaks in view, the base peak of that window is 100
    let top = 1e-9; for (let j = 0; j < y.length; j++) if (inz(j) && y[j] > top) top = y[j];
    const lab = s.lab ? (a.lbls || []).filter((/** @type {any} */ b) => typeof b.m === "number" && !b.ann).map((/** @type {any} */ b) => b.m) : null;
    /** @type {number[]} */ let ix = [];
    for (let j = 0; j < d.mz.length; j++) if (inz(j) && y[j] > 0 && y[j] / top * 100 >= s.thr && (!lab || lab.some((/** @type {number} */ m) => Math.abs(m - d.mz[j]) < 1e-9))) ix.push(j);
    ix.sort((u, v) => y[v] - y[u]); ix = ix.slice(0, Math.max(1, s.max));                       // most intense first, then the cut
    if (s.sort === "mz") ix.sort((u, v) => d.mz[u] - d.mz[v]);
    const head = ["m/z", T("tabp.c.int"), T("tabp.c.rel"), ...(ms2 ? [T("tabp.c.dmz"), T("tabp.c.nl")] : [])];
    const body = ix.map(j => [I18N.num(d.mz[j], dec), I18N.sciE(y[j], 5), I18N.num(y[j] / top * 100, 1), ...(ms2 ? [I18N.num(d.mz[j] - p.prec, dec), I18N.num(p.prec - d.mz[j], dec)] : [])]);
    return { head, body };
  }
  /** @param {{ head: string[], body: string[][] }} r */
  const tsv = r => [r.head, ...r.body].map(l => l.join("\t")).join("\n");
  /** download the peak list as CSV, named like the other downloads of the panel; header is row 1 and the provenance line closes the file (as dlc) @param {any} p */
  function save(p) {
    if (!p._a || !p._a.data || !p._a.data[0]) return false;
    (async () => {
      let t = csvBuild({ head: rows(p).head, rows: rows(p).body });
      const w = await PROV.tsv(t, { p, table: "peaks" }), i = w.indexOf("\n");
      if (w !== t && i > 0) t += csvBuild({ rows: [[w.slice(0, i)]] });
      dl(plotName(p) + "_" + T("tabp.file") + ".csv", "\ufeff" + t, "text/csv;charset=utf-8", false);
    })();
    return true;
  }
  /** the entry «Peak list (CSV)» of the panel download menu @param {any} p @param {string} icon */
  const item = (p, icon) => ({ label: T("tabp.dl"), icon, fn: () => { if (!save(p)) info(T("panel.dl.nodata")); } });
  /** @param {{ head: string[], body: string[][] }} r */
  const html = r => h`<table><thead><tr>${raw(r.head.map(x => h`<th>${x}</th>`).join(""))}</tr></thead><tbody>${raw(r.body.map(l => h`<tr>${raw(l.map(c => h`<td>${c}</td>`).join(""))}</tr>`).join(""))}</tbody></table>`;
  /** @param {any} p */
  function render(p) {
    const s = st(p), pop = s.pop; if (!pop || !p._a) return;
    const r = rows(p), tb = pop.querySelector("table");
    pop.querySelector(".tp-facts").innerHTML = facts(p).map(([k, v]) => h`<div><span class="muted">${k}</span> <b>${v}</b></div>`).join("");
    tb.innerHTML = h`<caption class="sr-only">${T("tabp.title")}</caption><thead><tr>${raw(r.head.map(x => h`<th scope="col">${x}</th>`).join(""))}</tr></thead><tbody>${raw(r.body.map(l => h`<tr>${raw(l.map(c => h`<td class="num">${c}</td>`).join(""))}</tr>`).join(""))}</tbody>`;
    pop.querySelector(".tp-n").textContent = T("tabp.rows", { n: r.body.length });
  }
  /** @param {any} p @param {boolean} [back] */
  function close(p, back) {
    const s = st(p); if (!s.pop) return;
    s.pop.remove(); s.pop = null;
    const b = /** @type {HTMLElement | null} */ (p.el.querySelector('[data-a="tabp"]')); if (b) { b.setAttribute("aria-expanded", "false"); if (back) b.focus({ preventScroll: true }); }
  }
  /** TSV + html on the clipboard; plain text alone when the browser has no ClipboardItem @param {any} p */
  async function copy(p) {
    const r = rows(p), t = tsv(r), s = st(p);
    try {
      if (window.ClipboardItem && navigator.clipboard.write) await navigator.clipboard.write([new ClipboardItem({ "text/plain": new Blob([t], { type: "text/plain" }), "text/html": new Blob([html(r)], { type: "text/html" }) })]);
      else await navigator.clipboard.writeText(t);
    } catch (_) { try { await navigator.clipboard.writeText(t); } catch (e) { return; } }
    const m = s.pop && s.pop.querySelector(".tp-ok"); if (!m) return;
    m.textContent = T("tabp.copied"); clearTimeout(s.tm); s.tm = setTimeout(() => { m.textContent = ""; }, 2000);
  }
  /** @param {any} p */
  function open(p) {
    const s = st(p); if (s.pop) { close(p, true); return; }
    if (!p._a || !p._a.data || !p._a.data[0]) return;
    const pop = document.createElement("div"); pop.className = "sp-pop tp-pop"; pop.setAttribute("role", "dialog"); pop.setAttribute("aria-label", T("tabp.title"));
    pop.innerHTML = h`<div class="tp-facts"></div>
      <div class="tp-ctl"><label>${T("tabp.thr")} <input type="number" data-f="thr" min="0" max="100" step="0.5" value="${s.thr}"></label>
      <label>${T("tabp.max")} <input type="number" data-f="max" min="1" max="5000" step="1" value="${s.max}"></label>
      <label>${T("tabp.sort")} <select data-f="sort"><option value="mz"${raw(s.sort === "mz" ? " selected" : "")}>${T("tabp.sort.mz")}</option><option value="int"${raw(s.sort === "int" ? " selected" : "")}>${T("tabp.sort.int")}</option></select></label>
      <label class="tp-chk"><input type="checkbox" data-f="lab"${raw(s.lab ? " checked" : "")}> ${T("tabp.lab")}</label></div>
      <div class="tp-sc" tabindex="0" role="region" aria-label="${T("tabp.title")}"><table></table></div>
      <div class="tp-ft"><button type="button" class="bt" data-f="copy">${T("tabp.copy")}</button><button type="button" class="bt" data-f="csv">${T("tabp.csv")}</button><button type="button" class="bt" data-f="no">${T("tabp.close")}</button><span class="sm tp-n"></span><span class="sm tp-ok" role="status" aria-live="polite"></span></div>`;
    p.el.appendChild(pop); s.pop = pop;
    const b = /** @type {HTMLElement} */ (p.el.querySelector('[data-a="tabp"]')), pr = p.el.getBoundingClientRect(), br = b.getBoundingClientRect();
    b.setAttribute("aria-expanded", "true");
    pop.style.left = Math.max(4, Math.min(br.left - pr.left, p.el.clientWidth - pop.offsetWidth - 6)) + "px"; pop.style.top = br.bottom - pr.top + 4 + "px";
    pop.addEventListener("input", e => {
      const t = /** @type {HTMLInputElement} */ (e.target), f = t.dataset.f;
      if (f === "thr") { const v = +t.value; if (t.value !== "" && isFinite(v)) s.thr = Math.max(0, Math.min(100, v)); }
      else if (f === "max") { const v = Math.floor(+t.value); if (t.value !== "" && isFinite(v) && v >= 1) s.max = v; }
      else if (f === "sort") s.sort = t.value === "int" ? "int" : "mz";
      else if (f === "lab") s.lab = t.checked;
      render(p);
    });
    pop.addEventListener("keydown", e => { if (e.key === "Escape") { e.stopPropagation(); close(p, true); } });
    const sc = /** @type {HTMLElement} */ (pop.querySelector(".tp-sc"));
    sc.addEventListener("keydown", e => { if ((e.ctrlKey || e.metaKey) && (e.key || "").toLowerCase() === "a") { e.preventDefault(); const g = getSelection(); if (g) g.selectAllChildren(/** @type {Node} */ (sc.querySelector("table"))); } });
    pop.addEventListener("copy", e => {                                                  // select-all + Ctrl/Cmd+C on the table gives the same TSV
      const g = getSelection(), tb = sc.querySelector("table"); if (!g || g.isCollapsed || !tb || !g.containsNode(tb, true) || !e.clipboardData) return;
      const r = rows(p); e.clipboardData.setData("text/plain", tsv(r)); e.clipboardData.setData("text/html", html(r)); e.preventDefault();
    });
    /** @type {HTMLElement} */ (pop.querySelector('[data-f="copy"]')).onclick = () => { copy(p); };
    /** @type {HTMLElement} */ (pop.querySelector('[data-f="csv"]')).onclick = () => { save(p); };
    /** @type {HTMLElement} */ (pop.querySelector('[data-f="no"]')).onclick = () => close(p, true);
    render(p);
    /** @type {HTMLElement} */ (pop.querySelector('[data-f="thr"]')).focus();
  }
  /** the toolbar button, in its own group before the «⋯» menu @param {any} p */
  function wire(p) {
    if (p.el.querySelector('[data-a="tabp"]')) return;
    const b = document.createElement("button"); b.type = "button"; b.className = "bt tpb"; b.dataset.a = "tabp"; b.title = T("tabp.btn.title"); b.setAttribute("aria-label", T("tabp.btn.title"));
    b.setAttribute("aria-haspopup", "dialog"); b.setAttribute("aria-expanded", "false"); b.innerHTML = IC; b.onclick = () => open(p);
    const g = document.createElement("span"); g.className = "tbg"; g.append(b);
    const mb = p.el.querySelector('.tbs [data-a="more"]'); if (mb) mb.before(g); else p.el.querySelector(".tbs")?.append(g);
  }
  /** after every drawing: the table follows the scan; a spectrum with no peaks closes it @param {any} p */
  function sync(p) { const s = p._tp; if (s && s.pop) render(p); }
  window.TABPICCHI = { wire, sync, rows, facts, open, close, save, item, tsv };
})();
