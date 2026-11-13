// @ts-check
// «Evidence colours» of Draw: other pages (and the substructure card) ask `TPDraw.highlight(layers)` to colour on the canvas the atoms and bonds a piece of
// evidence points at: a moiety (`moiety`), reactive sites (`sites`) or cuts supported by peaks (`cuts`). The Rust `hilite/1` (`hilite` of the molkit .wasm;
// the app asks `feat/grafo`) carries them onto the drawing; this file only paints them with Ketcher's highlights in Okabe-Ito colours and lists them in a
// legend and a data table, so colour is never the only channel. In the LR world only the atoms the student selected are coloured.
import { ask, ketcher, message } from "./disegno-verifica.js";
import { molfileOf } from "./disegno-molfile.js";

const t = (/** @type {string} */ k, /** @type {Record<string, any>} */ p = {}) => I18N.t(k, p);
/** @type {Record<string, string>} */ const KIND = { moiety: "draw.ev.kind.moiety", sites: "draw.ev.kind.sites", cuts: "draw.ev.kind.cuts" };
/** @type {Record<string, string>} */ const SKIP = { noMatch: "draw.ev.skip.noMatch", lr: "draw.ev.skip.lr", tooBig: "draw.ev.skip.tooBig" };
/** @type {any} */ let last = null;           // the last hilite/1 answer with the names of its atoms
let painted = false, listening = false;

/** HR world: a high-resolution file open with the HR tools on (as the bar decides) */
const hrWorld = () => !!(/** @type {any} */ (window).HR && typeof BANCO !== "undefined" && BANCO.on());
const colour = (/** @type {string} */ kind) => getComputedStyle(document.documentElement).getPropertyValue("--ev-" + kind).trim();

/** the card, after the CIP card (or the check card), hidden until the first request */
function card() {
  /** @type {HTMLElement | null} */ let c = document.querySelector("#ev-card");
  if (c) return c;
  c = document.createElement("div");
  c.className = "card"; c.id = "ev-card"; c.hidden = true;
  c.innerHTML = '<h2 data-i18n="draw.ev.heading"></h2>'
    + '<div id="ev-status" class="sm" role="status"></div><ul id="ev-legend" class="sm"></ul>'
    + '<div style="display:flex;gap:6px;flex-wrap:wrap"><button type="button" class="sm" id="ev-table" data-i18n="draw.ev.table"></button>'
    + '<button type="button" class="sm" id="ev-clear" data-i18n="draw.ev.clear"></button></div>';
  c.querySelectorAll("[data-i18n]").forEach(e => { e.textContent = t(/** @type {string} */ (e.getAttribute("data-i18n"))); });
  /** @type {HTMLElement} */ (c.querySelector("#ev-legend")).setAttribute("aria-label", t("draw.ev.table.caption"));
  const css = document.createElement("style");
  // Okabe-Ito on the light canvas; lighter ones on a dark panel and stronger ones in high contrast, with the same selectors as app.css
  const light = "--ev-moiety:#0072B2;--ev-sites:#009E73;--ev-cuts:#D55E00", dark = "--ev-moiety:#56B4E9;--ev-sites:#33C69B;--ev-cuts:#F07F4A";
  css.textContent = `:root{${light}}@media (prefers-color-scheme:dark){:root:not([data-theme=light]){${dark}}}:root[data-theme=dark]{${dark}}`
    + ":root[data-a11y-bg=hc][data-theme=light],:root[data-a11y-bg=hc]:not([data-theme=dark]){--ev-moiety:#005A8C;--ev-sites:#00664A;--ev-cuts:#A63A00}"
    + ":root[data-a11y-bg=hc][data-theme=dark]{--ev-moiety:#8FD0FF;--ev-sites:#6EE7A8;--ev-cuts:#FFA070}"
    + "@media (forced-colors:active){#ev-legend .ev-sw{forced-color-adjust:none;border-color:CanvasText}}"
    + "#ev-legend{list-style:none;padding:0;margin:4px 0}#ev-legend li{margin:2px 0;display:flex;gap:6px;align-items:center}"
    + "#ev-legend .ev-sw{display:inline-block;width:14px;height:14px;border-radius:3px;border:1px solid var(--line);flex:none}"
    + "#ev-card button{min-height:24px;min-width:24px}";
  c.appendChild(css);
  const after = document.querySelector("#cip-card") || document.querySelector("#chk-card") || document.querySelector("#anal-card");
  if (after && after.parentNode) after.parentNode.insertBefore(c, after.nextSibling);
  else document.body.appendChild(c);
  /** @type {HTMLElement} */ (c.querySelector("#ev-clear")).onclick = () => { clear().catch(() => { /* the canvas went away */ }); };
  /** @type {HTMLElement} */ (c.querySelector("#ev-table")).onclick = dataTable;
  return c;
}

/** the legend: one row per layer, colour swatch plus kind, label and what was coloured (or why nothing) */
function legend(/** @type {any} */ r) {
  const ul = /** @type {HTMLElement} */ (document.querySelector("#ev-legend"));
  ul.textContent = "";
  for (const l of r.layers) {
    const li = document.createElement("li"), sw = document.createElement("span");
    sw.className = "ev-sw"; sw.style.background = `var(--ev-${l.kind})`; sw.setAttribute("aria-hidden", "true");
    const what = l.skip ? t(SKIP[l.skip] || SKIP.noMatch) : t("draw.ev.count", { atoms: l.atoms.length, bonds: l.bonds.length });
    li.append(sw, t("draw.ev.row", { kind: t(KIND[l.kind]), label: l.label || "—", what }));
    ul.appendChild(li);
  }
}

/** every coloured atom and bond as a real table (TDATI) */
function dataTable() {
  if (!last || !(/** @type {any} */ (window).TDATI)) return;
  const rows = /** @type {string[][]} */ ([]);
  for (const l of last.r.layers) {
    const kind = t(KIND[l.kind]), label = l.label || "—";
    for (const a of l.atoms) rows.push([kind, label, t("draw.ev.atom"), last.name(a)]);
    for (const [a, b] of l.bonds) rows.push([kind, label, t("draw.ev.bond"), last.name(a) + "–" + last.name(b)]);
    if (!l.atoms.length && !l.bonds.length) rows.push([kind, label, "—", t(SKIP[l.skip] || SKIP.noMatch)]);
  }
  /** @type {any} */ (window).TDATI.table({ title: t("draw.ev.table.caption"), head: [t("draw.ev.col.kind"), t("draw.ev.col.label"), t("draw.ev.col.what"), t("draw.ev.col.where")], rows });
}

/** removes the colours (the drawing is left as it is) */
export async function clear() {
  const K = await ketcher();
  if (painted) { try { K.editor.highlights.clear(); } catch (_) { /* an older editor */ } }
  painted = false; last = null;
  const c = document.querySelector("#ev-card");
  if (c) { /** @type {HTMLElement} */ (c.querySelector("#ev-legend")).textContent = ""; /** @type {HTMLElement} */ (c.querySelector("#ev-status")).textContent = t("draw.ev.cleared"); }
}

/**
 * Colours the evidence `layers` (`{kind: "moiety", query} | {kind: "sites", smiles, atoms} | {kind: "cuts", smiles, bonds}`, each with an optional `label`)
 * on the drawing and fills the card; resolves with the `hilite/1` answer, or null when the canvas is empty or the request fails (the card says why).
 */
export async function highlight(/** @type {any[]} */ layers) {
  const c = card(), status = /** @type {HTMLElement} */ (c.querySelector("#ev-status"));
  c.hidden = false;
  try {
    const K = await ketcher(), st = K.editor.struct(), ids = [...st.atoms.keys()];
    if (painted) { try { K.editor.highlights.clear(); } catch (_) { /* nothing painted */ } painted = false; }
    if (!ids.length) { status.textContent = t("draw.ev.empty"); /** @type {HTMLElement} */ (c.querySelector("#ev-legend")).textContent = ""; return null; }
    const sel = (K.editor.selection() || {}).atoms || [], world = hrWorld() ? "hr" : "lr";
    const req = { molfile: molfileOf(st, ids, undefined, () => true), world, selected: sel.map((/** @type {number} */ a) => ids.indexOf(a)).filter((/** @type {number} */ i) => i >= 0),
      layers: layers.filter(l => l && KIND[l.kind]) };
    const r = JSON.parse(await ask(JSON.stringify(req), "hilite"));
    for (const l of r.layers) {
      const atoms = l.atoms.map((/** @type {number} */ i) => ids[i]), bonds = /** @type {number[]} */ ([]);
      st.bonds.forEach((/** @type {any} */ b, /** @type {number} */ id) => { if (l.bonds.some((/** @type {number[]} */ p) => (ids[p[0]] === b.begin && ids[p[1]] === b.end) || (ids[p[0]] === b.end && ids[p[1]] === b.begin))) bonds.push(id); });
      if (atoms.length || bonds.length) { K.editor.highlights.create({ atoms, bonds, rgroupAttachmentPoints: [], color: colour(l.kind) }); painted = true; }
    }
    last = { r, name: (/** @type {number} */ i) => (st.atoms.get(ids[i]).label || "") + (i + 1) };
    legend(r);
    const n = r.layers.filter((/** @type {any} */ l) => !l.skip).length;
    status.textContent = world === "lr" ? t("draw.ev.done.lr") : t("draw.ev.painted", { n, total: r.layers.length });
    if (!listening) {
      listening = true;
      K.editor.subscribe("change", () => { if (painted) clear().catch(() => { /* the canvas went away */ }); });
    }
    return r;
  } catch (e) {
    status.textContent = t("draw.ev.fail", { message: message(e) });
    return null;
  }
}
