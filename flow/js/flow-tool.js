// @ts-check
// Flow (administrator mode): registers the tab. Loaded with Tabui's package only in the administrator mode (static/mzfinder/, tpmine-loader.js); the page, the checker
// and the descriptors are fetched when the tab is first opened. English only. Docs: docs/agenti/flow-grafica.md.
(() => {
  "use strict";
  const T = /** @type {any} */ (window).QTOOLS;
  if (!T) return;
  const ICON = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="3" y="4" width="6" height="5" rx="1"/><rect x="15" y="15" width="6" height="5" rx="1"/><path d="M9 6.5h3a3 3 0 0 1 3 3v6"/></svg>';
  const PARTS = ["flow-registry.js", "flow-check.js", "flow-plan.js", "flow-inputs.js", "flow-ui.js", "flow-palette.js", "flow-form.js", "flow-compare.js", "flow-chain.js", "flow-page.js", "flow-names.js", "flow-fix.js", "flow-canvas.js", "flow-inspect.js", "flow-shell-css.js", "flow-shell.js", "flow-results.js", "flow-views-attributio.js", "flow-campaign.js", "flow-digest.js", "flow-review.js", "flow-views-features.js", "flow-views-tabui.js", "flow-desktop.js"];
  T.register({
    id: "flow", nav: true, admin: true, name: "Flow", icon: ICON, desc: "Build a chain of steps from the step descriptors and export it as an mzflow/2 plan",
    /** @param {HTMLElement} sec @param {any} ctx */
    async open(sec, ctx) {
      sec.textContent = "Loading Flow…";
      try {
        const lists = await Promise.all(["steps-1.json", "steps-attributio-1.json", "steps-tabui-1.json"].map(async n => { const r = await fetch(new URL("mzfinder/" + n, ctx.base)); if (!r.ok) throw new Error(n); return r.json(); }));
        await /** @type {any} */ (window).TPLAZY.load(...PARTS);
        /** @type {any} */ (globalThis).FLOW.injectCss();
        /** @type {any} */ (globalThis).FLOW.shell(sec, lists);
      } catch (e) { sec.textContent = "Flow failed to load: " + (e && /** @type {any} */ (e).message || e); }
    },
  });
})();
