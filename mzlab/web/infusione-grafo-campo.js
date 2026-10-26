// @ts-strict
// The «Ion formula» field of the «Fragmentation graph»: a formula typed, or a molecule chosen in the search box of the molecule database; the adduct for the
// polarity of the file and the SMILES. The text goes to the engine as it is; a chosen molecule only fills the field (formula + adduct, SMILES).
import { ADDUCTS, G, polarityOf, startAdduct } from "./infusione-grafo-stato.js";
import { campo } from "./molecole-campo.js";
import { prendi } from "./molecole-archivio.js";

/** @typedef {import("./infusione-grafo-stato.js").Molecola} Molecola */

const optionsOf = (/** @type {readonly string[]} */ list, /** @type {string} */ cur) => list.map(a => raw(h`<option value="${a}"${raw(a === cur ? " selected" : "")}>${a}</option>`));

/** The text of the ion field for the molecule in the state: its formula and the adduct. */
const ionOf = () => (G.mol ? `${G.mol.formula} ${G.adduct}` : G.text);

/** Draws the field in the host. `go()` is called when the user asks for the graph (Enter or the button). */
export function drawField(/** @type {HTMLElement} */ host, /** @type {any} */ file, /** @type {() => void} */ go) {
  const pol = polarityOf(file), list = ADDUCTS[pol];
  if (!list.includes(G.adduct)) G.adduct = startAdduct(pol, G.mol);
  host.innerHTML = h`<div class="row ig-field" id="ig-mol"></div>` +
    h`<div class="row ig-field"><label>${I18N.t("msngrafo.ion")} <input id="ig-ion" type="text" value="${ionOf()}" autocomplete="off" spellcheck="false" title="${I18N.t("msngrafo.ion.tip")}"></label>` +
    h`<label>${I18N.t("msngrafo.adduct")} <select id="ig-add">${optionsOf(list, G.adduct)}</select></label><button type="button" class="go" id="ig-go">${I18N.t("msngrafo.go")}</button><button type="button" class="go" id="ig-flow" title="${I18N.t("msngrafo.flow.tip")}">${I18N.t("msngrafo.flow")}</button></div>` +
    h`<div class="row ig-field"><label>${I18N.t("msngrafo.smiles")} <input id="ig-smi" type="text" value="${G.smiles}" autocomplete="off" spellcheck="false"></label><span id="ig-ionline" class="sm muted" aria-live="polite"></span></div>`;
  const ion = /** @type {HTMLInputElement} */ (host.querySelector("#ig-ion")), add = /** @type {HTMLSelectElement} */ (host.querySelector("#ig-add"));
  const smi = /** @type {HTMLInputElement} */ (host.querySelector("#ig-smi"));
  add.onchange = () => { G.adduct = add.value; if (G.mol) ion.value = ionOf(); go(); };
  smi.onchange = () => { G.smiles = smi.value.trim(); go(); };
  host.querySelector("#ig-go")?.addEventListener("click", () => { commit(ion); go(); });
  host.querySelector("#ig-flow")?.addEventListener("click", () => { commit(ion); G.auto = true; go(); });
  ion.addEventListener("keydown", e => { if (e.key === "Enter") { e.preventDefault(); commit(ion); go(); } });
  campo(/** @type {HTMLElement} */ (host.querySelector("#ig-mol")), {
    testi: { label: I18N.t("msngrafo.mol"), placeholder: I18N.t("msngrafo.mol.ph"), none: I18N.t("msngrafo.mol.none"), count: n => I18N.t("msngrafo.mol.count", { n }), manage: I18N.t("msngrafo.mol.manage") },
    onScegli: async row => {
      const m = /** @type {Molecola} */ (/** @type {unknown} */ ((await prendi(row.id)) || row));
      // a mass-only entry has no formula to write: the graph then shows masses only
      G.mol = m.formula ? m : null; G.text = ""; G.smiles = m.smiles || ""; G.adduct = startAdduct(polarityOf(E.files.find(INF.isInf)), m);
      ion.value = ionOf(); add.value = G.adduct; smi.value = G.smiles;
      go();
    },
  });
}

/** The typed text becomes the ion (and forgets a molecule chosen in the search box when the text no longer is its formula and adduct). */
function commit(/** @type {HTMLInputElement} */ ion) {
  const t = ion.value.trim();
  if (G.mol && t === ionOf()) return;
  G.mol = null; G.text = t;
}
