// @ts-check
// «Search lab library» of the «Fragmentation graph»: the graph in view is compared with the local library of MSn trees (OPFS, see «Add to library»)
// through POST api/msnlib/cerca. Shows the candidates with their three separate scores and proofs, analogs marked; never a verdict. The engine ranks and scores.
import { params } from "./infusione-grafo-stato.js";
import { readLibrary } from "./infusione-grafo-esporta.js";

const num = (/** @type {any} */ v, /** @type {number} */ d = 2) => (v == null ? "–" : Number(v).toFixed(d));

/** The rows of the candidates (msncerca/1) as a table with the proofs next to the scores. */
function table(/** @type {any} */ j) {
  if (!j.candidati.length) return h`<p class="muted sm">${I18N.t("msngrafo.find.none")}</p>`;
  const rows = j.candidati.map((/** @type {any} */ c) => {
    const a = c.analogo;
    const kind = a ? I18N.t("msngrafo.find.analog", { d: num(a.delta_da, 4) }) : I18N.t("msngrafo.find.exact");
    const pr = I18N.t("msngrafo.find.proofs", { p: c.prove.picchi_comuni, n: c.prove.nodi_allineati, t: c.prove.nodi_ignoto, l: c.prove.perdite_comuni }) +
      (a ? " · " + I18N.t("msngrafo.find.shift", { s: a.spostati, u: a.non_spostati, c: num(a.copertura) }) : "");
    return h`<tr><th scope="row">${c.id}</th><td>${c.ione}</td><td>${kind}</td><td class="num">${num(c.delta_ppm)}</td><td class="num">${num(c.punteggi.ms2)}</td><td class="num">${num(c.punteggi.albero)}</td><td class="num">${num(c.punteggi.perdite)}</td><td>${pr}</td></tr>`;
  }).join("");
  return h`<table class="ig-cands ig-find-t"><caption>${I18N.t("msngrafo.find.cap")}</caption><thead><tr><th scope="col">${I18N.t("msngrafo.find.col.entry")}</th><th scope="col">${I18N.t("msngrafo.find.col.ion")}</th><th scope="col">${I18N.t("msngrafo.find.col.kind")}</th><th scope="col">Δ ppm</th><th scope="col">${I18N.t("msngrafo.find.col.ms2")}</th><th scope="col">${I18N.t("msngrafo.find.col.tree")}</th><th scope="col">${I18N.t("msngrafo.find.col.loss")}</th><th scope="col">${I18N.t("msngrafo.find.col.proofs")}</th></tr></thead><tbody>${raw(rows)}</tbody></table><p class="muted sm">${I18N.t("msngrafo.find.hyp")}</p>`;
}

/** Compares the graph with the library of the user and draws the candidates in `out`. */
async function search(/** @type {HTMLElement} */ out, /** @type {any} */ file, /** @type {number} */ delta) {
  try {
    const lib = await readLibrary();
    if (!lib) { out.innerHTML = h`<p class="muted sm" role="status">${I18N.t("msngrafo.lib.empty")}</p>`; return; }
    const body = { libreria: lib, ...params(file.k), delta_max: delta };
    const r = await fetch("api/msnlib/cerca", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body) });
    const j = await r.json();
    if (j.error) throw I18N.err(j);
    out.innerHTML = table(j);
  } catch (e) { out.innerHTML = h`<p class="fail sm" role="alert">${/** @type {any} */ (e).message || String(e)}</p>`; }
}

/** The row «Search lab library» (with the largest precursor difference for analogs, 0 = exact only) and the place of the result. */
export function searchRow(/** @type {HTMLElement} */ host, /** @type {any} */ file) {
  host.innerHTML = h`<div class="row ig-find-r"><label>${I18N.t("msngrafo.find.delta")} <input type="number" id="ig-delta" min="0" max="500" step="1" value="0" title="${I18N.t("msngrafo.find.delta.tip")}"></label><button type="button" class="bt" id="ig-find-go" title="${I18N.t("msngrafo.find.tip")}">${I18N.t("msngrafo.find")}</button></div><div id="ig-find-out" aria-live="polite"></div>`;
  const out = /** @type {HTMLElement} */ (host.querySelector("#ig-find-out"));
  host.querySelector("#ig-find-go")?.addEventListener("click", () => search(out, file, Math.max(0, Number(/** @type {HTMLInputElement} */ (host.querySelector("#ig-delta")).value) || 0)));
}
