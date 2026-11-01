// @ts-check
// @ts-strict
"use strict";
// The page side of «What can this number be?»: ONE view component (choices row + groups of candidates) mounted in the three places that show it
// (under the m/z calculator, the Adducts panel, the Neutral losses panel), all fed by calc-qui.js and drawn by calc-qui-html.js.
// Public: CALCQUI.apri(n) opens the calculator on a number (the spectrum ruler uses it). Classic script, after calc-addotti.js and liste.js.

(() => {
  const Q1 = (/** @type {string} */ s) => /** @type {HTMLElement | null} */ (document.querySelector(s));
  const H = CALCQUIHTML, DEBOUNCE = 90;
  /** @typedef {{ ctl: HTMLElement, out: HTMLElement, idp: string, quiet?: boolean, numOnly?: boolean, arg: () => { text: string, ans: number }, groups?: string[], pol?: () => 1 | -1 | null, after?: (a: any, gs: CqGroup[]) => void }} CqCfg */
  /** @typedef {{ cfg: CqCfg, open: Set<string>, all: Set<string>, more: boolean, diffs: boolean, delta: string, seq: number, timer: number, tab: any, formula: string }} CqView */
  const views = /** @type {CqView[]} */ ([]);
  const prune = () => { for (let i = views.length - 1; i >= 0; i--) if (!views[i].cfg.out.isConnected) views.splice(i, 1); };      // a panel redrawn leaves its old view behind
  const memo = /** @type {Map<string, number>} */ (new Map());
  const mass = (/** @type {string} */ f) => { const c = memo.get(f); if (c != null) return c; const m = QQQRef.massOf(f); memo.set(f, m); return m; };

  // resolution and polarity follow the files shown (negative only = ESI-); a file is high resolution when its profile says so, and the instrument names the reason
  const files = () => (typeof E !== "undefined" ? E.files.filter((/** @type {any} */ f) => !f.gone) : []);
  const filesMode = () => {
    const fs = files();
    if (!fs.length) return null;
    const hf = window.HR ? fs.find((/** @type {any} */ f) => HR.isHr(f, 1) || HR.isHr(f, 2)) : null, f = hf || fs[0], p = f.prof1 || {};
    return { hr: !!hf, name: String(f.instrument || (p.an && p.an !== "?" ? p.an : "") || f.label || f.file || "") };
  };
  CALCQUI.follow(filesMode, () => (files().some((/** @type {any} */ f) => f.polarity === "negative") && !files().some((/** @type {any} */ f) => f.polarity === "positive") ? -1 : 1));
  CALCQUI.load();

  async function deps() {
    if (window.LISTE) await LISTE.ready();
    return { idx: window.LISTE ? LISTE.ensure() : null, find: LISTE.find, findNominal: LISTE.findNominal, table: CALCADD.table, mass, losses: LOSSES, lossCombos };
  }
  /** @param {CqView} v */
  function options(v) {
    const t = v.cfg.arg(), o = CALCQUI.opts(CALCQUI.arg(t.text, t.ans)), p = v.cfg.pol ? v.cfg.pol() : null;
    if (p) o.pol = p;
    return o;
  }

  /** Draws the groups of the view for its argument (the newest call wins). @param {CqView} v */
  async function render(v) {
    const my = ++v.seq, o = options(v), t = v.cfg.arg(), a = CALCQUI.arg(t.text, t.ans);
    sync(v);                                                                                  // the detected resolution (text and tolerance) follows what is typed
    if (v.cfg.numOnly && a.kind !== "num" || v.cfg.quiet && a.kind === "none") { v.cfg.out.innerHTML = ""; return; }      // a panel with its own field says nothing until something is typed
    let gs = await CALCQUI.gruppi(a, o, await deps());
    if (my !== v.seq) return;
    if (v.cfg.groups) gs = gs.filter(g => /** @type {string[]} */ (v.cfg.groups).includes(g.id));
    const ad = gs.find(g => g.id === "addotti");
    v.tab = ad ? ad.tab : null; v.formula = ad && /** @type {Bag} */ (ad).formula ? ad.tab.formula || "" : "";
    v.diffs = v.open.has("diff");
    v.cfg.out.innerHTML = H.groupsHtml(gs, o, v, { formula: a.kind === "formula" });
    if (v.cfg.after) v.cfg.after(a, gs);
  }
  const later = (/** @type {CqView} */ v) => { clearTimeout(v.timer); v.timer = window.setTimeout(() => render(v).catch(e => { v.cfg.out.innerHTML = h`<p class="muted">${e && e.message || ""}</p>`; }), DEBOUNCE); };

  /** The choices row of a view follows the shared choices. @param {CqView} v */
  function sync(v) {
    const o = options(v), c = v.cfg.ctl, put = (/** @type {string} */ id, /** @type {string} */ val) => { const i = /** @type {HTMLInputElement | null} */ (c.querySelector("#" + v.cfg.idp + id)); if (i && document.activeElement !== i) i.value = val; };
    c.querySelectorAll(".cq-ctl [data-p]").forEach(b => { const on = +(b.getAttribute("data-p") || 0) === o.pol; b.classList.toggle("on", on); b.setAttribute("aria-pressed", String(on)); });
    const tl = c.querySelector(".cq-tl"); if (tl) tl.textContent = I18N.t(o.hr ? "calc.tol" : "calc.tol.da");
    const cc = c.querySelector(".cq-ctl"); if (cc) { cc.setAttribute("data-mode", o.hr ? "hr" : "lr"); cc.setAttribute("data-src", o.mode.src); cc.setAttribute("data-name", o.mode.name || ""); }
    put("tol", String(o.tol));
  }

  /** @param {CqCfg} cfg */
  function mount(cfg) {
    prune();
    const v = /** @type {CqView} */ ({ cfg, open: new Set(), all: new Set(), more: false, diffs: false, delta: "", seq: 0, timer: 0, tab: null, formula: "" });
    views.push(v);
    cfg.ctl.innerHTML = H.ctrlHtml(cfg.idp, options(v));
    const set = (/** @type {Partial<CqState>} */ p) => CALCQUI.set(p), tolKey = () => (options(v).hr ? "ppm" : "da");
    cfg.ctl.addEventListener("click", ev => {
      const b = /** @type {HTMLElement} */ (ev.target).closest("button");
      if (!b) return;
      if (b.dataset.p) set({ pol: +b.dataset.p > 0 ? 1 : -1 });
    });
    cfg.ctl.addEventListener("input", ev => {
      const i = /** @type {HTMLInputElement} */ (ev.target);
      if (i.id === cfg.idp + "tol") set({ [tolKey()]: i.value });
    });
    bindOut(v);
    sync(v); later(v);
    return v;
  }

  /** Clicks inside the groups: folds, details of a loss, XIC of an adduct, the difference typed by hand. @param {CqView} v */
  function bindOut(v) {
    const out = v.cfg.out;
    out.addEventListener("click", ev => {
      const el = /** @type {HTMLElement} */ (ev.target), more = el.closest(".cadd-more"), x = /** @type {HTMLElement | null} */ (el.closest(".cxic")), lb = el.closest(".cq-lb");
      if (more && more.classList.contains("cq-all")) {
        const g = more.getAttribute("data-g") || ""; if (v.all.has(g)) v.all.delete(g); else v.all.add(g);
        render(v).then(() => { const nb = out.querySelector(`.cq-all[data-g="${g}"]`); if (nb instanceof HTMLElement) nb.focus(); });
      } else if (more) {
        v.more = more.getAttribute("aria-expanded") !== "true";
        render(v).then(() => { const nb = out.querySelector(".cadd-more:not(.cq-all)"); if (nb instanceof HTMLElement) nb.focus(); });
      } else if (lb) {
        const d = /** @type {HTMLElement} */ (document.getElementById(lb.getAttribute("aria-controls") || ""));
        if (d) { d.hidden = !d.hidden; lb.setAttribute("aria-expanded", String(!d.hidden)); }
      } else if (x && v.tab) openXicOf(v, +(x.dataset.i || 0));
    });
    out.addEventListener("toggle", ev => {
      // the catalogue of differences of a formula has its own key: the «diff» group of a number opens by itself and must not open this one
      const d = /** @type {HTMLDetailsElement} */ (ev.target), g = d.dataset.g || (d.id === "cadd-diffs" ? "catdiff" : "");
      if (!g) return;
      if (d.open) { v.open.add(g); v.open.delete("-" + g); } else { v.open.delete(g); v.open.add("-" + g); }
    }, true);
    out.addEventListener("input", ev => {
      const i = /** @type {HTMLInputElement} */ (ev.target);
      if (i.id !== "cadd-delta") return;
      v.delta = i.value.trim();
      const o = options(v);
      CALCADD.table("", "", String(CALCQUI.ppmFor(o, 400, true)), v.delta).then(r => { const d = out.querySelector("#cadd-dres"); if (d && i.value.trim() === v.delta) d.innerHTML = CALCADDHTML.deltaHtml(r); }).catch(() => { /* the field is the student's: keep what is shown */ });
    });
  }
  /** @param {CqView} v @param {number} i */
  function openXicOf(v, i) {
    const r = v.tab.rows[i], f = v.formula;
    if (v.cfg.idp === "calc") Q1("#calcdlg") && /** @type {any} */ (Q1("#calcdlg")).close();
    const panel = [...tabPanels()].reverse().find((/** @type {any} */ q) => q.type === "xic") || null;
    // the adducts the XIC window knows travel with the formula; the others as the m/z of the table
    openXic(panel, XIC_ADDUCTS.includes(r.n) && f ? { formula: f, adduct: r.n } : { mz: r.mz5 });
  }

  CALCQUI.onChange(() => { prune(); views.slice().forEach(v => { sync(v); later(v); }); });

  /** Opens the calculator on a number: the groups under it answer «what can this be?». gruppo: the group to open. @param {number | string} n @param {{ gruppo?: string }} [opt] */
  function apri(n, opt = {}) {
    const dlg = Q1("#calcdlg");
    if (window.BARRA && BARRA.isDataView()) BARRA.setTab("calc"); else if (dlg && dlg.hidden && window.calcToggleFloat) window.calcToggleFloat();
    const inp = /** @type {HTMLInputElement | null} */ (Q1("#calcin"));
    if (!inp) return;
    inp.value = String(n); CALC.fresh = true;
    const v = views.find(x => x.cfg.idp === "calc");
    if (v && opt.gruppo) { v.open.add(opt.gruppo); v.open.delete("-" + opt.gruppo); }
    calcRun();
  }

  // the calculator's own: the argument is its field (or the RESULT of the operation in it), the line under it says how a formula was read
  const calc = mount({
    ctl: /** @type {HTMLElement} */ (Q1("#calcctl")), out: /** @type {HTMLElement} */ (Q1("#calcout")), idp: "calc",
    arg: () => ({ text: /** @type {HTMLInputElement} */ (Q1("#calcin")).value, ans: CALC.ans }),
    after: (a, gs) => {
      if (a.kind !== "formula") return;
      const ad = gs.find(g => g.id === "addotti"), f = ad && ad.tab.formula, sum = Q1("#calcsum");
      if (sum) sum.innerHTML = f && a.text.replace(/\s+/g, "") !== f ? I18N.t("calc.interpreted", { formula: fmtFormula(f) }) : "";
    },
  });
  Object.assign(CALCQUI, { mount, apri, views, sync, render, later, calc });
})();
