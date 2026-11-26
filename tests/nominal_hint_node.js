// Prints the parameters of the Pipeline panel's hint line (`PIPEFORM.nominalHint` of TP_Mine/js/32_pipeline_form.js) for the `nominal` member of an `estimates/1` text read from stdin.
// usage: node nominal_hint_node.js < estimates.json (see tests/test_estimate_nominal_wasm.py)
const fs = require("fs");
const path = require("path");
global.window = {};
require(path.join(__dirname, "..", "TP_Mine", "js", "32_pipeline_form.js"));
const e = JSON.parse(fs.readFileSync(0, "utf8"));
process.stdout.write(JSON.stringify(global.window.PIPEFORM.nominalHint(e.nominal)));
