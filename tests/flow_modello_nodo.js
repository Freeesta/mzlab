// Model of the Flow page (flow/lazy/flow-*.js without the DOM): wiring, parameters, canonical export, import, goal. Exit code 1 and a message on the first failure.
const fs = require("fs"), vm = require("vm"), path = require("path"), assert = require("assert");
// the arrays come from another realm (vm): compare as JSON
const eq = (a, b) => assert.strictEqual(JSON.stringify(a), JSON.stringify(b));
const root = path.resolve(__dirname, "..");
const ctx = vm.createContext({});
for (const f of ["registry", "check", "plan", "inputs", "names", "fix"]) vm.runInContext(fs.readFileSync(path.join(root, "flow/lazy/flow-" + f + ".js"), "utf8"), ctx);
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
console.log("ok");
