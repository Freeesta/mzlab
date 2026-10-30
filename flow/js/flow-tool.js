// @ts-check
// Flow (administrator mode): registers the tab. Loaded with Tabui's package only in the administrator mode (static/mzfinder/, tpmine-loader.js), FIRST in the run order.
// Opening the tab draws a small static skeleton at once; the modules the shell needs (CORE) and the descriptors come in the background, the rest (LATE: results, review,
// runners) after the first draw. Nothing heavy runs before the first draw. English only. Docs: docs/agenti/flow-grafica.md («Avvio»).
(() => {
  "use strict";
  const T = /** @type {any} */ (window).QTOOLS;
  if (!T) return;
  const ICON = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="3" y="4" width="6" height="5" rx="1"/><rect x="15" y="15" width="6" height="5" rx="1"/><path d="M9 6.5h3a3 3 0 0 1 3 3v6"/></svg>';
  // what the shell needs to draw (the order is the order of execution)
  const CORE = ["flow-registry.js", "flow-check.js", "flow-plan.js", "flow-inputs.js", "flow-ui.js", "flow-palette.js", "flow-form.js", "flow-liste.js", "flow-compare.js", "flow-chain.js", "flow-page.js", "flow-names.js", "flow-fix.js", "flow-canvas.js", "flow-inspect.js", "flow-issues.js", "flow-shell-css.js", "flow-shell.js"];
  // what is used only after a click or when a result arrives; the shell hooks guard for it and redraw when it is there (F.whenLate)
  const LATE = ["flow-results.js", "flow-run-end.js", "flow-meta-grid.js", "flow-samples.js", "flow-parent.js", "flow-import.js", "flow-raw-files.js", "flow-xlsx.js", "flow-report.js", "flow-select.js", "flow-views-attributio.js", "flow-views-msn.js", "flow-msn-graph.js", "flow-web-msn.js", "flow-web-tp.js", "flow-web-ident.js", "flow-web-qc.js", "flow-web-suspects.js", "flow-web-insilico.js", "flow-web-drift.js", "flow-web-inclusion.js", "flow-web-qcreport.js", "flow-views-qc.js", "flow-msn-molecule.js", "flow-campaign.js", "flow-digest.js", "flow-review.js", "flow-decisions.js", "flow-family-proofs.js", "flow-views-features.js", "flow-views-tabui.js", "flow-views-tptab.js", "flow-desktop.js", "flow-web-run.js", "flow-live-view.js", "flow-live-card.js", "flow-live-bar.js", "flow-live-desktop.js", "flow-live-jobs.js", "flow-web-prov.js", "flow-live.js", "flow-web-ui.js", "flow-terminal.js", "flow-run-ui.js"];
  // not in these lists: flow-llm-chart.js, flow-llm-chart-ui.js, flow-llm-tools.js, flow-llm-readers.js, flow-llm-errors.js and flow-llm.js (the assistant) are fetched by flow-shell.js only when the profile, the settings and the consent allow it
  const LISTS = ["steps-1.json", "steps-attributio-1.json", "steps-tabui-1.json", "steps-cli-1.json"];
  const idle = (/** @type {() => void} */ fn) => { if ("requestIdleCallback" in window) /** @type {any} */ (window).requestIdleCallback(fn, { timeout: 1500 }); else setTimeout(fn, 150); };
  /** @type {Promise<any[][]> | null} */ let listsP = null;
  /** the four descriptor lists, fetched once @param {any} ctx */
  const getLists = ctx => listsP || (listsP = Promise.all(LISTS.map(async n => { const r = await fetch(new URL("mzfinder/" + n, ctx.base)); if (!r.ok) throw new Error(n); return r.json(); })).catch(e => { listsP = null; throw e; }));

  // the skeleton has the size of the real shell (bar, tabs, canvas, side box) so nothing jumps when it is replaced; colours come from the page variables
  const SK_CSS = ".flx-sk{display:flex;flex-direction:column;gap:8px;min-width:0}.flx-sk *{box-sizing:border-box}.flx-sk-bar{display:flex;flex-wrap:wrap;gap:8px;align-items:center}.flx-sk-bar h2{margin:0 4px 0 0;font-size:16px}"
    + ".flx-sk-bar button{min-height:28px;padding:2px 10px;border:1px solid var(--line);border-radius:6px;background:var(--panel);color:var(--muted);font:inherit}"
    + ".flx-sk-body{display:grid;grid-template-columns:minmax(0,1fr) minmax(0,1fr);gap:8px;align-items:start}.flx-sk-cv{border:1px solid var(--line);border-radius:10px;background:var(--panel);height:min(70vh,640px);min-height:340px}.flx-sk-side{border:1px solid var(--line);border-radius:10px;background:var(--panel);min-height:120px}"
    + "@media (max-width:900px){.flx-sk-body{grid-template-columns:minmax(0,1fr)}}";
  /** @param {HTMLElement} sec */
  const skeleton = sec => {
    if (!document.getElementById("flx-sk-css")) { const st = document.createElement("style"); st.id = "flx-sk-css"; st.textContent = SK_CSS; document.head.append(st); }
    const mk = (/** @type {string} */ tag, /** @type {string} */ cls, /** @type {string} */ text) => { const n = document.createElement(tag); if (cls) n.className = cls; if (text) n.textContent = text; return n; };
    const bar = mk("div", "flx-sk-bar", "");
    bar.append(mk("h2", "", "Flow"));
    for (const t of ["Canvas", "List", "Templates", "Add step", "Simple", "Advanced", "Undo", "Redo", "Fit", "Auto-run", "Run", "Terminal"]) { const b = /** @type {HTMLButtonElement} */ (mk("button", "", t)); b.type = "button"; b.disabled = true; bar.append(b); }
    const body = mk("div", "flx-sk-body", ""); body.append(mk("div", "flx-sk-cv", ""), mk("div", "flx-sk-side", ""));
    const root = mk("div", "flx-sk", ""); root.id = "flx-sk"; root.setAttribute("aria-busy", "true"); root.append(bar, body);
    sec.replaceChildren(root);
  };

  T.register({
    id: "flow", nav: true, name: "Flow", icon: ICON, desc: "Build a chain of steps from the step descriptors and export it as an mzflow/2 plan",
    /** @param {HTMLElement} sec @param {any} ctx */
    async open(sec, ctx) {
      skeleton(sec);
      const L = /** @type {any} */ (window).TPLAZY;
      try {
        const [lists] = await Promise.all([getLists(ctx), L.load(...CORE)]);
        const F = /** @type {any} */ (globalThis).FLOW;
        // the rest arrives after the first draw; whoever needs it waits on F.whenLate
        /** @type {(v?: any) => void} */ let go = () => {}; F.whenLate = new Promise(res => { go = res; }).then(() => L.whenLoaded()).then(() => L.load(...LATE)); F.whenLate.catch(() => {});
        F.injectCss();
        F.shell(sec, lists);
        idle(go);
      } catch (e) { sec.textContent = "Flow failed to load: " + (e && /** @type {any} */ (e).message || e); }
    },
  });
  // at once, before the page's own scripts queue up: the descriptors and the modules of the shell are fetched and run (they only define things), so the first click draws
  // from what is already there; the late modules are downloaded at low priority when the page is idle
  { const L = /** @type {any} */ (window).TPLAZY; if (L && T.ctx) { getLists(T.ctx).catch(() => {}); L.loadNow(...CORE).catch(() => {}); idle(() => L.warm(LATE, true)); } }
})();
