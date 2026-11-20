// @ts-check
// «Draw» of Draw for a SMILES: the molfile layout comes from the Rust engine (`draw` of the molkit .wasm, `draw/1`; the app asks `feat/grafo`, kind "draw";
// `mzlab-cli mol draw --format molfile --set layout=new` gives the same bytes), not from Ketcher's Indigo. Ketcher only ADDS the molfile it is given.
// No result is ever blocked: when the engine is not there or will not draw the text (an InChI, a reaction SMILES, an invalid SMILES), `{}` comes back and
// the caller adds the text itself, as Ketcher always did; Ketcher's own notice then says if it drew nothing.
import { loadWasm, rpc } from "./disegno-verifica.js";

/** The molfile the engine lays out for a SMILES: `{ molfile }`, or `{}` when the text is to go to Ketcher as it is.
 * @param {string} text @returns {Promise<{ molfile?: string }>} */
export async function layoutSmiles(text) {
  if (/^InChI=/i.test(text.trim())) return {};   // the engine does not read InChI (err.mol.draw.inchi); Ketcher does
  try {
    const request = JSON.stringify({ spec: "smiles:" + text, format: "molfile", layout: "new" });
    const molfile = /** @type {any} */ (window).MZLAB_DESKTOP ? await rpc("draw", request) : (await loadWasm()).draw(request);
    if (typeof molfile === "string" && molfile.includes("M  END")) return { molfile };
    console.warn("draw/1 gave no molfile, the text goes to Ketcher as it is");
  } catch (e) { console.warn("draw/1 did not lay the text out, it goes to Ketcher as it is:", e); }   // not there, or a refusal: the same way out
  return {};
}
