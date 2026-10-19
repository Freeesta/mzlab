// Study tool for docs/studi/STRUTTURA_DATO.md: does OpenChemLib (the copy in mzlab/web/vendor) keep the heavy atoms of a SMILES in the order written,
// as TPMOL (TP_Mine/js/46_tp_mol.js) assumes, and what its own canonical forms (idcode, SMILES) give for the same molecule written two ways.
// Usage: node tools/struttura_dato/ocl_ordine.mjs
import { pathToFileURL } from "node:url";
import path from "node:path";

const here = path.dirname(new URL(import.meta.url).pathname);
const O = await import(pathToFileURL(path.join(here, "../../mzlab/web/vendor/openchemlib.js")).href);
const M = O.Molecule || O.default.Molecule;

const cases = [
  ["timolol_S", "CC(C)(C)NC[C@H](O)COc1nsnc1N1CCOCC1"],
  ["timolol_S_other_way", "O[C@@H](CNC(C)(C)C)COc1nsnc1N1CCOCC1"],
  ["explicit_H", "[H]OC1=CC=CC=C1"],
  ["deuterated", "[2H]C([2H])([2H])Oc1ccccc1"],
  ["salt", "CC(C)(C)NC[C@H](O)COc1nsnc1N1CCOCC1.OC(=O)/C=C\\C(=O)O"],
  ["ring_closure_first", "C1CC(O)CCN1c1ccccc1"],
];

/** the heavy-atom symbols in SMILES order, from a minimal tokeniser (organic subset and brackets) */
function written(smi) {
  const out = [];
  const re = /\[([0-9]*)([A-Za-z][a-z]?)[^\]]*\]|Br|Cl|[BCNOPSFI]|[bcnops]/g;
  for (const m of smi.matchAll(re)) {
    const el = m[2] || m[0];
    const sym = el[0].toUpperCase() + el.slice(1);
    if (sym !== "H" || (m[1] && m[1] !== "")) out.push(m[1] === "2" && sym === "H" ? "D" : sym);
  }
  return out.filter(s => s !== "H");
}

for (const [name, smi] of cases) {
  const m = M.fromSmiles(smi);
  const labels = [];
  for (let i = 0; i < m.getAllAtoms(); i++) labels.push(m.getAtomMass(i) === 2 && m.getAtomLabel(i) === "H" ? "D" : m.getAtomLabel(i));
  const w = written(smi);
  const same = w.every((s, i) => labels[i] === s);
  console.log(`${name}\tallAtoms=${m.getAllAtoms()} written=${w.length}\torder_kept=${same}\tidcode=${m.getIDCode()}\tsmiles=${m.toIsomericSmiles()}`);
}
