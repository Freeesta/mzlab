// @ts-strict
"use strict";
// The precursor tree of an MSn infusion, as the body of a tab of the information bar of the bench (banco.js, BODY.alb): indented rows MS1 > MS2 > MSn with the
// precursor, the level, the collision energy, the number of scans and the minutes; above them the level, the tolerance and the energies of the grouping, and the
// Mean | Sum switch of the spectrum. A click (or Enter) puts the node on the active cell; the arrows walk the tree. Needs infusione.js.
(() => {
  /** A labelled select: the name above, the one-sentence meaning in the title of the label and of the control. */
  const sel = (/** @type {string} */ id, /** @type {string} */ label, /** @type {string} */ tip, /** @type {[string, string][]} */ opts, /** @type {string} */ cur) =>
    h`<label class="inf-f" title="${tip}"><span>${label}</span><select id="${id}" title="${tip}">${opts.map(([v, l]) => raw(h`<option value="${v}"${raw(v === cur ? " selected" : "")}>${l}</option>`))}</select></label>`;
  /** «MS» with the level as a superscript, for the cells of a table: copied as plain text it reads «MS3»; the screen reader says «MS 3». */
  const ms = (/** @type {number | string} */ l) => raw(`<span class="msn" role="img" aria-label="MS ${l}">MS<sup>${l}</sup></span>`);
  const mins = (/** @type {any} */ n) => `${n.rt0.toFixed(2)}–${n.rt1.toFixed(2)}`;
  /** The cell that takes a node: the active one if it can show scans of a file, else the chromatogram of the infusion tab. */
  function target(/** @type {any} */ p) {
    if (p && BANCO.eligible(p) && INF.isInf(E.files[p.k ?? E.cur])) return p;
    return E.panels.find(q => q.type === "chrom" && q.tab === "infusion") || E.panels.find(q => q.type === "spec" && q.tab === "infusion") || null;
  }
  /** Opens the node in a spectrum cell of its own (compare two energies side by side). */
  function inNewCell(/** @type {any} */ f, /** @type {any} */ n) {
    const c = addPanel("spec", { tab: "infusion", k: f.k, level: n.livello, r0: n.rt0, r1: n.rt1, filt: "msn:" + n.id, link: null, x: 0, y: E.panels.reduce((m, q) => Math.max(m, q.y + q.h + pnlGap()), 0), w: hostWidth(), h: 300, full: true });
    relayout(); fitHost(); setActive(c); c.el.scrollIntoView({ block: "center", behavior: "smooth" }); uiSave();
  }
  function rowMenu(/** @type {MouseEvent} */ e, /** @type {any} */ f, /** @type {any} */ r, /** @type {any} */ p) {
    menu(e, [
      { label: I18N.t("inf.row.spec"), fn: () => { const c = target(p); if (c) INF.pick(c, r.n.id); } },
      { label: I18N.t("inf.row.new"), fn: () => inNewCell(f, r.n) },
      { label: I18N.t("inf.row.copy"), fn: () => { try { navigator.clipboard.writeText(r.path).then(() => nearMsg(I18N.t("inf.row.copied"))); } catch (_) { /* no clipboard */ } } },
      { label: I18N.t("inf.row.formulas"), fn: () => { if (window.COMP) COMP.tree(f.k); } },
    ]);
  }
  /** Selection follows the node of the cell at once: aria-selected and the roving tabindex of the existing rows, no rebuilding. */
  function mark(/** @type {Element} */ table, /** @type {string | null | undefined} */ cur) {
    const id = !cur ? "" : cur.startsWith("msn:") ? cur.slice(4) : null;
    const trs = /** @type {HTMLElement[]} */ ([...table.querySelectorAll("tbody tr")]);
    let any = false;
    trs.forEach(tr => { const on = id !== null && tr.dataset.id === id; any ||= on; tr.setAttribute("aria-selected", String(on)); tr.tabIndex = on ? 0 : -1; });
    if (!any && trs[0]) trs[0].tabIndex = 0;
  }
  /** @type {{ b: HTMLElement, p: any } | null} */
  let view = null;
  /** The four controls, in three groups: what to show, how the precursors are grouped, how a spectrum is made. */
  function controls(/** @type {any} */ j) {
    const T = (/** @type {string} */ k) => I18N.t(k), O = INF.O;
    const levels = /** @type {[string, string][]} */ ([["0", T("inf.lv.all")], ...j.livelli.map((/** @type {number} */ l) => [String(l), INF.msTxt(l)])]);
    return h`<div class="inf-g">${raw(sel("inf-lv", T("inf.lv"), T("inf.lv.tip"), levels, String(O.lv)))}</div>` +
      h`<div class="inf-g">${raw(sel("inf-tol", T("inf.tol"), T("inf.tol.tip"), [["nom", T("inf.tol.nom")], ["0.01", "0.01 Da"], ["10ppm", "10 ppm"]], O.tol))}${raw(sel("inf-ce", T("inf.ce"), T("inf.ce.tip"), [["1", T("inf.ce.sep")], ["0", T("inf.ce.mer")]], O.ce ? "1" : "0"))}</div>` +
      h`<div class="inf-g">${raw(sel("inf-sum", T("inf.avg"), T("inf.avg.tip"), [["0", T("inf.mean")], ["1", T("inf.sumL")]], O.sum ? "1" : "0"))}</div>`;
  }
  /** @param {HTMLElement} b @param {any} p @param {number} tok */
  async function render(b, p, tok) {
    view = { b, p };
    const t = target(p), f = E.files[(t && t.k) ?? E.cur] || E.files.find(INF.isInf);
    if (!f || !INF.isInf(f)) return void (b.innerHTML = h`<div class="muted">${I18N.t("inf.none")}</div>`);
    if (!b.querySelector(".inf-ctl")) b.innerHTML = `<div class="muted">…</div>`;
    const j = await INF.tree(f.k);
    if (tok !== (window.BANCO && BANCO.token())) return;
    const lv = INF.O.lv, cur = t && t.filt, ckey = j.livelli.join();
    // the controls stay in place (focus and open lists survive a change of option); the table is rebuilt only when its rows change
    if (!b.querySelector(".inf-ctl") || b.dataset.infLv !== ckey) {
      b.dataset.infLv = ckey;
      b.innerHTML = h`<div class="inf-ctl">${raw(controls(j))}<span id="inf-busy" class="muted" role="status"></span></div><div id="inf-wrap"></div>`;
      const on = (/** @type {string} */ id, /** @type {(v: string) => void} */ fn) => { const el = /** @type {HTMLSelectElement | null} */ (b.querySelector("#" + id)); if (el) el.onchange = () => fn(el.value); };
      on("inf-lv", v => INF.setOpt("lv", +v));
      on("inf-tol", v => INF.setOpt("tol", v));
      on("inf-ce", v => INF.setOpt("ce", v === "1"));
      on("inf-sum", v => INF.setOpt("sum", v === "1"));
      ["inf-tol", "inf-ce"].forEach(id => { const el = b.querySelector("#" + id); if (el) el.addEventListener("focus", () => { const c = view && target(view.p); INF.warm((c && c.k) ?? E.cur); }); });
    }
    const wrap = /** @type {HTMLElement & { _j?: any }} */ (b.querySelector("#inf-wrap")), key = `${f.k}|${INF.O.tol}|${INF.O.ce}|${lv}`;
    if (wrap.dataset.key === key && wrap._j === j && wrap.firstElementChild) return void mark(wrap.firstElementChild, cur);
    wrap.dataset.key = key; wrap._j = j;
    const rows = INF.rows(j).filter(r => !lv || r.n.livello <= lv);
    const head = [raw(I18N.t("inf.col.prec")), ms("n"), I18N.t("inf.col.ce"), I18N.t("inf.col.n"), I18N.t("inf.col.min")].map((x, i) => raw(h`<th${raw(i === 3 ? ' class="num"' : "")}>${x}</th>`));
    const first = h`<tr role="treeitem" aria-level="1" aria-selected="false" tabindex="-1" data-id=""><td>${ms(1)}</td><td>${ms(1)}</td><td></td><td class="num">${j.ms1.n}</td><td>${j.ms1.rt0 == null ? "" : mins({ rt0: j.ms1.rt0, rt1: j.ms1.rt1 })}</td></tr>`;
    const line = (/** @type {any} */ r) => {
      const skip = r.n.salto ? " · " + I18N.t("inf.skip", { n: r.n.salto }) : "";
      return h`<tr role="treeitem" aria-level="${r.depth + 1}" aria-selected="false" tabindex="-1" data-id="${r.n.id}" data-par="${r.n.genitore == null ? "" : r.n.genitore}" title="${r.path + skip}"><td style="padding-left:${r.depth * 14}px">${INF.prec(r.n)}</td><td>${ms(r.n.livello)}</td><td>${r.n.ce ?? ""}</td><td class="num">${r.n.n}</td><td>${mins(r.n)}</td></tr>`;
    };
    wrap.innerHTML = h`<table id="inf-tree" role="tree" aria-label="${I18N.t("inf.tree.aria")}"><thead><tr>${head}</tr></thead><tbody>${raw(first)}${rows.map(r => raw(line(r)))}</tbody></table>`;
    const table = /** @type {HTMLElement} */ (wrap.firstElementChild);
    mark(table, cur);
    const byId = new Map(rows.map(r => [r.n.id, r]));
    const trs = /** @type {HTMLElement[]} */ ([...table.querySelectorAll("tbody tr")]);
    const cells0 = (/** @type {any} */ c) => { const { c: ch, specs } = INF.cells(c); [ch, ...specs].filter(Boolean).forEach(q => BANCO.setFilter(q, null, [])); };
    /** The highlight comes first, in the same frame as the click; the cells follow, with a notice only when they take longer than 100 ms. */
    const go = (/** @type {HTMLElement} */ tr) => {
      const c = target(p); if (!c) return;
      mark(table, tr.dataset.id ? "msn:" + tr.dataset.id : null);
      const busy = b.querySelector("#inf-busy"), timer = setTimeout(() => { if (busy) busy.textContent = I18N.t("inf.loading"); table.setAttribute("aria-busy", "true"); }, 100);
      const done = () => { clearTimeout(timer); if (busy) busy.textContent = ""; table.removeAttribute("aria-busy"); };
      Promise.resolve(tr.dataset.id ? INF.pick(c, tr.dataset.id) : cells0(c)).catch(e => console.info("INF pick", e)).then(done);
    };
    const rowOf = (/** @type {Event} */ e) => /** @type {HTMLElement | null} */ ((/** @type {Element} */ (e.target)).closest("tbody tr"));
    table.onclick = e => { const tr = rowOf(e); if (tr) go(tr); };
    table.oncontextmenu = e => { const tr = rowOf(e); if (!tr) return; e.preventDefault(); const r = byId.get(tr.dataset.id || ""); if (r) rowMenu(e, f, r, p); };
    table.onkeydown = e => {
      const tr = rowOf(e); if (!tr) return;
      const i = trs.indexOf(tr), mv = (/** @type {HTMLElement | undefined} */ x) => { if (x) { e.preventDefault(); trs.forEach(y => { y.tabIndex = -1; }); x.tabIndex = 0; x.focus(); } };
      if (e.key === "ArrowDown") mv(trs[i + 1]);
      else if (e.key === "ArrowUp") mv(trs[i - 1]);
      else if (e.key === "ArrowRight") mv(trs.find(y => y !== tr && y.dataset.par === (tr.dataset.id || "")));
      else if (e.key === "ArrowLeft") mv(trs.find(y => y.dataset.id === tr.dataset.par));
      else if (e.key === "Enter" || e.key === " ") { e.preventDefault(); go(tr); }
    };
  }
  /** Redraws the table of the open tab now (an option changed): no frame of waiting, no «…» in between. */
  function refresh() {
    if (!view || !view.b.isConnected || !view.b.querySelector(".inf-ctl") || !window.BANCO) return false;
    void render(view.b, view.p, BANCO.token());
    return true;
  }
  /** Computes the trees of the other groupings in advance (when the list of tolerances or energies is opened): the choice then shows at once. */
  const warm = (/** @type {number} */ k) => { for (const tol of ["nom", "0.01", "10ppm"]) for (const ce of [true, false]) INF.treeFor(k, tol, ce).catch(() => {}); };
  /** Spectra already computed, by Mean (false) and Sum (true): a switch back and forth does not ask the engine again. */
  const spectra = { false: new Map(), true: new Map() };
  const isSpectrumKey = (/** @type {string} */ k) => /^(s\d|dav\|)/.test(k);
  function swapSpectra(/** @type {boolean} */ was) {
    for (const [k, v] of [...CACHE]) if (isSpectrumKey(k)) { spectra[was ? "true" : "false"].set(k, v); CACHE.delete(k); }
    spectra[INF.O.sum ? "true" : "false"].forEach((v, k) => CACHE.set(k, v));
    spectra[INF.O.sum ? "true" : "false"].clear();
  }
  /** Redraws the table of the tree panel in the same turn (the panel in place, else the usual deferred render). */
  const redraw = () => { if (!(refresh()) && window.BANCO) BANCO.renderInfo(); };
  /** Changes an option of the tree. A new grouping makes every node id of the cells meaningless: they go back to the richest node.
   *  The table needs only the tree: it is redrawn first and never waits for the cells; Level touches nothing else, Sum only the spectra. */
  async function setOpt(/** @type {"tol" | "ce" | "sum" | "lv"} */ key, /** @type {any} */ v) {
    const regroup = key === "tol" || key === "ce", was = INF.O.sum;
    /** @type {any} */ (INF.O)[key] = v;
    if (key === "lv") { redraw(); uiSave(); return; }
    if (key === "sum") { swapSpectra(was); if (typeof SC !== "undefined") { SC.m.clear(); SC.n.clear(); } }
    if (regroup) INF.reset();
    if (regroup) redraw();
    const cs = regroup ? E.panels.filter(q => q.type === "chrom" && INF.isInf(E.files[q.k ?? E.cur]) || q.tab === "infusion" && q.type === "chrom") : [];
    for (const c of cs) { const f = E.files.find(INF.isInf); if (f) await INF.layout(c, null, f); }
    E.panels.filter(q => regroup || (q.type === "spec" && q.filt && q.filt.startsWith("msn:"))).forEach(q => draw(q));
    if (regroup) redraw();
    uiSave();
  }
  /** The only file loaded when it is an infusion: the «Fragmentation graph» (infusione-grafo.js) belongs to that case. */
  const single = () => { const l = E.files.filter((/** @type {any} */ f) => !f.gone); return l.length === 1 && INF.isInf(l[0]); };
  Object.assign(INF, { render, refresh, target, single, setOpt, warm });
})();
