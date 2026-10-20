// @ts-check
// The «Ion formula» field of the «Fragmentation graph»: a formula typed or a name looked up in the molecule database (api/molecules/cerca and /get), the
// adduct for the polarity of the file and the SMILES. The text goes to the engine as it is; a name only fills the field (formula + adduct, SMILES).
import { ADDUCTS, G, polarityOf, startAdduct } from "./infusione-grafo-stato.js";

/** @typedef {import("./infusione-grafo-stato.js").Molecola} Molecola */

/** Looks a name up: up to 8 molecules, or [] when the database is not there (asked once, then never again). @returns {Promise<Molecola[]>} */
async function searchNames(/** @type {string} */ q) {
  if (G.namesOff || q.trim().length < 2) return [];
  try { return await J(`api/molecules/cerca?q=${encodeURIComponent(q.trim())}&limite=8&ppm=10`); } catch (_) { G.namesOff = true; return []; }
}

/** The whole record of a molecule (the search row when the route fails). @returns {Promise<Molecola>} */
async function fullRecord(/** @type {Molecola} */ row) {
  try { return await J(`api/molecules/get?id=${encodeURIComponent(row.id)}`); } catch (_) { return row; }
}

const optionsOf = (/** @type {readonly string[]} */ list, /** @type {string} */ cur) => list.map(a => raw(h`<option value="${a}"${raw(a === cur ? " selected" : "")}>${a}</option>`));

/** Draws the field in the host. `go()` is called when the user asks for the graph (Enter or the button). */
export function drawField(/** @type {HTMLElement} */ host, /** @type {any} */ file, /** @type {() => void} */ go) {
  const pol = polarityOf(file), list = ADDUCTS[pol];
  if (!list.includes(G.adduct)) G.adduct = startAdduct(pol, G.mol);
  host.innerHTML = h`<div class="row ig-field"><label>${I18N.t("msngrafo.ion")} <input id="ig-ion" type="text" value="${G.mol ? G.mol.nome : G.text}" autocomplete="off" role="combobox" aria-expanded="false" aria-controls="ig-sug" title="${I18N.t("msngrafo.ion.tip")}"></label>` +
    h`<label>${I18N.t("msngrafo.adduct")} <select id="ig-add">${optionsOf(list, G.adduct)}</select></label><button type="button" class="go" id="ig-go">${I18N.t("msngrafo.go")}</button></div>` +
    h`<ul id="ig-sug" class="ig-sug" role="listbox" hidden aria-label="${I18N.t("msngrafo.sug.aria")}"></ul>` +
    h`<div class="row ig-field"><label>${I18N.t("msngrafo.smiles")} <input id="ig-smi" type="text" value="${G.smiles}" autocomplete="off" spellcheck="false"></label><span id="ig-ionline" class="sm muted" aria-live="polite"></span></div>`;
  const ion = /** @type {HTMLInputElement} */ (host.querySelector("#ig-ion")), sug = /** @type {HTMLElement} */ (host.querySelector("#ig-sug"));
  const add = /** @type {HTMLSelectElement} */ (host.querySelector("#ig-add")), smi = /** @type {HTMLInputElement} */ (host.querySelector("#ig-smi"));
  add.onchange = () => { G.adduct = add.value; go(); };
  smi.onchange = () => { G.smiles = smi.value.trim(); go(); };
  host.querySelector("#ig-go")?.addEventListener("click", () => { commit(ion); go(); });
  ion.addEventListener("keydown", e => {
    if (e.key === "Enter") { e.preventDefault(); const first = sug.querySelector("li button"); if (!sug.hidden && first) /** @type {HTMLElement} */ (first).click(); else { commit(ion); go(); } }
    else if (e.key === "Escape" && !sug.hidden) { e.stopPropagation(); sug.hidden = true; ion.setAttribute("aria-expanded", "false"); }
  });
  let timer = 0;
  ion.addEventListener("input", () => { clearTimeout(timer); timer = window.setTimeout(() => suggest(ion, sug, add, smi, go), 250); });
}

/** The typed text becomes the ion (and forgets a molecule chosen by name when the text no longer is its name). */
function commit(/** @type {HTMLInputElement} */ ion) {
  const t = ion.value.trim();
  if (G.mol && t === G.mol.nome) return;
  G.mol = null; G.text = t;
}

async function suggest(/** @type {HTMLInputElement} */ ion, /** @type {HTMLElement} */ sug, /** @type {HTMLSelectElement} */ add, /** @type {HTMLInputElement} */ smi, /** @type {() => void} */ go) {
  const rows = await searchNames(ion.value);
  sug.hidden = !rows.length; ion.setAttribute("aria-expanded", String(!!rows.length));
  sug.innerHTML = rows.map(r => h`<li role="option"><button type="button" class="ig-sugb" data-id="${r.id}">${r.nome} · ${r.formula}</button></li>`).join("");
  sug.querySelectorAll("button").forEach(b => b.addEventListener("click", async () => {
    const row = rows.find(r => r.id === /** @type {HTMLElement} */ (b).dataset.id); if (!row) return;
    const m = await fullRecord(row);
    G.mol = m; G.text = ""; G.smiles = m.smiles || ""; G.adduct = startAdduct(polarityOf(E.files.find(INF.isInf)), m);
    ion.value = m.nome; add.value = G.adduct; smi.value = G.smiles; sug.hidden = true; ion.setAttribute("aria-expanded", "false");
    go();
  }));
}
