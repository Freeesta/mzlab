// @ts-strict
// @ts-noany
// The outside lookups of the canvas menu (PubChem, ChEBI, BioTransformer) for the selection or the whole drawing. URLs, row order and what to copy
// are the `link/1` of the Rust graph (crates/mzlab-feat chem::link, through the `.wasm` `grafo` or the app's `feat/grafo`). The page never fetches
// them: a row opens its URL in a new tab (noopener, noreferrer) only when clicked, copying the SMILES first when the site takes no structure in its URL.

import { carica } from "./disegno-frammenti.js";
import { copyText } from "./disegno-menu.js";

/** @typedef {{ id: string, url: string, copy: string | null }} Link */
/** @typedef {{ format: string, part: boolean, preferred: string, links: Link[] }} Links */
/** @typedef {{ text: string, title: string, id: string, run: () => void }} Row */

/** `link/1` of a structure, or null when the engine cannot answer */
export async function linkOf(/** @type {string} */ smiles, /** @type {string} */ inchi, /** @type {boolean} */ part) {
  try { return /** @type {Links} */ (JSON.parse(await (await carica()).link(JSON.stringify({ smiles, inchi, part })))); } catch (_) { return null; }
}

/** opens `url` in a new tab cut off from this page (no opener, no referrer) */
export function openOut(/** @type {string} */ url) { window.open(url, "_blank", "noopener,noreferrer"); }

/**
 * The menu rows: a «PubChem» submenu that starts with the exact structure (then substructure and similar, in the engine's order) and an «Other sites» submenu
 * (ChEBI when Indigo gives the InChI, BioTransformer). The copy starts inside the click, before the new tab takes the focus.
 */
export async function linkRows(/** @type {string} */ smiles, /** @type {boolean} */ part, /** @type {() => Promise<string>} */ inchi) {
  let i = "";
  try { i = await inchi(); } catch (_) { /* no InChI: no ChEBI row */ }
  const r = await linkOf(smiles, i, part);
  if (!r) return [];
  const rows = r.links.map(l => /** @type {Row} */ ({
    id: "link-" + l.id, text: I18N.t(`draw.link.${l.id}`), title: I18N.t("draw.link.title", { host: new URL(l.url).host }),
    run: () => { if (l.copy) copyText(l.copy, `draw.link.copied.${l.id}`); openOut(l.url); }
  }));
  const pub = rows.filter(x => x.id.startsWith("link-pubchem-")).sort((a, b) => +(b.id === "link-pubchem-exact") - +(a.id === "link-pubchem-exact"));
  return [{ id: "link-pubchem", text: I18N.t("draw.link.pubchem"), title: I18N.t("draw.link.pubchem.title"), sub: pub },
    { id: "link-other", text: I18N.t("draw.link.other"), title: I18N.t("draw.link.other.title"), sub: rows.filter(x => !x.id.startsWith("link-pubchem-")) }];
}
