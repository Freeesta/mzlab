// The Tabui result views of Flow (flow/lazy/flow-views-tabui.js without the DOM): the readers of tptab/1, gruppi/1, interp/1 and reatt/1. Exit code 1 on the first failure.
const fs = require("fs"), vm = require("vm"), path = require("path"), assert = require("assert");
const eq = (a, b) => assert.strictEqual(JSON.stringify(a), JSON.stringify(b));
const root = path.resolve(__dirname, "..");
const ctx = vm.createContext({});
for (const f of ["registry", "results", "views-attributio", "views-tabui"]) vm.runInContext(fs.readFileSync(path.join(root, "flow/lazy/flow-" + f + ".js"), "utf8"), ctx);
const F = ctx.FLOW;
// the Tabui views: the product table words the status bytes, the conditions give a matrix per product, the reading of a row lists what it found
const tp = F.readTptab({ rows: [{ row: 3, id: 7, mz: 200.1, rt: 4.5, status: 0, kinship: 3, via: 9, n_det: 4, max_area: 1500.4, tmax: 30, in_t0: false, in_blank: true, blank_ratio: 0.25 }] });
eq(tp.rows[0], ["3", "7", "200.1", "4.5", "Product", "Probable", "rule, MS2", "4", "1500", "30", "no", "yes (×0.25)"]);
const gr = F.readGruppi({ conditions: [{ id: "O2", parent: { k: 0.05, half_life: 13.9, remaining: 0.2 } }, { id: "N2", parent: {} }], rows: [{ tp_name: "TP 7", mz: 200.1, rt: 4.5, neutral_formula: "C8H9", cond: [{ present: true, area: 900, ratio_ref: 1 }, { present: false }] }], sets: [{ conditions: ["O2"], rows: [1] }] });
eq([gr.matrix.head.slice(4), gr.matrix.rows[0].slice(4), gr.sets.rows, gr.parent.rows[0]], [["O2", "N2"], ["present, area 900", "absent"], [["O2", "1"]], ["O2", "0.05", "13.9", "0.2"]]);
const ip = F.readInterp({ id: 5, change: { delta: "+O", ion_formula: "C8H10O", ppm: 1.234 }, ms2: { scans: [{ chimeric: true }, {}], files_with_ms2: 1 }, vs_parent: { cos_mod: 0.81234 }, site: { transform: "+O", classes: [{ id: 1, kind: "ring", status: "open", support: 0.6 }] } });
eq(ip.summary.rows.filter(r => ["Change", "MS2 scans", "Mixed spectra left out", "Modified cosine to the parent"].includes(r[0])), [["Change", "+O"], ["MS2 scans", "2"], ["Mixed spectra left out", "1"], ["Modified cosine to the parent", "0.812"]]);
eq(ip.classes.rows, [["1", "ring", "open", "", "0.6", "", ""]]);
eq(F.readReatt({ atoms: [{ i: 0, el: 6, heavy_index: 0, q: -0.1234, f_minus: 0.3333 }] }).rows[0], ["0", "6", "0", "-0.123", "", "", "0.333", ""]);
eq([F.viewFor("tp", { label: "x.json" }).id, F.viewFor("tp", { label: "x.csv" }).id], ["tp", "table"]);
console.log("ok");
