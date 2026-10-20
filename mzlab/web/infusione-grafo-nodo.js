// @ts-check
// The panel of the selected ion of the «Fragmentation graph»: its candidate formulas (error, evidences, «Use»), the isolation window, the peaks of its spectra
// the graph could not explain, the warnings of the node and a drawing box (the fragment the user draws, kept next to the node: a hypothesis, not a result).
import { G, TXT, ionText, useCandidate } from "./infusione-grafo-stato.js";

const ST = "mzlab.infg.frag";
/** @returns {Record<string, { smiles: string, svg: string }>} */
const frags = () => { try { return JSON.parse(localStorage.getItem(ST) || "{}"); } catch (_) { return {}; } };
const fragKey = (/** @type {any} */ n) => `${(G.j.sorgente && G.j.sorgente.sha256_idx) || ""}|${ionText()}|${n.mz.toFixed(3)}`;

/** The saved drawing of a node, or null. */
export const savedFrag = (/** @type {any} */ n) => frags()[fragKey(n)] || null;

function saveFrag(/** @type {any} */ n, /** @type {{ smiles: string, svg: string } | null} */ v) {
  const all = frags();
  if (v) all[fragKey(n)] = v; else delete all[fragKey(n)];
  try { localStorage.setItem(ST, JSON.stringify(all)); } catch (_) { /* storage full or blocked: the drawing stays in the editor */ }
}

const ppmTxt = (/** @type {number} */ v) => `${v > 0 ? "+" : v < 0 ? "−" : ""}${Math.abs(v).toFixed(2)} ppm`;

/** The table of the candidates of a node; a «Use» button on each when there is more than one. */
function candidates(/** @type {any} */ n) {
  const f = n.formula;
  if (!f) return h`<p class="muted sm">${I18N.t("msngrafo.nof")}</p>`;
  const rows = f.candidati.map((/** @type {any} */ c) => {
    const on = f.scelta === c.formula, prove = c.prove.map((/** @type {string} */ p) => I18N.t(/** @type {any} */ (TXT.prove)[p])).join(", ");
    const btn = f.candidati.length > 1 && !on ? h`<button type="button" class="bt ig-use" data-f="${c.formula}" title="${I18N.t("msngrafo.use.tip", { f: c.formula })}">${I18N.t("msngrafo.use")}</button>` : on ? I18N.t(f.fonte === "utente" ? "msngrafo.used" : "msngrafo.chosen") : "";
    return h`<tr${raw(on ? ' class="on"' : "")}><th scope="row">${c.formula}</th><td class="num">${c.mz_teorico.toFixed(5)}</td><td class="num">${ppmTxt(c.ppm)}</td><td class="num">${c.dbe}</td><td>${I18N.t(/** @type {any} */ (TXT.elettroni)[c.elettroni])}</td><td class="num">${c.finestra}</td><td>${prove}</td><td>${raw(btn)}</td></tr>`;
  });
  const head = ["msngrafo.col.cand", "msngrafo.col.mz", "msngrafo.col.err", "DBE", "msngrafo.col.e", "msngrafo.col.win", "msngrafo.col.prove", ""].map(k => raw(h`<th scope="col">${k.startsWith("msngrafo") ? I18N.t(k) : k}</th>`));
  return h`<table class="ig-cands"><caption class="sr-only">${I18N.t("msngrafo.cands.cap")}</caption><thead><tr>${head}</tr></thead><tbody>${rows.map((/** @type {string} */ r) => raw(r))}</tbody></table>`;
}

function isolation(/** @type {any} */ n) {
  const i = n.isolamento; if (!i) return "";
  const w = i.larghezza != null ? I18N.t("msngrafo.iso.win", { w: i.larghezza }) : i.a_monte != null ? I18N.t("msngrafo.iso.up", { w: i.a_monte }) : "";
  return h`<p class="sm">${w}${raw(i.stretto ? ` <span class="muted">${EH(I18N.t("msngrafo.iso.narrow"))}</span>` : "")}</p>`;
}

function unexplained(/** @type {any} */ n) {
  const u = n.non_spiegati; if (!u) return "";
  const top = u.picchi.map((/** @type {any} */ p) => `${p.mz.toFixed(4)} (${p.rel}%)`).join(", ");
  return h`<p class="sm ig-unexp">${I18N.t("msngrafo.unexp", { n: u.n })} ${top}</p>`;
}

const warnings = (/** @type {any} */ n) => G.j.avvisi.filter((/** @type {any} */ a) => a.nodo === n.id).map((/** @type {any} */ a) => h`<p class="sm ig-warn">${I18N.t(/** @type {any} */ (TXT.warn)[a.chiave], { node: n.mz.toFixed(4) })}</p>`).join("");

/** The drawing box: the saved fragment (picture from MOLUI) with Draw / Save drawing / Remove. */
function drawBox(/** @type {any} */ n) {
  const v = savedFrag(n);
  const pic = v ? `<div class="ig-pic" role="img" aria-label="${EH(I18N.t("msngrafo.draw.aria", { smi: v.smiles }))}">${window.MOLUI ? MOLUI.svg(v.svg) : ""}</div><code class="sm">${EH(v.smiles)}</code>` : `<p class="muted sm">${EH(I18N.t("msngrafo.draw.none"))}</p>`;
  return `<div class="ig-draw"><b class="sm">${EH(I18N.t("msngrafo.draw.title"))}</b>${pic}<div class="row"><button type="button" class="bt" data-a="draw" title="${EH(I18N.t("msngrafo.draw.go.tip"))}">${EH(I18N.t("msngrafo.draw.go"))}</button><button type="button" class="bt" data-a="save" title="${EH(I18N.t("msngrafo.draw.save.tip"))}">${EH(I18N.t("msngrafo.draw.save"))}</button>${v ? `<button type="button" class="bt" data-a="del">${EH(I18N.t("msngrafo.draw.del"))}</button>` : ""}</div><p class="muted sm">${EH(I18N.t("msngrafo.draw.hyp"))}</p></div>`;
}

async function openDraw(/** @type {any} */ n) {
  const v = savedFrag(n), smi = (v && v.smiles) || (n.livello === 1 ? G.smiles : "");
  if (typeof setView === "function") setView("draw");
  try { if (window.TPDraw && smi) await TPDraw.load(smi); } catch (e) { console.info("INFG draw", e); }
}

async function saveDrawing(/** @type {any} */ n) {
  if (!window.TPDraw || !TPDraw.ready()) return nearMsg(I18N.t("msngrafo.draw.notReady"));
  try {
    const smiles = await TPDraw.smiles(), blob = await TPDraw.image("svg");
    if (!smiles) return nearMsg(I18N.t("msngrafo.draw.empty"));
    saveFrag(n, { smiles, svg: await blob.text() });
  } catch (e) { console.info("INFG save", e); nearMsg(I18N.t("msngrafo.draw.notReady")); }
}

/** Fills the panel of a node. `redo` asks for the graph again (a candidate was chosen), `again` draws the panel again. */
export function nodePanel(/** @type {HTMLElement} */ host, /** @type {any} */ n, /** @type {{ redo: () => void, again: () => void }} */ cb) {
  if (!n) { host.innerHTML = h`<p class="muted sm">${I18N.t("msngrafo.pick")}</p>`; return; }
  host.innerHTML = h`<h4>${I18N.t("msngrafo.node.title", { level: n.livello, mz: n.mz.toFixed(4) })}</h4>` + candidates(n) + isolation(n) + unexplained(n) + warnings(n) + drawBox(n);
  host.querySelectorAll(".ig-use").forEach(b => b.addEventListener("click", () => {
    useCandidate(n, /** @type {string} */ (/** @type {HTMLElement} */ (b).dataset.f)); cb.redo();
  }));
  const on = (/** @type {string} */ a, /** @type {() => any} */ fn) => { const b = host.querySelector(`[data-a="${a}"]`); if (b) b.addEventListener("click", fn); };
  on("draw", () => openDraw(n));
  on("save", async () => { await saveDrawing(n); cb.again(); });
  on("del", () => { saveFrag(n, null); cb.again(); });
}
