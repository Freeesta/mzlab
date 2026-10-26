// @ts-strict
// The graph of the «Fragmentation graph» tab: a thin adapter between the state of the tab (view, filters, rule labels, molecule drawings) and the pure drawer
// msngrafo-vista.js, which the Flow and the stand-alone HTML use too. Selection, path from the root, keys, zoom and move: see the drawer.
import { G, TXT, chosen } from "./infusione-grafo-stato.js";
import { arcLabel } from "./infusione-grafo-regole.js";
import { drawVista } from "./msngrafo-vista.js";
import { drawingsOf } from "./infusione-grafo-mol.js";

/** @typedef {(key: string, p?: Record<string, any>) => string} Tr */
/** The texts of the tab (the language of the page); the Flow hands in its own English function. @type {Tr} */
const TR = (key, p) => I18N.t(/** @type {any} */ (key), p);

/**
 * Draws the graph in the host and returns the SVG; `cb.onSelect(id|null)` on a click, arrow keys or Esc, `cb.onOpen(id)` on a click or Enter.
 * `opts.t` and `opts.sel` replace the texts and the selected ion of the tab (the Flow draws with its own, in English, tree view, no filters and no drawings).
 */
export function drawGraph(/** @type {HTMLElement} */ host, /** @type {any} */ j, /** @type {{ onSelect: (id: string | null) => void, onOpen: (id: string) => void }} */ cb, /** @type {{ t?: Tr, sel?: string | null, mode?: "tree" | "radial" }} */ opts = {}) {
  const own = !opts.t, t = opts.t || TR, sel = "sel" in opts ? opts.sel || null : G.sel;
  const dr = own ? drawingsOf() : { root: null, byId: new Map() };
  return drawVista(host, j, cb, {
    t, sel, mode: opts.mode || (own ? G.view : "tree"), minRel: own ? G.minRel : 0, maxLevel: own ? G.maxLevel : 0, bestOnly: own && G.bestOnly, collapsed: own ? G.collapsed : new Set(),
    chosen, proveText: p => t(/** @type {any} */ (TXT.prove)[p]), ruleLabel: a => arcLabel(j, a, t), drawing: id => dr.byId.get(id) || null, root: dr.root,
  }).svg;
}
