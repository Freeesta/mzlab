// @ts-strict
// @ts-noany
// «Info…» of the canvas menu: what the selection (or the whole drawing) is, as drawn. The numbers are the `sel/1` of the Rust graph (crates/mzlab-feat
// chem::sel, through the `.wasm` `grafo` or the app's `feat/grafo`), fed with the V2000 molfile of the piece: open valences are filled with H. InChI and
// InChIKey come from Indigo (Ketcher). Loaded on the first «Info…» click; the page only lays the values out.

import { carica } from "./disegno-frammenti.js";
import { copyText } from "./disegno-menu.js";

/** @typedef {{ formula: string, text: string, charge: number, mass: number, mz?: number, unpaired: number }} Label */
/** @typedef {{ format: string, part: boolean, label: Label, average?: number, heavy: number, atoms: number, pieces: number, rings: number, aromaticAtoms: number, aromaticRings: number }} Sel */
/** @typedef {{ molfile: string, part: boolean, smiles: string, convert: (format: string) => Promise<string>, big: (title: string, html: string, onopen?: () => void) => void, esc: (s: string) => string }} Helpers */

/** `sel/1` of a molfile, or null when the engine cannot answer */
export async function selOf(/** @type {string} */ molfile, /** @type {boolean} */ part) {
  try { return /** @type {Sel} */ (JSON.parse(await (await carica()).sel(JSON.stringify({ molfile, part })))); } catch (_) { return null; }
}

const t = (/** @type {string} */ k, /** @type {Record<string, unknown>} */ p = {}) => I18N.t(k, p);
const fix = (/** @type {number} */ v, /** @type {number} */ d) => v.toFixed(d);

/** the rows of the dialog: [label, shown value, value to copy or ""] */
export function infoRows(/** @type {Sel} */ r) {
  const l = r.label, rows = [[t("draw.info.formula"), l.text, l.text], [t("draw.info.exactMass"), fix(l.mass, 5), fix(l.mass, 5)]];
  if (l.charge && l.mz !== undefined) rows.push([t("draw.info.mz"), fix(l.mz, 5), fix(l.mz, 5)]);
  if (r.average !== undefined) rows.push([t("draw.info.average"), fix(r.average, 2), ""]);
  rows.push([t("draw.info.charge"), l.charge > 0 ? "+" + l.charge : String(l.charge), ""], [t("draw.info.heavy"), String(r.heavy), ""],
    [t("draw.info.rings"), t("draw.info.ringsValue", { n: r.rings, a: r.aromaticRings }), ""]);
  if (l.unpaired) rows.push([t("draw.info.unpaired"), String(l.unpaired), ""]);
  if (r.pieces > 1) rows.push([t("draw.info.pieces"), String(r.pieces), ""]);
  return rows;
}

/** opens the dialog; InChI and InChIKey are filled when Indigo answers */
export async function infoOpen(/** @type {Helpers} */ h) {
  const r = await selOf(h.molfile, h.part);
  if (!r) { if (window.toast) toast(t("draw.info.none")); return; }
  const E = h.esc, btn = (/** @type {string} */ id, /** @type {string} */ what) =>
    `<button class="sm" data-info-cp="${id}" aria-label="${E(t("draw.info.copyOf", { what }))}">${E(t("draw.copy.button"))}</button>`;
  const vals = /** @type {Record<string, string>} */ ({ smiles: h.smiles, molfile: h.molfile, inchi: "", inchikey: "" });
  const row = (/** @type {string} */ k, /** @type {string} */ v, /** @type {string} */ id) => `<tr><th scope="row">${E(k)}</th><td><code id="info-${id}" style="word-break:break-all">${E(v)}</code></td><td>${btn(id, k)}</td></tr>`;
  const num = infoRows(r).map((x, i) => {
    if (x[2]) vals["n" + i] = x[2];
    return `<tr><th scope="row">${E(x[0])}</th><td>${E(x[1])}</td><td>${x[2] ? btn("n" + i, x[0]) : ""}</td></tr>`;
  }).join("");
  const ids = [["SMILES", h.smiles, "smiles"], ["InChI", "…", "inchi"], ["InChIKey", "…", "inchikey"]].filter(x => x[2] !== "smiles" || h.smiles);
  const html = (h.part ? `<p class="muted sm" id="info-part">${E(t("draw.info.partNote"))}</p>` : "") +
    `<table class="info-tab" aria-label="${E(t("draw.info.title"))}"><tbody>${num}${ids.map(x => row(x[0], x[1], x[2])).join("")}` +
    `<tr><th scope="row">Molfile</th><td class="muted sm">${E(t("draw.info.molfileNote"))}</td><td>${btn("molfile", "Molfile")}</td></tr></tbody></table>` +
    `<style>.info-tab th{text-align:left;padding:2px 10px 2px 0;font-weight:600}.info-tab td{padding:2px 6px 2px 0}.info-tab button{min-height:24px;min-width:24px}html.touch .info-tab button{min-height:40px}</style>`;
  h.big(t(h.part ? "draw.info.titleSel" : "draw.info.titleWhole"), html, () => {
    document.querySelectorAll("[data-info-cp]").forEach(b => { /** @type {HTMLElement} */ (b).onclick = () => { const v = vals[/** @type {HTMLElement} */ (b).dataset.infoCp || ""]; if (v) copyText(v); }; });
  });
  for (const [id, fmt] of [["inchi", "chemical/x-inchi"], ["inchikey", "chemical/x-inchi-key"]]) {
    let v = "";
    try { v = String(await h.convert(fmt)).trim(); } catch (_) { /* Indigo cannot write it */ }
    vals[id] = v;
    const c = document.getElementById("info-" + id);
    if (c) c.textContent = v || t("draw.info.notAvailable");
  }
}
