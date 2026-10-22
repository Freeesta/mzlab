// @ts-check
// The wait of the «Fragmentation graph» tab: the bar over the phases of api/msn_grafo (events «mzlab-msngrafo» of rust-msngrafo-pagina.js) and the watch
// that turns a request with no news for STALL_MS into an error with «Copy diagnosis» (never silence). Used by infusione-grafo.js. docs/agenti/lettura-nativa.md.

import { TXT } from "./infusione-grafo-stato.js";

export const STALL_MS = 30000;

/**
 * Shows the bar in `host` and calls `onStall(seconds, phase)` once if nothing arrives for `stallMs`; the returned function ends the watch.
 * @param {HTMLElement} host @param {(s: number, phase: string) => void} onStall @param {number} [stallMs]
 */
export function guarda(host, onStall, stallMs = STALL_MS) {
  let last = performance.now(), phase = "load", frac = 0, done = false;
  const show = () => {
    const pct = Math.round(frac * 100);
    host.innerHTML = `<p class="sm ig-wait"><label>${I18N.t(/** @type {Record<string, string>} */ (TXT.prog)[phase], { pct })} <progress max="100" value="${pct}"></progress></label></p>`;
  };
  /** @param {Event} ev */
  const on = ev => {
    const d = /** @type {CustomEvent} */ (ev).detail || {};
    last = performance.now(); phase = ["load", "convert", "index", "graph"].includes(d.phase) ? d.phase : phase; frac = Math.max(frac, +d.frac || 0);
    show();
  };
  window.addEventListener("mzlab-msngrafo", on);
  const timer = setInterval(() => {
    const s = (performance.now() - last) / 1000;
    if (!done && s * 1000 >= stallMs) { end(); onStall(Math.round(s), phase); }
  }, 1000);
  function end() { done = true; clearInterval(timer); window.removeEventListener("mzlab-msngrafo", on); }
  show();
  return () => { end(); };
}

/** The phases of the graph as the phases of a diagnosis (diagnosi.js FASI). @type {Record<string, string>} */
const FASE = { load: "open", convert: "decode", index: "index", graph: "decode" };

/**
 * An error of the tab with its «Details» and «Copy diagnosis» (diagnosi.js); `e` is an Error of I18N.err ({key, params}) or a plain `{key, params}`.
 * @param {HTMLElement} host @param {string} text @param {any} e @param {{ name?: string, size?: number }} file @param {string} [phase] the phase the graph was in
 */
export function mostraErrore(host, text, e, file, phase = "graph") {
  const err = { key: e && e.key, message: text, params: { ...(e && e.params), fase: FASE[phase] || "decode", raw: (e && e.params && e.params.raw) || text } };
  const d = DIAG.da(err, { name: file.name, size: file.size, format: "raw" });
  const t = (/** @type {string} */ k) => I18N.t(k);
  host.innerHTML = `<p class="fail sm" role="alert">${EH(text)}</p>` + DIAG.html(d, EH, t);
  DIAG.cabla(host, d, window.MZLAB_VERSION, t);
}
