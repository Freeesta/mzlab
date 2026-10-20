// @ts-check
// Flow (administrator mode): one selection for all the views of a result. F.select(id, from) says which row is selected (a feature, a product), every view that shows it listens with F.onSelect and marks it
// with a thick outline (not colour alone); the tables of the views publish a click on a row through F.watchRows. Leaving the results forgets it.
(() => {
  "use strict";
  const F = /** @type {any} */ (globalThis).FLOW;
  /** @type {Set<(id: string | null, from: string) => void>} */ const listeners = new Set();
  F.selected = /** @type {string | null} */ (null);

  /** selects the row `id` (null clears it); `from` names the view that asked, so it can skip redrawing itself @param {string | null} id @param {string} [from] */
  F.select = (id, from = "") => {
    if (id === F.selected) return;
    F.selected = id; listeners.forEach(fn => fn(id, from));
  };
  /** calls `fn` on every change of the selection; returns the way to stop @param {(id: string | null, from: string) => void} fn */
  F.onSelect = fn => { listeners.add(fn); return () => listeners.delete(fn); };

  /** marks the row `id` of a table of `host` (`tr[data-id]`) and unmarks the others; the mark is an outline and `aria-current`, so it is not colour alone @param {HTMLElement} host @param {string | null} id */
  F.markRow = (host, id) => {
    host.querySelectorAll("tr[data-id]").forEach(tr => {
      const on = id != null && /** @type {HTMLElement} */ (tr).dataset.id === id;
      tr.classList.toggle("flx-sel", on);
      if (on) tr.setAttribute("aria-current", "true"); else tr.removeAttribute("aria-current");
    });
  };

  /** the rows of the tables of `host` publish their id on a click and show the selection of the other views; `name` says who asks @param {HTMLElement} host @param {string} name */
  F.watchRows = (host, name) => {
    host.addEventListener("click", e => {
      const tr = /** @type {HTMLElement} */ (e.target).closest("tr[data-id]");
      if (tr && host.contains(tr) && !(/** @type {HTMLElement} */ (e.target)).closest("a,input,select,textarea")) F.select(/** @type {HTMLElement} */ (tr).dataset.id || null, name);
    });
    const stop = F.onSelect((/** @type {string | null} */ id) => { if (!host.isConnected) { stop(); return; } F.markRow(host, id); });
    new MutationObserver(() => F.markRow(host, F.selected)).observe(host, { childList: true, subtree: true });
    F.markRow(host, F.selected);
  };
})();
