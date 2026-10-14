// @ts-check
"use strict";
// The precursor tree of an MSn infusion, as the body of a tab of the information bar of the bench (banco.js, BODY.alb): indented rows MS1 > MS2 > MSn with the
// precursor, the level, the collision energy, the number of scans and the minutes; above them the level, the tolerance and the energies of the grouping, and the
// Mean | Sum switch of the spectrum. A click (or Enter) puts the node on the active cell; the arrows walk the tree. Needs infusione.js.
(() => {
  const sel = (/** @type {string} */ id, /** @type {string} */ label, /** @type {[string, string][]} */ opts, /** @type {string} */ cur) =>
    h`<label>${label} <select id="${id}" title="${label}">${opts.map(([v, l]) => raw(h`<option value="${v}"${raw(v === cur ? " selected" : "")}>${l}</option>`))}</select></label>`;
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
  /** @param {HTMLElement} b @param {any} p @param {number} tok */
  async function render(b, p, tok) {
    const t = target(p), f = E.files[(t && t.k) ?? E.cur] || E.files.find(INF.isInf);
    if (!f || !INF.isInf(f)) return void (b.innerHTML = h`<div class="muted">${I18N.t("inf.none")}</div>`);
    const j = await INF.tree(f.k);
    if (tok !== (window.BANCO && BANCO.token())) return;
    const lv = INF.O.lv, rows = INF.rows(j).filter(r => !lv || r.n.livello <= lv), cur = t && t.filt;
    const levels = /** @type {[string, string][]} */ ([["0", I18N.t("inf.lv.all")], ...j.livelli.map((/** @type {number} */ l) => [String(l), "MS" + l])]);
    const head = ["inf.col.prec", "MSⁿ", "inf.col.ce", "inf.col.n", "inf.col.min"].map((k, i) => raw(h`<th${raw(i === 3 ? ' class="num"' : "")}>${k.startsWith("inf.") ? I18N.t(k) : k}</th>`));
    const first = h`<tr role="treeitem" aria-level="1" aria-selected="${cur ? "false" : "true"}" tabindex="${cur ? -1 : 0}" data-id=""><td>MS1</td><td>MS1</td><td></td><td class="num">${j.ms1.n}</td><td>${j.ms1.rt0 == null ? "" : mins({ rt0: j.ms1.rt0, rt1: j.ms1.rt1 })}</td></tr>`;
    const line = (/** @type {any} */ r) => {
      const on = cur === "msn:" + r.n.id, skip = r.n.salto ? " · " + I18N.t("inf.skip", { n: r.n.salto }) : "";
      return h`<tr role="treeitem" aria-level="${r.depth + 1}" aria-selected="${on}" tabindex="${on ? 0 : -1}" data-id="${r.n.id}" data-par="${r.n.genitore == null ? "" : r.n.genitore}" title="${r.path + skip}"><td style="padding-left:${r.depth * 14}px">${INF.prec(r.n)}</td><td>MS${r.n.livello}</td><td>${r.n.ce ?? ""}</td><td class="num">${r.n.n}</td><td>${mins(r.n)}</td></tr>`;
    };
    b.innerHTML = h`<div class="row">${raw(sel("inf-lv", I18N.t("inf.lv"), levels, String(lv)))}${raw(sel("inf-tol", I18N.t("inf.tol"), [["nom", I18N.t("inf.tol.nom")], ["0.01", "0.01 Da"], ["10ppm", "10 ppm"]], INF.O.tol))}` +
      h`${raw(sel("inf-ce", I18N.t("inf.ce"), [["1", I18N.t("inf.ce.sep")], ["0", I18N.t("inf.ce.mer")]], INF.O.ce ? "1" : "0"))}${raw(sel("inf-sum", I18N.t("inf.avg"), [["0", I18N.t("inf.mean")], ["1", I18N.t("inf.sumL")]], INF.O.sum ? "1" : "0"))}</div>` +
      h`<table id="inf-tree" role="tree" aria-label="${I18N.t("inf.tree.aria")}"><thead><tr>${head}</tr></thead><tbody>${raw(first)}${rows.map(r => raw(line(r)))}</tbody></table>`;
    const byId = new Map(rows.map(r => [r.n.id, r]));
    /** @type {NodeListOf<HTMLElement>} */
    const trs = b.querySelectorAll("#inf-tree tbody tr");
    const go = (/** @type {HTMLElement} */ tr) => { const c = target(p); if (!c) return; if (tr.dataset.id) INF.pick(c, tr.dataset.id); else cells0(c); };
    const cells0 = (/** @type {any} */ c) => { const { c: ch, specs } = INF.cells(c); [ch, ...specs].filter(Boolean).forEach(q => BANCO.setFilter(q, null, [])); };
    trs.forEach(tr => {
      tr.onclick = () => go(tr);
      tr.oncontextmenu = e => { e.preventDefault(); const r = byId.get(tr.dataset.id || ""); if (r) rowMenu(e, f, r, p); };
      tr.onkeydown = e => {
        const i = [...trs].indexOf(tr), mv = (/** @type {HTMLElement | undefined} */ x) => { if (x) { e.preventDefault(); trs.forEach(y => { y.tabIndex = -1; }); x.tabIndex = 0; x.focus(); } };
        if (e.key === "ArrowDown") mv(trs[i + 1]);
        else if (e.key === "ArrowUp") mv(trs[i - 1]);
        else if (e.key === "ArrowRight") mv([...trs].find(y => y !== tr && y.dataset.par === (tr.dataset.id || "")));
        else if (e.key === "ArrowLeft") mv([...trs].find(y => y.dataset.id === tr.dataset.par));
        else if (e.key === "Enter" || e.key === " ") { e.preventDefault(); go(tr); }
      };
    });
    const on = (/** @type {string} */ id, /** @type {(v: string) => void} */ fn) => { const el = /** @type {HTMLSelectElement | null} */ (b.querySelector("#" + id)); if (el) el.onchange = () => fn(el.value); };
    on("inf-lv", v => INF.setOpt("lv", +v));
    on("inf-tol", v => INF.setOpt("tol", v));
    on("inf-ce", v => INF.setOpt("ce", v === "1"));
    on("inf-sum", v => INF.setOpt("sum", v === "1"));
  }
  Object.assign(INF, { render });
})();
