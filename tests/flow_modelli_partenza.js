// Templates of the Flow start screen (flow/lazy/flow-names.js, F.templatePlan): each gives a valid plan; the new ones carry their sample list and their starting settings. Exit code 1 on the first failure.
const fs = require("fs"), vm = require("vm"), path = require("path"), assert = require("assert");
const eq = (a, b) => assert.strictEqual(JSON.stringify(a), JSON.stringify(b));
const root = path.resolve(__dirname, "..");
const ctx = vm.createContext({});
for (const f of ["registry", "check", "plan", "inputs", "names", "fix", "results", "views-attributio"]) vm.runInContext(fs.readFileSync(path.join(root, "flow/lazy/flow-" + f + ".js"), "utf8"), ctx);
const F = ctx.FLOW;
const reg = F.registry(["steps-1", "steps-attributio-1", "steps-tabui-1", "steps-cli-1"].map(n => JSON.parse(fs.readFileSync(path.join(root, "docs/schemi", n + ".json"), "utf8"))));
const problems = plan => F.check(F.normalize(F.exportPlan(plan)), reg, "native").map(F.problemLine);

// every template of the start screen gives a valid plan; the new ones carry their sample list and their starting settings
for (const t of F.TEMPLATES) { const p = F.templatePlan(reg, t); eq(problems(p), []); assert.ok(/^Good for: /.test(t.good), t.id); }
assert.ok(F.TEMPLATES.length >= 7);
// the first row: Tabui, the feature table, Attributio; the others sit under «More templates»
eq(F.TEMPLATES.filter(t => !t.more).map(t => t.id), ["tp", "table", "identify", "msn"]);
eq(F.TEMPLATES.filter(t => t.more).map(t => t.id), ["bentazone", "targeted", "tpseries", "standards", "blankfree"]);
{
  const by = id => F.templatePlan(reg, F.TEMPLATES.find(t => t.id === id));
  eq(["formula.ms1", "spectra.match", "annot.level"].every(x => F.findStep(by("identify"), x)), true);
  eq(["tp.generate", "tp.link"].every(x => F.findStep(by("tp"), x)), true);
  eq(F.hasAddons(by("table")), false);
  const series = by("tpseries");
  eq(["tp.generate", "tp.filter", "table.filter", "tp.network"].every(s => F.findStep(series, s)), true);
  assert.ok(series.inputs.some(f => f.type === "samples"));
  // the reliable-rows filter reads the table of the alignment, not the registry of the filters
  eq(F.findStep(series, "table.filter").in.table, "gapfill.table");
  // the MSn template has its own plan: no feature-table base, the graph feeds the two exports
  const msn = by("msn");
  eq(msn.steps.map(s => s.step.split("@")[0]), ["spectra.msn_graph", "msn.library_export", "msn.table_export"]);
  eq(F.findStep(msn, "index.mzidx"), undefined);
  eq(F.hasAddons(msn), true);
  assert.ok(F.TEMPLATES.find(t => t.id === "msn").help.includes("CID vs HCD"));
  // the bentazone model: the whole TP chain, the index wired into the hypotheses so the files decide polarity and resolution
  const ben = by("bentazone");
  eq(["tp.generate", "tp.link", "tp.filter", "tp.position", "tp.network"].every(s => F.findStep(ben, s)), true);
  eq(F.findStep(ben, "tp.generate").in.index, ["index.index"]);
  eq(F.findStep(by("tp"), "tp.generate").in.index, ["index.index"]);
  eq(F.findStep(ben, "tp.generate").params, undefined);
  // the network reads the rows of the link and its sample list, as the executor wants
  eq(F.findStep(ben, "tp.network").in.table, "link.tp");
  eq(F.findStep(ben, "tp.network").in.samples, "file:input1.samples");
  eq(problems(ben), []);
  eq(F.instanceOf(by("blankfree"), "filter").params, { blank_ratio: 3 });
  eq(F.instanceOf(by("standards"), "filter").params, { qc_cv_max: 30 });
}
console.log("ok");
