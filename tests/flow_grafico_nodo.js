// Runs the Flow checker (flow/lazy/flow-*.js) on the shared cases and prints {name, got} per case, as JSON; tests/test_flow_grafico.py compares with the file.
const fs = require("fs"), vm = require("vm"), path = require("path");
const root = path.resolve(__dirname, "..");
const ctx = vm.createContext({});
for (const f of ["registry", "check", "plan", "inputs", "compare"]) vm.runInContext(fs.readFileSync(path.join(root, "flow/lazy/flow-" + f + ".js"), "utf8"), ctx);
const F = ctx.FLOW;
const reg = F.registry(["steps-1", "steps-attributio-1", "steps-tabui-1"].map(n => JSON.parse(fs.readFileSync(path.join(root, "docs/schemi", n + ".json"), "utf8"))));
// the comparison cases are answered by the CLI registry (crate::flow::steps), which the page also loads: steps-cli-1 on top
const regCli = F.registry(["steps-1", "steps-attributio-1", "steps-tabui-1", "steps-cli-1"].map(n => JSON.parse(fs.readFileSync(path.join(root, "docs/schemi", n + ".json"), "utf8"))));
const cases = JSON.parse(fs.readFileSync(path.join(root, "crates/mzlab-feat/tests/data/flow_check_cases.json"), "utf8"));
const compareCases = JSON.parse(fs.readFileSync(path.join(root, "crates/mzlab-cli/tests/data/flow_compare_cases.json"), "utf8"));
const out = cases.map(c => {
  try { return { name: c.name, got: F.check(F.normalize(c.plan), reg, "native").map(F.problemLine) }; }
  catch (e) { return { name: c.name, parse_error: e.key || String(e) }; }
});
// the plans with meta.compare, as the page validates them (flow-page.js F.validate): the checker on the plan without the comparisons, then F.validateExtra
const withCompare = compareCases.map(c => {
  try { return { name: c.name, got: [...F.check(F.normalize(F.cleanRefs(F.withoutCompare(c.plan))), regCli, "native"), ...F.validateExtra(c.plan, regCli)].map(F.problemLine) }; }
  catch (e) { return { name: c.name, parse_error: e.key || String(e) }; }
});
// a saved plan at an older step version (table.filter@1): the checker refuses it as read; F.migrated asks the engine (the route answer replayed from the Rust test) and the plan then passes
const mig = JSON.parse(fs.readFileSync(path.join(root, "crates/mzlab-cli/tests/data/flow_migrate_case.json"), "utf8"));
(async () => {
  const asRead = F.importText(JSON.stringify(mig.old)), calls = [];
  F.lineCall = async (_S, line) => { calls.push(line); return mig.migrated; };
  const before = F.check(F.normalize(asRead.plan), reg, "native").map(F.problemLine);
  const done = await F.migrated(asRead, reg);
  const after = F.check(F.normalize(done.plan), reg, "native").map(F.problemLine);
  process.stdout.write(JSON.stringify({ cases: out, compare: withCompare, migrate: { before, after, calls, step: done.plan.steps[0].step } }));
})();
