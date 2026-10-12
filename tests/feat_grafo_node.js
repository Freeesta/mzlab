// Runs the structure bridge of mzFinder (crates/mzlab-feat-grafo-wasm, wasm-bindgen --target nodejs) and prints the JSON of `analyse(smiles)`
// (see tests/test_feat_grafo_wasm.py).  usage: node feat_grafo_node.js <pkg dir> <smiles>
const path = require("path");
const [pkg, smiles] = process.argv.slice(2);
const w = require(path.join(pkg, "mzlab_feat_grafo_wasm.js"));
try {
  console.log(w.analyse(smiles));
} catch (e) {
  console.log(typeof e === "string" ? e : String(e));
}
