// Prints the target list and the union/1 text of flow_union of the flowtab .wasm (wasm-bindgen --target nodejs); see tests/test_flowtab_union_wasm.py.
// usage: node flowtab_union_node.js <pkg dir> <params json> <table.mzfeat>...
const fs = require("fs");
const path = require("path");
const w = require(path.join(process.argv[2], "mzlab_feat_wasm.js"));
const files = process.argv.slice(4), bytes = files.map(f => fs.readFileSync(f));
const r = w.flow_union(process.argv[3], JSON.stringify(files.map(f => path.basename(f))), JSON.stringify(bytes.map(b => b.length)), Buffer.concat(bytes));
process.stdout.write(JSON.stringify({ csv: r.take_csv(), doc: r.take_doc() }));
