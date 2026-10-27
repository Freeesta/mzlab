// @ts-check
// Flow (administrator mode): the molecule box in the form of spectra.msn_graph. The search box of the molecules database (molecole-campo.js, the one the Infusion tab uses) fills the ion of the root,
// its SMILES, its adduct and its name in the parameters of the step, so the plan needs only the infusion file and a molecule; the fields below stay editable. Docs: docs/agenti/flow-web.md.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);
  /** the app module `name` (loaded once, the same instance as the app's) @param {string} name */
  const app = name => import(/* webpackIgnore: true */ new URL(name, G.QTOOLS && G.QTOOLS.ctx ? G.QTOOLS.ctx.base : document.baseURI).href);

  /** the box for the instance `inst` (spectra.msn_graph): [] when the page cannot give the search box @param {any} inst @param {any} step @param {() => void} done */
  F.msnMolecule = (inst, step, done) => {
    const { el } = F, host = el("div", { class: "fl-meta", style: "grid-column:1/-1", "data-fk": "fl-msn-mol" });
    const set = (/** @type {string} */ name, /** @type {any} */ v) => F.setParam(inst, step, name, v === "" ? null : v);
    Promise.all([app("molecole-campo.js"), app("molecole-archivio.js")]).then(([c, a]) => {
      c.campo(host, {
        testi: { label: "Molecule", placeholder: "name, formula or mass", none: "No molecule found", count: (/** @type {number} */ n) => `${n} found` },
        onScegli: async (/** @type {any} */ row) => {
          const m = (await a.prendi(row.id)) || row;
          const adduct = (m.addotti && m.addotti[0]) || "[M+H]+";
          // a mass-only entry has no formula: the graph then shows masses only
          set("molecule", m.nome || m.name || ""); set("adduct", adduct); set("smiles", m.smiles || "");
          set("formula", m.formula ? `${m.formula} ${adduct}` : "");
          done();
        },
      });
    }).catch(() => { host.textContent = "Molecule search unavailable: type the ion in the fields below."; });
    return host;
  };
})();
