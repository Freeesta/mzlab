// @ts-strict
// Molecule drawings of the «Fragmentation graph»: the 2D molfile of the SMILES of the tab (OpenChemLib, loaded on first use), sent back to the graph as the option
// `molfile`, and the SVG of the molecule or of the parent of a fragment with the atoms of the cut piece highlighted (vermillion fill and dashed ring: the
// colour is never the only cue). A fragment drawing is a hypothesis of the cleavage candidates, never a result. Used by infusione-grafo*.js.
import { G } from "./infusione-grafo-stato.js";
import { ocl } from "./mol-svg.js";

/** @type {any} */ let OCL = null;
let doneFor = "";
/** @type {Map<string, string>} */
const pics = new Map();

/** Makes `G.molfile` from `G.smiles` (empty when there is no SMILES, it cannot be read or OpenChemLib is missing). Call before asking for the graph. */
export async function prepareMolfile() {
  if (doneFor === G.smiles && (G.molfile || !G.smiles)) return;
  G.molfile = ""; pics.clear(); doneFor = G.smiles;
  if (!G.smiles) return;
  const o = OCL = await ocl();
  if (!o) return;
  try { const m = o.Molecule.fromSmiles(G.smiles); m.inventCoordinates(); G.molfile = m.toMolfile(); } catch (_) { G.molfile = ""; }
}

/** The SVG text of the molecule (the molfile of the graph) with the atoms `atoms` highlighted (0-based as in the SMILES). Null without OpenChemLib or molfile. */
export function molSvg(/** @type {number[]} */ atoms, /** @type {number} */ w, /** @type {number} */ h, /** @type {string} */ id) {
  const key = `${id}|${w}|${h}|${atoms.join(",")}`;
  if (pics.has(key)) return /** @type {string} */ (pics.get(key));
  if (!OCL || !G.molfile) return null;
  try {
    const mol = OCL.Molecule.fromMolfile(G.molfile);
    let svg = mol.toSVG(w, h, id, window.MOLUI ? MOLUI.SVG_OPTS : { suppressChiralText: true, suppressCIPParity: true, suppressESR: true });
    if (window.MOLUI) svg = MOLUI.svg(svg);
    if (atoms.length) {
      const rings = atoms.map(i => {
        const m = new RegExp(`<circle id="${id}:Atom:${i}"[^>]*?cx="([\\d.\\-]+)" cy="([\\d.\\-]+)"`).exec(svg);
        return m ? `<circle cx="${m[1]}" cy="${m[2]}" r="12" class="ig-hl" data-atom="${i}"/>` : "";
      }).join("");
      svg = svg.replace(/<\/style>/, "</style>" + rings);
    }
    pics.set(key, svg);
    return svg;
  } catch (_) { return null; }
}

/** The atoms of the piece of the first cut of the first candidate of the cleavages of a node (`strutture` of the graph), or null. */
export const pieceOf = (/** @type {any} */ s) => (s && s.candidati && s.candidati[0] && s.candidati[0].tagli[0] ? /** @type {number[]} */ (s.candidati[0].tagli[0].pezzo) : null);

/** The drawing of the root and of every ion with cleavages for the graph in G.j: `{ root, byId }` with SVG texts (empty without a molfile or with the structures off). */
export function drawingsOf() {
  /** @type {Map<string, string>} */
  const byId = new Map();
  if (!G.structs || !G.j || !G.molfile || !OCL) return { root: null, byId };
  for (const s of G.j.strutture_tutti || []) {
    const atoms = pieceOf(s);
    const svg = atoms && molSvg(atoms, 150, 80, "igs-" + s.nodo);
    if (svg) byId.set(s.nodo, svg);
  }
  return { root: molSvg([], 150, 80, "igs-root"), byId };
}
