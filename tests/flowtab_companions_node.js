// Prints the companions/1 text and the warnings of flow_companions of the flowtab .wasm (wasm-bindgen --target nodejs); see tests/test_flowtab_companions_wasm.py.
// usage: node flowtab_companions_node.js <pkg dir> <params json> <table.mzfeat>
const fs = require("fs");
const path = require("path");
const w = require(path.join(process.argv[2], "mzlab_feat_wasm.js"));
const r = w.flow_companions(process.argv[3], fs.readFileSync(process.argv[4]));
process.stdout.write(JSON.stringify({ text: r.take_text(), warnings: JSON.parse(r.warnings()) }));
