// @ts-check
// MOL / SDF export of Draw: the molfile of the canvas goes to the Rust writer (`sdf` of the molkit .wasm, `sdf/1`; the app asks `feat/grafo`,
// kind "sdf"; `mzlab-cli mol-export` gives the same bytes). A .mol keeps the whole canvas in one record; an .sdf has one record per structure
// with formula, exact mass, the m/z of the chosen ion and the mol1: identifier. This file only builds the request and returns the text.
import { molfileOf } from "./disegno-molfile.js";
import { carica } from "./disegno-verifica.js";

const t = (/** @type {string} */ k, /** @type {Record<string, any>} */ p = {}) => I18N.t(k, p);

/** the message of an engine error (`{error_key, params}` text) or of anything else */
function message(/** @type {any} */ e) {
  try { const j = JSON.parse(String(e)); if (j.error_key) return t(j.error_key, j.params || {}); } catch (_) { /* not JSON */ }
  return e && e.message ? e.message : String(e);
}

/** `sdf/1` text of a request: the native engine in the app, the molkit .wasm on the site */
async function ask(/** @type {string} */ request) {
  if (/** @type {any} */ (window).MZLAB_DESKTOP) {
    const base = /Windows/.test(navigator.userAgent) ? "http://mzlab.localhost/" : "mzlab://localhost/";
    const r = await fetch(base + "rpc", { method: "POST", body: JSON.stringify({ op: "feat/grafo", kind: "sdf", request }) });
    const x = await r.text();
    if (r.status !== 200) throw x;
    return x;
  }
  return (await carica()).sdf(request);
}

/**
 * The text of the .mol or .sdf file of the whole canvas; an engine error comes back as an Error with its translated message.
 * @param {any} K Ketcher @param {"mol" | "sdf"} format @param {string} ion "none" or an adduct of the engine table (the options of #ex-ion) @param {string} name the title line of the records
 */
export async function exportText(K, format, ion, name) {
  const st = K.editor.struct(), mf = molfileOf(st, [...st.atoms.keys()], undefined, () => true);
  try { return await ask(JSON.stringify({ molfile: mf, format, ion, name })); }
  catch (e) { throw new Error(message(e)); }
}
