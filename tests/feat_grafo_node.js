// Runs the structure bridge of mzFinder (crates/mzlab-feat-grafo-wasm, wasm-bindgen --target nodejs) and prints the JSON of `analyse(smiles)`
// (see tests/test_feat_grafo_wasm.py).  usage: node feat_grafo_node.js <pkg dir> <smiles>; with `labels`, `cut` or `mech` as the third argument, `labels(request)`, `cut(request)` or `mech(request)`
const path = require("path");
const [pkg, smiles, a, b] = process.argv.slice(2);   // with a and b: `smiles` is a molfile and the output is `fragments(molfile, a, b)`
const w = require(path.join(pkg, "mzlab_feat_grafo_wasm.js"));
try {
  if (a === "labels" || a === "cut" || a === "mech") process.stdout.write(w[a](smiles) + "\n");
  else console.log(a === undefined ? w.analyse(smiles) : w.fragments(smiles, Number(a), Number(b)));
} catch (e) {
  console.log(typeof e === "string" ? e : String(e));
}
