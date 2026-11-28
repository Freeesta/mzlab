// Prints the pair/1 text the browser makes for features.pair: flow_pair of the flowtab .wasm over the two tables (what flow-web-pair.js does; wasm-bindgen --target nodejs);
// see tests/test_flowtab_pair_wasm.py. usage: node flowtab_pair_node.js <flowtab pkg dir> <params json> <pos.mzfeat> <neg.mzfeat>
const fs = require("fs");
const path = require("path");
const tab = require(path.join(process.argv[2], "mzlab_feat_wasm.js"));
const [pos, neg] = process.argv.slice(4);
const doc = tab.flow_pair(process.argv[3], path.basename(pos), new Uint8Array(fs.readFileSync(pos)), path.basename(neg), new Uint8Array(fs.readFileSync(neg)));
process.stdout.write(JSON.stringify({ csv: JSON.parse(doc).csv, doc }));
