// Prints the target list and the union/1 text the browser makes for features.union: flow_series_csv of the flowtab .wasm for each table, then union_tables of the rotte .wasm
// (what flow-web-union.js does; wasm-bindgen --target nodejs); see tests/test_flowtab_union_wasm.py.
// usage: node flowtab_union_node.js <flowtab pkg dir> <rotte pkg dir> <params json> <table.mzfeat>...
const fs = require("fs");
const path = require("path");
const tab = require(path.join(process.argv[2], "mzlab_feat_wasm.js"));
const rotte = require(path.join(process.argv[3], "mzlab_rotte_wasm.js"));
const files = process.argv.slice(5);
const tables = files.map(f => ({ name: path.basename(f), text: tab.flow_series_csv(new Uint8Array(fs.readFileSync(f))) }));
const doc = rotte.union_tables(JSON.stringify({ tables, params: JSON.parse(process.argv[4]) }));
process.stdout.write(JSON.stringify({ csv: JSON.parse(doc).csv, doc }));
