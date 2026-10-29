// @ts-check
// Flow (administrator mode): the report of a run and the network of the products. «Report» shows the `corsa/1` of the last run of the app (route `flow/report`) or one chosen from disk with RAPPORTO_FLOW
// (printable, with its JSON); the network view draws a `rete/1` with TPRETE_VIEW and joins the one selection of the results (F.select). Both scripts are loaded when a view first needs them.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW, G = /** @type {any} */ (window);

  /** loads a classic script of the page once, from the static folder of the page (QTOOLS.ctx.base) @param {string} file */
  const loadStatic = file => {
    const key = "flow-script-" + file;
    if (document.getElementById(key)) return /** @type {Promise<void>} */ (G[key]);
    const base = (G.QTOOLS && G.QTOOLS.ctx && G.QTOOLS.ctx.base) || document.baseURI;
    const s = document.createElement("script"); s.id = key; s.src = new URL(file, base).href;
    G[key] = new Promise((ok, no) => { s.onload = () => ok(); s.onerror = () => no(new Error(file + " could not be loaded")); });
    document.head.append(s);
    return /** @type {Promise<void>} */ (G[key]);
  };

  F.registerView({ id: "corsa", title: "Run report", types: ["report"], accepts: (/** @type {any} */ item) => /\.json$/i.test(item.label), open: async (/** @type {HTMLElement} */ host, /** @type {any} */ item) => {
    const { el } = F, say = el("p", { class: "fl-note", role: "status" }), page = el("div");
    host.append(say, page);
    let corsa = null;
    try { corsa = JSON.parse(await item.text()); } catch (_) { say.textContent = `${item.label} is not a run report (corsa/1).`; return; }
    if (corsa && corsa.formato === "msngrafo/1" && F.openMsnGraph) { say.remove(); F.openMsnGraph(page, corsa, item.label); return; }
    if (!corsa || corsa.schema !== "corsa/1") { say.textContent = `${item.label} is not a run report (corsa/1).`; return; }
    try { await loadStatic("rapporto-flow.js"); } catch (e) { say.textContent = String((e && /** @type {any} */ (e).message) || e); return; }
    G.RAPPORTO_FLOW.render(page, corsa);
  } });

  F.registerView({ id: "rete", title: "Network of the products", types: ["transform"], accepts: (/** @type {any} */ item) => /\.json$/i.test(item.label), open: async (/** @type {HTMLElement} */ host, /** @type {any} */ item) => {
    const { el } = F, say = el("p", { class: "fl-note", role: "status" }), page = el("div", { class: "tp" });
    host.append(say, page);
    let rete = null;
    try { rete = JSON.parse(await item.text()); } catch (_) { say.textContent = `${item.label} is not a product network (rete/1).`; return; }
    if (!rete || rete.schema !== "rete/1") { say.textContent = `${item.label} is not a product network (rete/1).`; return; }
    try { await G.TPLAZY.load("rete-vista.js"); } catch (e) { say.textContent = "The network view could not be loaded."; return; }
    const view = G.TPRETE_VIEW.mount(page, rete, { onSelect: (/** @type {number | null} */ row) => F.select(row == null ? null : String(row), "rete") });
    const stop = F.onSelect((/** @type {string | null} */ id, /** @type {string} */ from) => { if (!page.isConnected) { stop(); view.destroy(); return; } if (from !== "rete") view.selectRow(id == null ? null : Number(id)); });
    if (F.selected != null) view.selectRow(Number(F.selected));
  } });

  /** the report of the last run of the app as an item for the results; rejects when the app has no such run @returns {Promise<any>} */
  F.lastRunReport = async () => {
    const text = F.runners.desktop.available() ? await F.runners.desktop.report() : F.webProv && F.webProv.last ? F.webProv.last.corsa : await F.runners.desktop.report();
    return { type: "report", label: "corsa.json", text: async () => text };
  };
})();
