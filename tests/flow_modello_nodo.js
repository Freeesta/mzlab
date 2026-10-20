// Model of the Flow page (flow/lazy/flow-*.js without the DOM): wiring, parameters, canonical export, import, goal. Exit code 1 and a message on the first failure.
const fs = require("fs"), vm = require("vm"), path = require("path"), assert = require("assert");
// the arrays come from another realm (vm): compare as JSON
const eq = (a, b) => assert.strictEqual(JSON.stringify(a), JSON.stringify(b));
const root = path.resolve(__dirname, "..");
const ctx = vm.createContext({});
for (const f of ["registry", "check", "plan", "inputs", "names", "fix", "results", "views-attributio", "views-tabui"]) vm.runInContext(fs.readFileSync(path.join(root, "flow/lazy/flow-" + f + ".js"), "utf8"), ctx);
const F = ctx.FLOW;
const reg = F.registry(["steps-1", "steps-attributio-1", "steps-tabui-1"].map(n => JSON.parse(fs.readFileSync(path.join(root, "docs/schemi", n + ".json"), "utf8"))));
const problems = plan => F.check(F.normalize(F.exportPlan(plan)), reg, "native").map(F.problemLine);

// a first step brings its own input; the chain wires itself and is valid
let plan = F.newPlan();
for (const s of ["convert.mzml", "index.mzidx", "features.detect", "features.align"]) F.addStep(plan, reg, s);
eq(plan.inputs.map(f => f.type), ["raw"]);
eq(plan.steps.map(s => s.id), ["convert", "index", "detect", "align"]);
eq(problems(plan), []);

// only the differences from the default are written; a value equal to the default disappears
const detect = F.instanceOf(plan, "detect"), step = reg.byId.get("features.detect");
F.setParam(detect, step, "floor", 5000);
eq(detect.params, { floor: 5000 });
F.setParam(detect, step, "floor", 20000);
assert.strictEqual(detect.params, undefined);
// a planned algorithm is a problem; changing the algorithm drops the parameters it does not declare
F.setAlgorithm(detect, step, "centwave"); F.setParam(detect, step, "snr", 7);
assert.ok(problems(plan).some(l => l.includes("err.flow.algorithm.unavailable")));
F.setAlgorithm(detect, step, "masstrace");
assert.strictEqual(detect.params, undefined);

// reordering the list does not change the exported order (the order of the run), and the export reads back to the same plan
const before = F.exportText(plan);
F.moveStep(plan, "align", -3);
assert.strictEqual(F.exportText(plan), before);
const again = F.importText(before);
assert.ok(again.plan);
assert.strictEqual(F.exportText(again.plan), before);
eq(F.importText("{").error.key, "err.flow.json");
eq(F.importText('{"format":"mzflow/9"}').error.key, "err.flow.format");

// removing a step removes the references to it; removing an input too
F.removeStep(plan, "index");
assert.ok(problems(plan).some(l => l.startsWith("detect|index|err.flow.port.missing")));
F.removeInput(plan, "input1.raw");
assert.ok(problems(plan).some(l => l.startsWith("convert|raw|err.flow.port.missing")));

// a port that takes several sources keeps a list; a candidate is offered only when it fits
plan = F.newPlan(); F.addStep(plan, reg, "convert.mzml"); F.addStep(plan, reg, "index.mzidx");
const port = F.ports(reg.byId.get("index.mzidx").in)[0];
const c = F.candidates(plan, reg, "index", port);
eq(c.map(x => [x.ref, x.fit]), [["file:input1.raw", "ok"], ["convert.mzml", "ok"]]);
assert.ok(F.candidates(plan, reg, "convert", F.ports(reg.byId.get("convert.mzml").in)[0]).every(x => x.ref !== "index.mzidx"));

// the goal: from raw files alone the identification cannot be reached; with spectra and a library it can
plan = F.newPlan(); F.addStep(plan, reg, "convert.mzml");
let g = F.goal(plan, reg);
eq([g.reached, g.canReach, g.missing], [false, false, ["spectra", "library"]]);
F.addInput(plan, "spectra"); F.addInput(plan, "library");
g = F.goal(plan, reg);
eq([g.reached, g.canReach], [false, true]);
for (const s of ["index.mzidx", "features.detect", "features.align", "formula.ms1", "spectra.match", "annot.level"]) F.addStep(plan, reg, s);
assert.strictEqual(F.goal(plan, reg).reached, true);
// the canvas: the base alone is complete and valid; every template is valid and shows as nodes, in Simple mode the base folds into two
eq(problems(F.basePlan(reg)), []);
eq(F.basePlan(reg).steps.map(s => s.id), ["index", "detect", "align", "gapfill"]);
for (const t of F.TEMPLATES) {
  const p = F.templatePlan(reg, t);
  eq(problems(p), []);
  assert.ok(F.canvasModel({ plan: p, reg }, true).nodes.length < F.canvasModel({ plan: p, reg }, false).nodes.length || p.steps.length === 4, t.id);
}
// the CSV of a result: quotes, doubled quotes, CRLF, a tab-separated file, a missing last newline
eq(F.parseCsv('a,b\r\n"x,1","say ""hi"""\r\n3,4'), [["a", "b"], ["x,1", 'say "hi"'], ["3", "4"]]);
eq(F.parseCsv("a\tb\n1\t2\n"), [["a", "b"], ["1", "2"]]);
eq(F.tableSummary([["id", "mz", "name"], ["1", "100.5", "a"], ["2", "", ""], ["3", "99", "b"]]),
  [{ name: "id", filled: 3, empty: 0, min: 1, max: 3 }, { name: "mz", filled: 2, empty: 1, min: 99, max: 100.5 }, { name: "name", filled: 2, empty: 1 }]);
// every add-on names steps of the registry, and the Attributio steps are all reachable from «Add analysis»
for (const a of F.ADDONS) for (const id of a.steps) assert.ok(F.NAMES[id], a.id + " " + id);
for (const id of ["spectra.chimeric", "formula.ms1", "formula.ms2", "spectra.match", "spectra.insilico", "rt.predict", "annot.level", "annot.regulatory", "annot.hazard"]) assert.ok(F.ADDONS.some(a => a.steps.includes(id)), id);
assert.ok(F.viewFor("table") && F.viewFor("annot") && !F.viewFor("raw"));
// the identification views: JSON shapes become tables, the MGF reader keeps title, precursor, RT and peaks; a file is routed to the view that accepts it
eq(F.readCandidates({ hits: [{ query: "q1", name: "A", formula: "C2H6O", score: 0.91234, shared: 3, mass_ppm: 1.234, source: "lib", algorithm: "cosine" }] }).rows, [["q1", "A", "C2H6O", "0.9123", "3", "1.23", "lib", "cosine"]]);
eq(F.readAnnot({ rows: [{ id: "q1", level: "2a", why: "2a", name: "A", gates: [{ id: "score", ok: false, value: 0.5, threshold: 0.7 }] }] }).rows, [["q1", "2a", "2a", "A", "", "score 0.5 / 0.7 (not met)"]]);
eq(F.readFormulas({ features: [{ id: "f1", mz: 100, rt: 2, file: "a", scan: 5, result: { chosen: "C5H8", candidates: [{ formula: "C6H10", error_ppm: 1 }, { formula: "C5H8", error_ppm: 0.5, score: 0.8, confidence: "high" }] } }, { id: "f2", mz: 1, rt: 1, error: "err.x" }] }).rows.map(r => [r[0], r[5], r[9]]), [["f1", "C5H8", ""], ["f2", "", "err.x"]]);
const fs2 = F.readFormulas({ spectra: [{ query: "s1", precursor: 150.1, peaks: 4, candidates: [{ formula: "C9H10", adduct: "[M+H]+", ppm_ion: 1, n_explained: 1, frac_explained: 0.5, score: 0.4, fragments: [{ mz: 91.05, intensity: 10, formula: "C7H7", mz_th: 91.054, ppm: 2, loss: "C2H3" }] }] }] });
eq([fs2.rows.length, fs2.fragments.rows.length], [1, 1]);
const mgf = F.readMgf("BEGIN IONS\nTITLE=s1\nPEPMASS=150.1 1000\nCHARGE=1+\nRTINSECONDS=12.5\n91.05 10\n120 5.5\nEND IONS\nBEGIN IONS\nTITLE=s2\nEND IONS\n");
eq([mgf.length, mgf[0].title, mgf[0].pepmass, mgf[0].rt, mgf[0].peaks], [2, "s1", 150.1, 12.5, [[91.05, 10], [120, 5.5]]]);
eq([F.viewFor("candidates", { label: "x.json" }).id, F.viewFor("spectra", { label: "x.mgf" }).id, F.viewFor("annot", { label: "x.csv" }).id], ["candidates", "spectra", "table"]);
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
