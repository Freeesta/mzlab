// Writes the gold m/z of every row of sostanze_hrms.csv, computed by the current JS core (numbers only, row order).
// Used by tests/test_oro_molecole_hrms.py to prove the molecules-based screening gives the same m/z (<= 1e-5).
"use strict";
const fs = require("fs");
const path = require("path");
const root = path.join(__dirname, "..");
const core = require(path.join(root, "mzlab/web/liste-target-core.js"));

const text = fs.readFileSync(path.join(root, "mzlab/web/sostanze_hrms.csv"), "utf8");
const adducts = Object.keys(core.ADDUCT_DELTAS);
const rows = core.parseTargetCsv(text).map(e => [e.monoisotopic_mass, ...adducts.map(a => core.calcTargetMz(e, a))]);
const out = { adducts, columns: ["mass", ...adducts], rows };
fs.writeFileSync(path.join(root, "tests/dati/molecole_hrms_oro.json"), JSON.stringify(out) + "\n");
console.log(rows.length + " rows");
